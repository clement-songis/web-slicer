//! Endpoints projets (`/api/projects/*`). Toute ressource est scopée par le
//! compte connecté (`CurrentUser`) : un projet d'autrui répond **404** (SC-008,
//! jamais 403). Handlers minces : validation DTO → repo → réponse.
//!
//! La scène et les presets actifs sont des documents JSON versionnés
//! (`schema_version`) ; la sauvegarde utilise un verrou optimiste (409 en cas
//! de conflit multi-onglets).

use axum::extract::{Multipart, Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use glam::{DMat4, DVec3, EulerRot};
use serde_json::{json, Map, Value};
use uuid::Uuid;

use std::collections::HashSet;

use crate::domain::{
    GcodeId, NewProject, Preset, PresetId, PresetKind, PresetOrigin, Project, ProjectId,
    StorageError, UserId,
};
use crate::http::dto::{
    CreateProjectRequest, GcodeWarning, ImportLoadMode, ImportQuery, ImportReport,
    PatchProjectRequest, ProjectResponse, RenameProjectRequest, SaveProjectRequest,
};
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;

/// Version du schéma des documents de scène produits par le backend.
const SCENE_SCHEMA_VERSION: i64 = 1;

/// Document de scène vide par défaut (aucun objet, un plateau).
fn default_scene() -> serde_json::Value {
    json!({ "schema_version": SCENE_SCHEMA_VERSION, "objects": [], "plates": [] })
}

fn parse_project_id(raw: &str) -> ApiResult<ProjectId> {
    // Un id mal formé ne peut appartenir à personne → 404 (SC-008).
    Uuid::parse_str(raw)
        .map(ProjectId)
        .map_err(|_| ApiError::not_found("Projet"))
}

/// Retire un suffixe d'unicité final « (n) » (n ≥ 2) : « cube (2) » → « cube ».
fn name_base(name: &str) -> &str {
    if let Some(open) = name.strip_suffix(')').and_then(|s| s.rfind(" (")) {
        let digits = &name[open + 2..name.len() - 1];
        if !digits.is_empty()
            && digits.bytes().all(|b| b.is_ascii_digit())
            && digits.parse::<u64>().is_ok_and(|n| n >= 2)
        {
            return &name[..open];
        }
    }
    name
}

/// Premier nom libre « base (n) », n ≥ 2, comme l'interface (`names.ts`).
fn next_free_name(name: &str, taken: &HashSet<String>) -> String {
    let base = name_base(name);
    (2u64..)
        .map(|n| format!("{base} ({n})"))
        .find(|candidate| !taken.contains(candidate))
        .unwrap_or_else(|| format!("{base} ({})", Uuid::new_v4().simple()))
}

async fn taken_names(state: &AppState, owner: UserId) -> ApiResult<HashSet<String>> {
    Ok(state
        .storage
        .projects()
        .list(owner)
        .await?
        .into_iter()
        .map(|p| p.name)
        .collect())
}

/// Crée un projet sous un nom **unique** pour le compte (lot 32) : nom demandé
/// s'il est libre, sinon « nom (2) », « nom (3) »… Nouvel essai si un autre
/// onglet prend le nom entre-temps ; jamais l'erreur SQL brute.
pub(crate) async fn create_unique_project(
    state: &AppState,
    owner: UserId,
    mut project: NewProject,
) -> ApiResult<Project> {
    for _ in 0..8 {
        match state
            .storage
            .projects()
            .create(owner, project.clone())
            .await
        {
            Ok(created) => return Ok(created),
            Err(StorageError::Conflict(_)) => {
                let taken = taken_names(state, owner).await?;
                project.name = next_free_name(&project.name, &taken);
            }
            Err(e) => return Err(e.into()),
        }
    }
    Err(ApiError::conflict(
        "impossible d'attribuer un nom de projet libre",
    ))
}

/// Renomme un projet sous un nom **unique** du compte (lot 42) : nom demandé
/// s'il est libre (ou déjà le sien), sinon « nom (2) », « nom (3) »… comme
/// [`create_unique_project`]. Pour les noms produits par le serveur (projet de
/// calibration) : jamais de 409.
pub(crate) async fn rename_unique_project(
    state: &AppState,
    owner: UserId,
    id: ProjectId,
    name: &str,
) -> ApiResult<Project> {
    let mut candidate = name.to_string();
    for _ in 0..8 {
        match state.storage.projects().rename(owner, id, &candidate).await {
            Ok(project) => return Ok(project),
            Err(StorageError::Conflict(_)) => {
                let taken = taken_names(state, owner).await?;
                candidate = next_free_name(name, &taken);
            }
            Err(e) => return Err(e.into()),
        }
    }
    Err(ApiError::conflict(
        "impossible d'attribuer un nom de projet libre",
    ))
}

/// Renomme un projet du compte ; nom pris par un **autre** projet du compte →
/// 409 `name_taken` avec `details.suggestion` (premier nom libre).
async fn rename_project(
    state: &AppState,
    owner: UserId,
    id: ProjectId,
    name: &str,
) -> ApiResult<Project> {
    match state.storage.projects().rename(owner, id, name).await {
        Ok(project) => Ok(project),
        Err(StorageError::Conflict(_)) => {
            let taken = taken_names(state, owner).await?;
            Err(ApiError::name_taken(name, &next_free_name(name, &taken)))
        }
        Err(e) => Err(e.into()),
    }
}

/// `GET /api/projects` — bibliothèque du compte connecté.
pub async fn list(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<ProjectResponse>>> {
    let projects = state.storage.projects().list(user.id).await?;
    // Une seule lecture des jobs du compte : dernier G-code réussi par projet.
    let jobs = state.storage.jobs().list(user.id).await?;
    let mut out = Vec::with_capacity(projects.len());
    for project in projects {
        let scene_thumb = has_scene_thumbnail(&state, user.id, project.id);
        let gcode_thumb = project.thumbnail_path.is_none()
            && !scene_thumb
            && latest_gcode_thumbnail_from(&state, user.id, project.id, &jobs)
                .await
                .is_some();
        let mut resp: ProjectResponse = project.into();
        resp.has_thumbnail |= gcode_thumb || scene_thumb;
        out.push(resp);
    }
    Ok(Json(out))
}

/// `POST /api/projects` — crée un projet (scène/presets par défaut si absents).
pub async fn create(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Json(body): Json<CreateProjectRequest>,
) -> ApiResult<(StatusCode, Json<ProjectResponse>)> {
    let name = body.name.trim();
    if name.is_empty() {
        return Err(ApiError::validation(
            "le nom du projet est requis",
            json!({ "field": "name" }),
        ));
    }
    let project = create_unique_project(
        &state,
        user.id,
        NewProject {
            name: name.to_string(),
            scene: body.scene.unwrap_or_else(default_scene),
            active_presets: super::overrides::normalize_active_presets(
                body.active_presets.unwrap_or_else(|| json!({})),
            )?,
            thumbnail_path: None,
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(project.into())))
}

/// `POST /api/projects/import` — crée un projet depuis un fichier importé.
///
/// Un 3MF projet est lu comme une archive OPC : chaque occurrence de `<build>`
/// devient un modèle indépendant, sa géométrie (composants externes compris)
/// alimente une conversion asynchrone, et les métadonnées Orca restaurent les
/// transformations, plateaux, réglages d'objet et configuration globale. Les
/// autres formats conservent le comportement d'import d'un modèle simple.
///
/// `?load=project|geometry` (Orca : « Ouvrir comme projet » / « Importer la
/// géométrie seule », `LoadType`) ; absent → préférence du compte
/// `project_load_behaviour` (`load_geometry_only` → géométrie, sinon projet ;
/// les choix « demander » sont posés par le client qui transmet alors `load`).
/// La réponse d'un 3MF porte `import` ([`ImportReport`]) : mode retenu,
/// avertissement « G-code modifié » (`PresetBundle::validate_presets`) et
/// présence de métadonnées.
pub async fn import(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Query(query): Query<ImportQuery>,
    multipart: Multipart,
) -> ApiResult<(StatusCode, Json<ProjectResponse>)> {
    // Validation avant toute création : aucun projet orphelin sur fichier
    // invalide. L'extraction ZIP/XML est CPU/IO bornée et hors worker async.
    let (filename, format, bytes) =
        super::models::read_model_upload(&state, user.id, multipart).await?;
    let name = project_name_from(&filename);

    if format != crate::domain::ModelFormat::ThreeMf {
        let project = create_unique_project(
            &state,
            user.id,
            NewProject {
                name,
                scene: default_scene(),
                active_presets: json!({}),
                thumbnail_path: None,
            },
        )
        .await?;
        let model = super::models::write_model_record(
            &state, user.id, project.id, format, filename, &bytes,
        )
        .await?;
        super::models::spawn_conversion(&state, model);
        return Ok((StatusCode::CREATED, Json(project.into())));
    }

    let mut imported = crate::models::threemf::read_project(&bytes)
        .await
        .map_err(|e| ApiError::validation(e.to_string(), details(&filename)))?;

    let preferences = state.storage.users().get_preferences(user.id).await?;
    let load = query
        .load
        .unwrap_or_else(|| preferred_load_mode(&preferences));
    // Plater::priv::load_files : validate_presets seulement en ouvrant comme
    // projet une config non vide, sauf « ne plus avertir ».
    let gcode_warning = if load == ImportLoadMode::Project
        && !imported.config.is_empty()
        && !no_warn_when_modified_gcodes(&preferences)
    {
        let (machines, filaments) = trusted_preset_names(&state, user.id).await?;
        validate_presets(
            &imported.config,
            |name| machines.contains(name),
            |name| filaments.contains(name),
        )
    } else {
        None
    };
    let metadata = match load {
        ImportLoadMode::Project => {
            // Positions monde d'Orca → repère du plateau de chaque objet
            // (lot 43) : plateau de l'imprimante du projet.
            if let Some((width, depth)) = bed_rect(&imported.config) {
                to_plate_local(&mut imported, width, depth);
            }
            plates_follow_project_bed_type(&mut imported);
            imported.metadata.take()
        }
        ImportLoadMode::Geometry => {
            geometry_only(&mut imported);
            None
        }
    };

    let project = create_unique_project(
        &state,
        user.id,
        NewProject {
            name: name.clone(),
            scene: default_scene(),
            active_presets: json!({}),
            thumbnail_path: None,
        },
    )
    .await?;

    let result = import_3mf_contents(&state, user.id, project.id, &name, &filename, imported).await;
    let (scene, active_presets, models) = match result {
        Ok(value) => value,
        Err(error) => {
            // Les FK suppriment les lignes de modèles ; les fichiers déjà
            // matérialisés sont retirés au mieux par le nettoyage ci-dessous.
            let _ = state.storage.projects().delete(user.id, project.id).await;
            return Err(error);
        }
    };
    let project = state
        .storage
        .projects()
        .update(
            user.id,
            project.id,
            project.version,
            scene,
            active_presets,
            None,
        )
        .await?;

    // Fichiers auxiliaires du 3MF (Auxiliaries/), extraits par libslic3r comme à
    // l'ouverture d'un projet dans Orca (LoadStrategy::LoadAuxiliary : pas en
    // géométrie seule). Un échec n'empêche pas l'import.
    if load == ImportLoadMode::Project {
        let aux_dir = state.files.auxiliaries_dir(user.id, project.id);
        if let Err(error) =
            crate::engine::project_3mf::read_project_auxiliaries(&bytes, &aux_dir).await
        {
            tracing::warn!(%error, "fichiers auxiliaires du 3MF ignorés");
        }
    }
    // Métadonnées (Model::model_info / design_info) : copiées seulement à
    // l'ouverture comme projet (`if (load_model && !load_config) ;`).
    // libslic3r crée toujours un ModelInfo à la lecture : « présentes » =
    // au moins un champ affiché par l'onglet Projet d'Orca.
    let has_metadata = match &metadata {
        Some(metadata) => {
            super::metadata::store(&state, user.id, project.id, metadata).await?;
            super::metadata::is_meaningful(metadata)
        }
        None => false,
    };

    for (model, mesh) in models {
        let converter = state.converter.clone();
        tokio::spawn(async move {
            converter.convert_mesh(model, mesh).await;
        });
    }
    let mut response: ProjectResponse = project.into();
    response.import = Some(ImportReport {
        load,
        gcode_warning,
        has_metadata,
    });
    Ok((StatusCode::CREATED, Json(response)))
}

/// Préférence `project_load_behaviour` d'Orca (AppConfig.hpp,
/// SETTING_PROJECT_LOAD_BEHAVIOUR) dans le document de préférences du compte
/// (clé Orca ou camelCase du client).
pub(crate) fn preferred_load_mode(preferences: &Value) -> ImportLoadMode {
    let setting = ["project_load_behaviour", "projectLoadBehaviour"]
        .iter()
        .find_map(|key| preferences.get(*key).and_then(Value::as_str));
    match setting {
        Some("load_geometry_only") => ImportLoadMode::Geometry,
        _ => ImportLoadMode::Project,
    }
}

/// Préférence `no_warn_when_modified_gcodes` (case « Ne plus afficher » de la
/// boîte « Modified G-code »).
pub(crate) fn no_warn_when_modified_gcodes(preferences: &Value) -> bool {
    ["no_warn_when_modified_gcodes", "noWarnWhenModifiedGcodes"]
        .iter()
        .filter_map(|key| preferences.get(*key))
        .any(|value| value.as_bool() == Some(true) || value.as_str() == Some("true"))
}

/// Noms des presets « de confiance » d'Orca (`PresetCollection::validate_preset` :
/// système, défaut ou bundle) : presets système machine et filament, **et
/// leurs anciens noms** (`renamed_from` → `find_preset_renamed`, lot 35) —
/// un 3MF référençant un preset sous un nom renommé reste de confiance,
/// pour le preset lui-même comme pour son `inherits_group`.
async fn trusted_preset_names(
    state: &AppState,
    user_id: UserId,
) -> ApiResult<(HashSet<String>, HashSet<String>)> {
    let mut out = (HashSet::new(), HashSet::new());
    for (kind, set) in [
        (PresetKind::Machine, &mut out.0),
        (PresetKind::Filament, &mut out.1),
    ] {
        for preset in state.storage.presets().list_by_kind(kind, user_id).await? {
            if preset.origin == PresetOrigin::System {
                set.insert(preset.name);
                for alias in preset.renamed_from {
                    set.insert(alias);
                }
            }
        }
    }
    Ok(out)
}

/// Clés G-code surveillées par `PresetBundle::validate_presets`
/// (PresetBundle.cpp, `gcodes_key_set`).
pub(crate) const VALIDATED_GCODE_KEYS: &[&str] = &[
    "filament_end_gcode",
    "filament_start_gcode",
    "change_filament_gcode",
    "layer_change_gcode",
    "machine_end_gcode",
    "machine_pause_gcode",
    "machine_start_gcode",
    "template_custom_gcode",
    "printing_by_object_gcode",
    "before_layer_change_gcode",
    "time_lapse_gcode",
    "wrapping_detection_gcode",
];

/// `PresetBundle::validate_presets` d'Orca 2.4.1 sur la config d'un 3MF :
/// 1. imprimante `printer_settings_id` (ou son `inherits_group` final) non
///    système → `customized_presets` (VALIDATE_PRESETS_PRINTER_NOT_FOUND) ;
/// 2. filament `filament_settings_id[i]` non vide, ni lui ni
///    `inherits_group[i + 1]` système → `customized_presets` ;
/// 3. sinon clés G-code de `different_settings_to_system[1 .. n - 1]` (boucle
///    d'Orca `for index = 1; index < filament_count`, qui n'examine ni le
///    dernier filament ni l'imprimante) → `modified_gcodes`.
///
/// Éléments triés et dédupliqués (`std::set`).
pub(crate) fn validate_presets(
    config: &Map<String, Value>,
    printer_trusted: impl Fn(&str) -> bool,
    filament_trusted: impl Fn(&str) -> bool,
) -> Option<GcodeWarning> {
    let filament_count = raw_items(config.get("filament_diameter")).len();
    let mut inherits = raw_items(config.get("inherits_group"));
    let mut different = raw_items(config.get("different_settings_to_system"));
    let mut names = raw_items(config.get("filament_settings_id"));
    inherits.resize(filament_count + 2, String::new());
    different.resize(filament_count + 2, String::new());
    names.resize(filament_count, String::new());
    let printer = raw_text(config.get("printer_settings_id")).unwrap_or_default();
    let trusted = |name: &str, parent: &str, check: &dyn Fn(&str) -> bool| {
        check(name) || (!parent.is_empty() && check(parent))
    };

    let mut items = std::collections::BTreeSet::new();
    let mut customized = false;
    if !trusted(&printer, &inherits[filament_count + 1], &printer_trusted) {
        items.insert(printer);
        customized = true;
    }
    for (index, name) in names.iter().enumerate() {
        if name.is_empty() {
            continue;
        }
        if !trusted(name, &inherits[index + 1], &filament_trusted) {
            items.insert(name.clone());
            customized = true;
        }
    }
    if customized {
        return Some(GcodeWarning {
            kind: "customized_presets".into(),
            items: items.into_iter().collect(),
        });
    }
    for keys in different.iter().take(filament_count).skip(1) {
        for key in keys.split(';').map(str::trim) {
            if VALIDATED_GCODE_KEYS.contains(&key) {
                items.insert(key.to_string());
            }
        }
    }
    (!items.is_empty()).then(|| GcodeWarning {
        kind: "modified_gcodes".into(),
        items: items.into_iter().collect(),
    })
}

/// « Importer la géométrie seule » (`LoadStrategy::LoadModel` sans
/// `LoadConfig`) : ni configuration ni presets du projet, plateaux sans nom
/// ni réglages, sans métadonnées. Les réglages d'objet et de volume sont
/// conservés : la remise à zéro (`else if (!load_config)`) suit la chaîne des
/// `En3mfType` dont toutes les valeurs sont déjà traitées, elle est
/// inatteignable dans Orca 2.4.1.
///
/// Lot 35, deux écarts du lot 34 comblés comme Orca :
/// - **Réaffectation des plateaux par position** (`partplate_list.
///   reload_all_objects`, Plater.cpp `else { reload_all_objects(); }`) : le
///   contenu `plate_data` du 3MF est ignoré, chaque instance rejoint le
///   premier plateau dont le rectangle la croise — grille d'Orca `col ×
///   stride` avec `stride = largeur × (1 + LOGICAL_PART_PLATE_GAP)`,`
///   `LOGICAL_PART_PLATE_GAP = 1/5` (PartPlate.cpp). Les objets hors de
///   toute cellule gardent leur plateau déclaré (Orca les met sur le plateau
///   non imprimable, non représentable dans la scène web).
/// - **Filaments manquants ajoutés** (Plater.cpp « add extruder … if only
///   import geometry ») : les extrudeurs utilisés par les objets/volumes
///   déterminent le nombre de filaments, jusqu'à
///   `MAXIMUM_EXTRUDER_NUMBER` (16), avec la palette d'Orca
///   (`Plater::get_next_color_for_filament`, depuis l'index 0 pour rester
///   déterministe).
pub(crate) fn geometry_only(imported: &mut crate::models::threemf::ImportedProject) {
    let bed = bed_rect(&imported.config);
    imported.config.clear();
    imported.metadata = None;
    for plate in &mut imported.plates {
        plate.name.clear();
        plate.settings.clear();
        plate.plate_type = crate::models::threemf::DEFAULT_PLATE_TYPE.to_string();
    }
    // Filaments manquants : à calculer avant d'oublier la géométrie (les
    // extrudeurs vivent sur les objets, pas sur la config).
    imported.filament_colours = missing_filament_colours(&imported.objects);
    if let Some((width, depth)) = bed {
        reassign_plates_by_position(imported, width, depth);
        to_plate_local(imported, width, depth);
    }
}

/// Lot 43 (audit 42 I3) : `curr_bed_type` est une option **projet** d'Orca
/// (project_settings.config), pas une option de preset : un plateau sans type
/// propre (PartPlate « btDefault ») suit celui du projet. Sans cette reprise,
/// un projet ouvert avec son process nommé tranchait sur le type par défaut
/// du registre (Cool Plate : plateau à 35 °C au lieu de 55 °C sur arbre.3mf).
/// Le type du projet devient le réglage explicite des plateaux qui n'en ont pas.
pub(crate) fn plates_follow_project_bed_type(
    imported: &mut crate::models::threemf::ImportedProject,
) {
    let Some(bed) = raw_text(imported.config.get("curr_bed_type")).filter(|b| !b.is_empty()) else {
        return;
    };
    for plate in &mut imported.plates {
        if !plate.settings.contains_key("curr_bed_type") {
            plate
                .settings
                .insert("curr_bed_type".into(), Value::String(bed.clone()));
            plate.plate_type = crate::models::threemf::bed_type_to_plate_type(&bed);
        }
    }
}

/// Lot 43 (audit 42 I1) : un 3MF d'Orca écrit les positions dans le repère
/// monde de la GUI, où le plateau k est décalé de
/// PartPlateList::compute_shape_position (grille compute_colum_count, pas =
/// largeur × 1,2 en X et −profondeur × 1,2 en Y). La scène web place chaque
/// objet dans le repère de **son** plateau (plate-layout.ts : le plateau
/// actif est à l'origine) et le moteur tranche ces coordonnées telles
/// quelles : sans cette conversion, un objet du plateau 2 était tranché à
/// X + 420 (G-code hors course, audit 42). Les objets non déclarés restent
/// sur le plateau 1 (origine nulle). L'export ajoute à nouveau l'origine
/// (export::to_orca_world).
pub(crate) fn to_plate_local(
    imported: &mut crate::models::threemf::ImportedProject,
    width: f64,
    depth: f64,
) {
    let count = imported.plates.len();
    if count < 2 {
        return;
    }
    let plate_of = |source: &str, instance: usize| {
        imported.plates.iter().position(|plate| {
            plate
                .instances
                .iter()
                .any(|(id, index)| id == source && *index == instance)
        })
    };
    let shifts: Vec<[f64; 2]> = imported
        .objects
        .iter()
        .map(
            |object| match plate_of(&object.source_id, object.instance_index) {
                Some(index) => crate::build_volume::plate_origin(index, count, width, depth),
                None => [0.0, 0.0],
            },
        )
        .collect();
    for (object, [dx, dy]) in imported.objects.iter_mut().zip(shifts) {
        if dx != 0.0 || dy != 0.0 {
            object.matrix = DMat4::from_translation(DVec3::new(-dx, -dy, 0.0)) * object.matrix;
        }
    }
}

/// Rectangle du plateau (largeur, profondeur) depuis `printable_area` de la
/// config Orca du 3MF (polygone `x,y;x,y;…` ou tableau). Orca utilise le
/// plateau de l'imprimante courante ; au moment de l'import le projet n'a
/// pas encore de preset — la zone du fichier est la meilleure estimation.
fn bed_rect(config: &Map<String, Value>) -> Option<(f64, f64)> {
    let points: Vec<(f64, f64)> = match config.get("printable_area") {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|v| v.as_str())
            .filter_map(parse_point)
            .collect(),
        Some(Value::String(s)) => s.split([',', ';']).filter_map(parse_point).collect(),
        _ => Vec::new(),
    };
    let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
    let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for (x, y) in points {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    if !max_x.is_finite() || !max_y.is_finite() || max_x <= min_x || max_y <= min_y {
        return None;
    }
    Some((max_x - min_x, max_y - min_y))
}

fn parse_point(part: &str) -> Option<(f64, f64)> {
    let part = part.trim();
    // Format Orca « 200x0 » (et tolérance « 200,0 »).
    let (x, y) = part.split_once('x').or_else(|| part.split_once(','))?;
    Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
}

/// `PartPlateList::reload_all_objects` : chaque instance rejoint le premier
/// plateau dont le rectangle `origine(k) + [0, largeur] × [0, profondeur]`
/// croise sa boîte englobante monde ; origine = grille d'Orca
/// (`compute_shape_position`, lot 43 : rangées comprises au-delà de 2
/// plateaux, pas = largeur × 1,2, `LOGICAL_PART_PLATE_GAP = 1/5`).
fn reassign_plates_by_position(
    imported: &mut crate::models::threemf::ImportedProject,
    width: f64,
    depth: f64,
) {
    let count = imported.plates.len();
    let mut members: Vec<Vec<(String, usize)>> = vec![Vec::new(); count];
    for object in &imported.objects {
        let bbox = world_bbox_xy(object);
        let target = (0..count).find(|&k| {
            let [x0, y0] = crate::build_volume::plate_origin(k, count, width, depth);
            bbox.is_none_or(|(min_x, max_x, min_y, max_y)| {
                min_x < x0 + width && max_x > x0 && max_y > y0 && min_y < y0 + depth
            })
        });
        let key = (object.source_id.clone(), object.instance_index);
        match target {
            Some(k) => members[k].push(key),
            // Hors grille : garde son plateau déclaré (voir doc `geometry_only`).
            None => {
                if let Some(k) = imported
                    .plates
                    .iter()
                    .position(|p| p.instances.iter().any(|i| i == &key))
                {
                    members[k].push(key);
                } else {
                    members[0].push(key);
                }
            }
        }
    }
    for (plate, instances) in imported.plates.iter_mut().zip(members) {
        plate.instances = instances;
    }
}

/// Boîte englobante monde XY d'un objet importé (8 coins du bbox local
/// transformés par la matrice `<build>`), `(min_x, max_x, min_y, max_y)`.
fn world_bbox_xy(object: &crate::models::threemf::ImportedObject) -> Option<(f64, f64, f64, f64)> {
    let (min, max) = object.mesh.bounding_box()?;
    let (mut x0, mut x1, mut y0, mut y1) = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    for a in [min[0] as f64, max[0] as f64] {
        for b in [min[1] as f64, max[1] as f64] {
            for c in [min[2] as f64, max[2] as f64] {
                let p = object.matrix.transform_point3(glam::DVec3::new(a, b, c));
                x0 = x0.min(p.x);
                x1 = x1.max(p.x);
                y0 = y0.min(p.y);
                y1 = y1.max(p.y);
            }
        }
    }
    Some((x0, x1, y0, y1))
}

/// Palette « couleurs système filament » d'Orca (`FILAMENT_SYSTEM_COLORS_NUM`
/// = 16, `Plater::get_next_color_for_filament`).
const FILAMENT_SYSTEM_COLORS: [&str; 16] = [
    "#00C1AE", "#F4E2C1", "#ED1C24", "#00FF7F", "#F26722", "#FFEB31", "#7841CE", "#115877",
    "#ED1E79", "#2EBDEF", "#345B2F", "#800080", "#FA8173", "#800000", "#F7B763", "#A4C41E",
];

/// `MAXIMUM_EXTRUDER_NUMBER` d'Orca.
const MAXIMUM_EXTRUDER_NUMBER: usize = 16;

/// Filaments manquants (Plater.cpp, « add extruder … if only import
/// geometry ») : le plus grand extrudeur utilisé par les objets, volumes et
/// pièces détermine le nombre de filaments ; couleurs de la palette système
/// d'Orca depuis l'index 0 (le compteur global de la GUI n'est pas
/// reproductible serveur-side). Vide si un seul extrudeur (le défaut).
fn missing_filament_colours(objects: &[crate::models::threemf::ImportedObject]) -> Vec<String> {
    let mut max_extruder = 0usize;
    let mut note = |value: u64| {
        if value > 1 {
            max_extruder = max_extruder.max(value.min(MAXIMUM_EXTRUDER_NUMBER as u64) as usize);
        }
    };
    for object in objects {
        note(object.extruder);
        for part in &object.parts {
            note(part.extruder);
        }
        for volume in &object.volumes {
            if let Some(extruder) = volume.get("extruder").and_then(Value::as_u64) {
                note(extruder);
            }
        }
        for range in &object.layer_ranges {
            if let Some(extruder) = range
                .get("settings")
                .and_then(|s| s.get("extruder"))
                .and_then(Value::as_u64)
            {
                note(extruder);
            }
        }
    }
    (0..max_extruder.saturating_sub(1))
        .map(|index| FILAMENT_SYSTEM_COLORS[index % FILAMENT_SYSTEM_COLORS.len()].to_string())
        .collect()
}

/// `POST /api/projects/{id}/import-3mf` — ajoute le contenu d'un 3MF à un projet
/// existant comme Orca (Plater::load_files) : un objet nommé par objet du 3MF,
/// avec transformations, volumes, réglages par objet et peinture. La config
/// embarquée n'est pas appliquée (Orca la propose séparément). Réponse :
/// documents d'objets de scène (`objects`) + modèles créés (`models`).
pub async fn import_into(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(raw_id): Path<String>,
    multipart: Multipart,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let project = state
        .storage
        .projects()
        .get(user.id, parse_project_id(&raw_id)?)
        .await?;
    let (filename, format, bytes) =
        super::models::read_model_upload(&state, user.id, multipart).await?;
    if format != crate::domain::ModelFormat::ThreeMf {
        return Err(ApiError::validation(
            "fichier 3MF attendu",
            details(&filename),
        ));
    }
    let mut imported = crate::models::threemf::read_project(&bytes)
        .await
        .map_err(|e| ApiError::validation(e.to_string(), details(&filename)))?;
    imported.config.clear();
    let name = project_name_from(&filename);
    let (scene, _, models) =
        import_3mf_contents(&state, user.id, project.id, &name, &filename, imported).await?;
    let responses: Vec<crate::http::dto::ModelResponse> =
        models.iter().map(|(m, _)| m.clone().into()).collect();
    for (model, mesh) in models {
        let converter = state.converter.clone();
        tokio::spawn(async move {
            converter.convert_mesh(model, mesh).await;
        });
    }
    Ok((
        StatusCode::CREATED,
        Json(json!({ "objects": scene["objects"], "models": responses })),
    ))
}

/// Matérialise les objets extraits et produit les documents projet. Les meshes
/// restent en mémoire jusqu'aux conversions lancées après la sauvegarde.
/// Archive 3MF autonome d'un objet/pièce importé (géométrie locale, instance
/// identité), écrite par libslic3r (`store_bbs_3mf`) : une conversion ultérieure
/// (notamment après duplication) ne recharge jamais toute l'archive d'origine.
/// 3MF autonome d'un objet importé. `stats` : erreurs du maillage lues dans le
/// projet (`<mesh_stat>`), réécrites telles quelles pour que la liste d'objets
/// (`GET …/mesh-errors`) affiche, comme Orca, les erreurs réparées d'origine.
async fn source_3mf(
    name: &str,
    mesh: &engine::api::TriangleMesh,
    stats: Option<&engine::api::MeshErrorStats>,
) -> ApiResult<Vec<u8>> {
    let mut model = engine::api::Model::default();
    model.add_object(name.to_string(), mesh.clone()).volumes[0].mesh_stats = stats.cloned();
    crate::engine::project_3mf::write_project_3mf(
        &model,
        &engine::api::DynamicPrintConfig::new(),
        &[],
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "création du 3MF autonome");
        ApiError::internal()
    })
}

pub(crate) async fn import_3mf_contents(
    state: &AppState,
    user_id: UserId,
    project_id: ProjectId,
    project_name: &str,
    filename: &str,
    imported: crate::models::threemf::ImportedProject,
) -> ApiResult<(
    Value,
    Value,
    Vec<(crate::domain::Model, engine::api::TriangleMesh)>,
)> {
    let mut active_presets = json!({});
    // Réglages du projet Orca (project_settings.config) → scene.projectConfig :
    // couleurs des filaments, matrice et multiplicateur de purge. En « géométrie
    // seule », la palette des filaments manquants remplace celle de la config
    // (vide) pour que les extrudeurs des objets restent valides (lot 35).
    let mut project_config = imported_project_config(&imported.config);
    if project_config
        .get("filament_colour")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
        && !imported.filament_colours.is_empty()
    {
        project_config.insert(
            "filament_colour".into(),
            Value::Array(
                imported
                    .filament_colours
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            ),
        );
    }
    if !imported.config.is_empty() {
        // Presets filament / imprimante du projet (lot 25, audit J2-a/J2-h),
        // avant le process qui consomme la configuration.
        let (printer, filaments) =
            import_project_presets(state, user_id, project_name, &imported.config).await?;
        // Lot 40 : comme Orca (load_external_preset, LoadAndSelect::Always),
        // imprimante posée + process nommé connu → ce preset est sélectionné
        // et les valeurs du projet qui en diffèrent deviennent des surcharges
        // (preset « modifié »). Sinon (imprimante non posée, process inconnu),
        // la config aplatie reste portée par un preset importé.
        let named_process = match printer {
            Some(_) => named_process_preset(state, user_id, &imported.config).await?,
            None => None,
        };
        let process = match named_process {
            Some(process) => process,
            None => {
                create_imported_preset(state, user_id, project_name, imported.config.clone())
                    .await?
                    .id
            }
        };
        active_presets = json!({ "process": process.to_string() });
        if let Some(printer) = printer {
            active_presets["printer"] = json!(printer.to_string());
        }
        if !filaments.is_empty() {
            active_presets["filaments"] = json!(filaments
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>());
        }
        if named_process.is_some() {
            let (resolved, _) =
                super::slice::resolve_active_config(state, user_id, &active_presets).await?;
            let project = engine::presets::resolve_preset_chain(&[engine::api::RawPreset {
                name: "3MF".into(),
                inherits: None,
                values: imported.config.clone(),
            }])
            .map_err(|e| {
                tracing::warn!(error = %e, "configuration 3MF non résolue");
                ApiError::validation("configuration 3MF illisible", json!({}))
            })?;
            // Clés déclarées modifiées par Orca : [process, filaments…, imprimante].
            let different = raw_items(imported.config.get("different_settings_to_system"));
            let listed = |entry: Option<&String>| -> Vec<String> {
                entry
                    .map(|keys| {
                        keys.split(';')
                            .map(|k| k.trim().trim_matches('"').to_string())
                            .filter(|k| !k.is_empty())
                            .collect()
                    })
                    .unwrap_or_default()
            };
            let process_listed = listed(different.first());
            let printer_listed = if different.len() >= 2 {
                listed(different.last())
            } else {
                Vec::new()
            };
            if let Some(overrides) = super::overrides::imported_overrides(
                &project,
                &resolved,
                printer.is_some(),
                &process_listed,
                &printer_listed,
            ) {
                active_presets[super::overrides::OVERRIDES_KEY] =
                    serde_json::to_value(overrides).map_err(|_| ApiError::internal())?;
            }
        }
        // Lot 43 (I3) : valeurs filament du projet ≠ preset filament retenu →
        // surcharges du filament (preset « (modifié) »), comme
        // load_external_preset d'Orca, quel que soit le preset process.
        let filament_overrides = imported_filament_overrides(
            state,
            user_id,
            &imported.config,
            &active_presets,
            &filaments,
        )
        .await?;
        if filament_overrides.iter().any(Option::is_some) {
            let mut draft = super::overrides::overrides_of(&active_presets)?.unwrap_or_default();
            draft.filaments = Some(filament_overrides);
            if let Some(draft) = super::overrides::compact(draft) {
                active_presets[super::overrides::OVERRIDES_KEY] =
                    serde_json::to_value(draft).map_err(|_| ApiError::internal())?;
            }
        }
    }

    let mut models: Vec<(crate::domain::Model, engine::api::TriangleMesh)> =
        Vec::with_capacity(imported.objects.len());
    let mut scene_objects = Vec::with_capacity(imported.objects.len());
    let mut object_ids = Vec::with_capacity(imported.objects.len());
    for object in &imported.objects {
        let source_bytes =
            source_3mf(&object.name, &object.mesh, object.mesh_stats.as_ref()).await?;
        let model_name = if imported.objects.len() == 1 {
            filename.to_string()
        } else {
            format!("{} — {}.3mf", project_name, object.name)
        };
        let model = match super::models::write_model_record(
            state,
            user_id,
            project_id,
            crate::domain::ModelFormat::ThreeMf,
            model_name,
            &source_bytes,
        )
        .await
        {
            Ok(model) => model,
            Err(error) => {
                for (created, _) in &models {
                    let _ = tokio::fs::remove_file(&created.file_path).await;
                }
                return Err(error);
            }
        };
        models.push((model.clone(), object.mesh.clone()));
        // Pièces supplémentaires : un modèle chacune, référencé par un volume
        // maillé de la scène (`shape: "mesh"`).
        let mut mesh_volumes = Vec::with_capacity(object.parts.len());
        for part in &object.parts {
            let part_source = source_3mf(&part.name, &part.mesh, part.mesh_stats.as_ref()).await?;
            let part_model = match super::models::write_model_record(
                state,
                user_id,
                project_id,
                crate::domain::ModelFormat::ThreeMf,
                format!("{} — {}.3mf", object.name, part.name),
                &part_source,
            )
            .await
            {
                Ok(model) => model,
                Err(error) => {
                    for (created, _) in &models {
                        let _ = tokio::fs::remove_file(&created.file_path).await;
                    }
                    return Err(error);
                }
            };
            let size: Vec<f64> = part
                .mesh
                .bounding_box()
                .map(|(min, max)| (0..3).map(|a| (max[a] - min[a]) as f64).collect())
                .unwrap_or_else(|| vec![0.0; 3]);
            let mut volume = json!({
                "id": Uuid::new_v4().to_string(),
                "type": part.role,
                "name": part.name,
                "shape": "mesh",
                "modelId": part_model.id.to_string(),
                "size": size,
                "offset": part.offset,
                "settings": part.settings,
            });
            if part.extruder > 0 {
                volume["extruder"] = part.extruder.into();
            }
            if let Some(emboss) = &part.emboss {
                volume["emboss"] = super::mesh_ops::emboss_with_font_id(emboss.clone());
            }
            if let Some(info) = &part.cut_info {
                volume["cutInfo"] = crate::scene::cut_info_to_scene(info);
            }
            mesh_volumes.push(volume);
            models.push((part_model, part.mesh.clone()));
        }
        let object_id = Uuid::new_v4().to_string();
        let mut scene_object = imported_scene_object(&object_id, &model.id.to_string(), object);
        // Orca crée un objet par `<object>` du 3MF, avec son nom
        // (bbs_3mf.cpp, metadata « name ») ; un objet unique garde le nom du fichier.
        if imported.objects.len() > 1 && !object.name.is_empty() {
            scene_object["name"] = object.name.clone().into();
        }
        if let Some(volumes) = scene_object["volumes"].as_array_mut() {
            volumes.extend(mesh_volumes);
        }
        scene_objects.push(scene_object);
        object_ids.push(object_id);
    }

    let index_by_instance: std::collections::HashMap<(&str, usize), usize> = imported
        .objects
        .iter()
        .enumerate()
        .map(|(index, object)| ((object.source_id.as_str(), object.instance_index), index))
        .collect();
    let mut plates = Vec::with_capacity(imported.plates.len());
    for (index, plate) in imported.plates.iter().enumerate() {
        let mut ids: Vec<String> = plate
            .instances
            .iter()
            .filter_map(|instance| index_by_instance.get(&(instance.0.as_str(), instance.1)))
            .map(|object_index| object_ids[*object_index].clone())
            .collect();
        if index == 0 {
            // Certains fichiers ne déclarent pas explicitement toutes les
            // instances : elles restent visibles sur le premier plateau.
            for (object_index, id) in object_ids.iter().enumerate() {
                if !imported.plates.iter().any(|plate| {
                    plate.instances.iter().any(|instance| {
                        index_by_instance.get(&(instance.0.as_str(), instance.1))
                            == Some(&object_index)
                    })
                }) {
                    ids.push(id.clone());
                }
            }
        }
        let mut document = json!({
            "id": Uuid::new_v4().to_string(),
            "name": plate.name,
            "plateType": plate.plate_type,
            "objectIds": ids,
        });
        // Réglages du plateau (PLATE_SETTING_KEYS : type de plaque, séquence…),
        // appliqués au tranchage de ce plateau (slice::with_plate_settings).
        // Lot 43 : ils étaient lus puis perdus à l'ouverture.
        if !plate.settings.is_empty() {
            document["settings"] = Value::Object(plate.settings.clone());
        }
        plates.push(document);
    }
    let active_id = plates.first().and_then(|plate| plate["id"].as_str());
    let mut scene = json!({
        "schema_version": SCENE_SCHEMA_VERSION,
        "schemaVersion": SCENE_SCHEMA_VERSION,
        "objects": scene_objects,
        "plates": { "plates": plates, "activeId": active_id },
    });
    if !project_config.is_empty() {
        scene["projectConfig"] = Value::Object(project_config);
    }
    Ok((scene, active_presets, models))
}

/// `scene.projectConfig` depuis la config Orca d'un 3MF (chaînes ou tableaux
/// de chaînes) : `filament_colour` (tableau), `flush_volumes_matrix` (nombres),
/// `flush_multiplier` (nombre, première buse).
fn imported_project_config(config: &Map<String, Value>) -> Map<String, Value> {
    let strings = |v: &Value| -> Vec<String> {
        match v {
            Value::Array(items) => items
                .iter()
                .filter_map(|i| i.as_str().map(str::to_string))
                .collect(),
            Value::String(s) => s
                .split([';', ','])
                .map(|p| p.trim().trim_matches('"').to_string())
                .collect(),
            _ => Vec::new(),
        }
    };
    let mut out = Map::new();
    if let Some(colours) = config
        .get("filament_colour")
        .map(strings)
        .filter(|c| !c.is_empty())
    {
        out.insert("filament_colour".into(), json!(colours));
    }
    if let Some(matrix) = config.get("flush_volumes_matrix").map(strings) {
        let values: Option<Vec<f64>> = matrix.iter().map(|s| s.parse().ok()).collect();
        if let Some(values) = values.filter(|v| !v.is_empty()) {
            out.insert("flush_volumes_matrix".into(), json!(values));
        }
    }
    if let Some(first) = config
        .get("flush_multiplier")
        .map(strings)
        .and_then(|v| v.first().and_then(|s| s.parse::<f64>().ok()))
    {
        out.insert("flush_multiplier".into(), json!(first));
    }
    out
}

fn imported_scene_object(
    object_id: &str,
    model_id: &str,
    object: &crate::models::threemf::ImportedObject,
) -> Value {
    // Le client et le runner recentrent les meshes ayant une transformation.
    // Compense ce recentrage pour que `T × (v - centre)` retrouve exactement
    // les coordonnées monde du 3MF d'origine.
    let center = object
        .mesh
        .bounding_box()
        .map(|(min, max)| {
            DVec3::new(
                (min[0] + max[0]) as f64 / 2.0,
                (min[1] + max[1]) as f64 / 2.0,
                (min[2] + max[2]) as f64 / 2.0,
            )
        })
        .unwrap_or(DVec3::ZERO);
    let matrix = object.matrix * DMat4::from_translation(center);
    let (scale, rotation, translation) = matrix.to_scale_rotation_translation();
    let (rx, ry, rz) = rotation.to_euler(EulerRot::XYZ);
    let mut document = json!({
        "id": object_id,
        "modelId": model_id,
        "transform": {
            "position": [translation.x, translation.y, translation.z],
            "rotation": [rx.to_degrees(), ry.to_degrees(), rz.to_degrees()],
            "scale": [scale.x, scale.y, scale.z],
        },
        "extruder": object.extruder,
        "printable": object.printable,
        "settings": object.settings,
        "layerProfile": object.layer_profile,
        "layerRanges": object.layer_ranges,
        "painting": object.painting,
        "brimEars": { "points": [] },
        "volumes": object.volumes,
    });
    if let Some(mesh_name) = &object.mesh_name {
        document["meshName"] = mesh_name.clone().into();
    }
    if let Some(emboss) = &object.emboss {
        document["emboss"] = super::mesh_ops::emboss_with_font_id(emboss.clone());
    }
    if let Some(cut) = &object.cut_id {
        document["cutId"] = crate::scene::cut_id_to_scene(cut);
    }
    document
}

/// `Preset::filament_options()` d'Orca 2.4.1 (libslic3r/Preset.cpp,
/// `s_Preset_filament_options`, 127 clés, `filament_colour` commenté
/// dans Orca : option projet). Sert à répartir la configuration aplatie d'un 3MF
/// en presets filament comme `PresetBundle::load_config_file_config`.
const ORCA_FILAMENT_OPTIONS: [&str; 127] = [
    "default_filament_colour",
    "required_nozzle_HRC",
    "filament_diameter",
    "pellet_flow_coefficient",
    "volumetric_speed_coefficients",
    "filament_type",
    "filament_soluble",
    "filament_is_support",
    "filament_printable",
    "filament_max_volumetric_speed",
    "filament_adaptive_volumetric_speed",
    "filament_flow_ratio",
    "filament_density",
    "filament_adhesiveness_category",
    "filament_cost",
    "filament_minimal_purge_on_wipe_tower",
    "filament_tower_interface_pre_extrusion_dist",
    "filament_tower_interface_pre_extrusion_length",
    "filament_tower_ironing_area",
    "filament_tower_interface_purge_volume",
    "filament_tower_interface_print_temp",
    "nozzle_temperature",
    "nozzle_temperature_initial_layer",
    "cool_plate_temp",
    "textured_cool_plate_temp",
    "eng_plate_temp",
    "hot_plate_temp",
    "textured_plate_temp",
    "cool_plate_temp_initial_layer",
    "textured_cool_plate_temp_initial_layer",
    "eng_plate_temp_initial_layer",
    "hot_plate_temp_initial_layer",
    "textured_plate_temp_initial_layer",
    "supertack_plate_temp_initial_layer",
    "supertack_plate_temp",
    "bed_type",
    "temperature_vitrification",
    "reduce_fan_stop_start_freq",
    "dont_slow_down_outer_wall",
    "slow_down_for_layer_cooling",
    "fan_min_speed",
    "fan_max_speed",
    "enable_overhang_bridge_fan",
    "overhang_fan_speed",
    "overhang_fan_threshold",
    "close_fan_the_first_x_layers",
    "close_additional_fan_first_x_layers",
    "first_x_layer_fan_speed",
    "full_fan_speed_layer",
    "additional_fan_full_speed_layer",
    "fan_cooling_layer_time",
    "slow_down_layer_time",
    "slow_down_min_speed",
    "filament_start_gcode",
    "filament_end_gcode",
    "filament_change_extrusion_role_gcode",
    "activate_air_filtration",
    "activate_air_filtration_during_print",
    "activate_air_filtration_on_completion",
    "during_print_exhaust_fan_speed",
    "complete_print_exhaust_fan_speed",
    "filament_retraction_length",
    "filament_z_hop",
    "filament_z_hop_types",
    "filament_retract_lift_above",
    "filament_retract_lift_below",
    "filament_retract_lift_enforce",
    "filament_retraction_speed",
    "filament_deretraction_speed",
    "filament_retract_restart_extra",
    "filament_retraction_minimum_travel",
    "filament_retract_when_changing_layer",
    "filament_wipe",
    "filament_retract_before_wipe",
    "filament_vendor",
    "compatible_prints",
    "compatible_prints_condition",
    "compatible_printers",
    "compatible_printers_condition",
    "inherits",
    "filament_wipe_distance",
    "additional_cooling_fan_speed",
    "nozzle_temperature_range_low",
    "nozzle_temperature_range_high",
    "filament_extruder_variant",
    "enable_pressure_advance",
    "pressure_advance",
    "adaptive_pressure_advance",
    "adaptive_pressure_advance_model",
    "adaptive_pressure_advance_overhangs",
    "adaptive_pressure_advance_bridges",
    "chamber_temperature",
    "filament_shrink",
    "filament_shrinkage_compensation_z",
    "support_material_interface_fan_speed",
    "internal_bridge_fan_speed",
    "filament_notes",
    "ironing_fan_speed",
    "filament_ironing_flow",
    "filament_ironing_spacing",
    "filament_ironing_inset",
    "filament_ironing_speed",
    "filament_loading_speed",
    "filament_loading_speed_start",
    "filament_unloading_speed",
    "filament_unloading_speed_start",
    "filament_toolchange_delay",
    "filament_cooling_moves",
    "filament_stamping_loading_speed",
    "filament_stamping_distance",
    "filament_cooling_initial_speed",
    "filament_cooling_final_speed",
    "filament_ramming_parameters",
    "filament_multitool_ramming",
    "filament_multitool_ramming_volume",
    "filament_multitool_ramming_flow",
    "activate_chamber_temp_control",
    "chamber_minimal_temperature",
    "filament_long_retractions_when_cut",
    "filament_retraction_distances_when_cut",
    "idle_temperature",
    "filament_change_length",
    "filament_flush_volumetric_speed",
    "filament_flush_temp",
    "filament_cooling_before_tower",
    "long_retractions_when_ec",
    "retraction_distances_when_ec",
];

/// Texte d'une valeur scalaire brute de 3MF (chaîne telle quelle, sinon JSON).
fn raw_text(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(s) => Some(s.clone()),
        other => Some(other.to_string()),
    }
}

/// Éléments d'un vecteur brut de 3MF (tableau JSON ou chaîne « a;b »).
fn raw_items(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .map(|v| {
                v.as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| v.to_string())
            })
            .collect(),
        Some(Value::String(s)) if !s.is_empty() => s
            .split(';')
            .map(|p| p.trim_matches('"').to_string())
            .collect(),
        _ => Vec::new(),
    }
}

/// Presets filament et imprimante du projet 3MF, comme Orca
/// (`PresetBundle::load_config_file_config` → `load_external_preset`) :
/// - imprimante : preset machine visible nommé `printer_settings_id` (sinon
///   dernier `inherits_group`), retenu si le compte possède ce modèle
///   d'imprimante (garde de possession du tranchage) — les valeurs machine du
///   3MF restent portées par le preset importé (process aplati) ;
/// - filaments (un par `filament_colour`) : preset visible nommé
///   `filament_settings_id[i]` s'il n'est pas modifié dans le projet
///   (`different_settings_to_system[1 + i]` vide, comme Orca qui sélectionne
///   alors le preset système), sinon preset utilisateur « <nom> (<projet>) »
///   avec les valeurs du filament i (options `Preset::filament_options()`,
///   vecteurs répartis par filament comme Orca).
async fn import_project_presets(
    state: &AppState,
    user_id: UserId,
    project_name: &str,
    config: &Map<String, Value>,
) -> ApiResult<(Option<PresetId>, Vec<PresetId>)> {
    let different = raw_items(config.get("different_settings_to_system"));
    let inherits = raw_items(config.get("inherits_group"));
    let colours = raw_items(config.get("filament_colour"));
    let machine_conditions = raw_items(config.get("compatible_machine_expression_group"));
    let process_conditions = raw_items(config.get("compatible_process_expression_group"));
    let count = colours.len();

    // Imprimante.
    let machines = state
        .storage
        .presets()
        .list_by_kind(PresetKind::Machine, user_id)
        .await?;
    let printer_name = raw_text(config.get("printer_settings_id")).filter(|n| !n.is_empty());
    let candidates = [
        printer_name,
        inherits.last().cloned().filter(|n| !n.is_empty()),
    ];
    let printer = candidates.iter().flatten().find_map(|name| {
        machines
            .iter()
            .filter(|p| &p.name == name && p.instantiation)
            // Preset utilisateur du même nom prioritaire (comme Orca).
            .max_by_key(|p| p.origin == PresetOrigin::User)
    });
    let printer = match printer {
        Some(preset) => {
            let owned = state.storage.printers().list(user_id).await?;
            let model = super::slice::machine_model(&machines, preset);
            let owns = owned.iter().any(|o| {
                machines
                    .iter()
                    .find(|p| p.id == o.machine_preset_id)
                    .is_some_and(|p| super::slice::machine_model(&machines, p) == model)
            });
            owns.then_some(preset.id)
        }
        None => None,
    };

    // Filaments.
    if count == 0 {
        return Ok((printer, Vec::new()));
    }
    let visible = state
        .storage
        .presets()
        .list_by_kind(PresetKind::Filament, user_id)
        .await?;
    let names = raw_items(config.get("filament_settings_id"));
    let ids = raw_items(config.get("filament_ids"));
    let self_index: Vec<usize> = raw_items(config.get("filament_self_index"))
        .iter()
        .filter_map(|v| v.trim().parse().ok())
        .collect();
    let mut filaments = Vec::with_capacity(count);
    for index in 0..count {
        let name = names.get(index).cloned().unwrap_or_default();
        let existing = visible
            .iter()
            .filter(|p| !name.is_empty() && p.name == name && p.instantiation)
            .max_by_key(|p| p.origin == PresetOrigin::User);
        let modified = different
            .get(1 + index)
            .is_some_and(|keys| !keys.trim().is_empty());
        if let (Some(preset), false) = (existing, modified) {
            filaments.push(preset.id);
            continue;
        }
        let mut values = filament_slot_values(config, index, count, &self_index);
        if let Some(colour) = colours.get(index) {
            values.insert("filament_colour".into(), json!([colour]));
        }
        // load_config_file_config : compatible_printers_condition =
        // compatible_machine_expression_group[i + 1] (entrée 0 = process),
        // compatible_prints_condition = compatible_process_expression_group[i].
        values.insert(
            "compatible_printers_condition".into(),
            json!(machine_conditions
                .get(1 + index)
                .cloned()
                .unwrap_or_default()),
        );
        values.insert(
            "compatible_prints_condition".into(),
            json!(process_conditions.get(index).cloned().unwrap_or_default()),
        );
        let parent = inherits
            .get(1 + index)
            .filter(|n| !n.is_empty())
            .cloned()
            .or_else(|| existing.map(|p| p.name.clone()));
        let base = if name.is_empty() {
            format!("Filament {} ({project_name})", index + 1)
        } else {
            format!("{name} ({project_name})")
        };
        let preset = create_user_preset_from_raw(
            state,
            user_id,
            PresetKind::Filament,
            &base,
            parent,
            ids.get(index).filter(|id| !id.is_empty()).cloned(),
            values,
        )
        .await?;
        filaments.push(preset.id);
    }
    Ok((printer, filaments))
}

/// Valeurs du filament `index` d'une config 3MF (load_config_file_config :
/// scalaires copiés, vecteurs → élément i ; vecteurs par variante
/// d'extrudeur → éléments dont filament_self_index vaut i + 1). Clés propres
/// au preset (héritage, compatibilités) jamais reprises : Orca les tire des
/// vecteurs *_group.
fn filament_slot_values(
    config: &Map<String, Value>,
    index: usize,
    count: usize,
    self_index: &[usize],
) -> Map<String, Value> {
    let mut values = Map::new();
    for key in ORCA_FILAMENT_OPTIONS {
        if matches!(
            key,
            "inherits"
                | "compatible_printers"
                | "compatible_prints"
                | "compatible_printers_condition"
                | "compatible_prints_condition"
        ) {
            continue;
        }
        let Some(value) = config.get(key) else {
            continue;
        };
        let picked = match value {
            Value::Array(items) if items.len() == count => Value::Array(vec![items[index].clone()]),
            Value::Array(items) if self_index.len() == items.len() && items.len() > count => {
                Value::Array(
                    items
                        .iter()
                        .zip(self_index)
                        .filter(|(_, owner)| **owner == index + 1)
                        .map(|(item, _)| item.clone())
                        .collect(),
                )
            }
            other => other.clone(),
        };
        values.insert(key.to_string(), picked);
    }
    values
}

/// Lot 43 (audit 42 I3) : surcharges filament d'un projet 3MF. Orca
/// (PresetCollection::load_external_preset) sélectionne le preset filament
/// nommé puis lui applique les valeurs du projet : toute valeur du filament i
/// du project_settings.config qui diffère reste en vigueur et le preset
/// apparaît « (modifié) » — y compris quand different_settings_to_system est
/// vide (ex. filament_flow_ratio = 1 sur « Generic PLA @System » qui vaut
/// 0,98). Comparaison avec l'élément i de la config **effective** des presets
/// retenus (active, surcharges process / imprimante comprises) : seules les
/// valeurs qui changeraient le tranchage deviennent des surcharges. Une
/// entrée par filament, None si identique.
async fn imported_filament_overrides(
    state: &AppState,
    user_id: UserId,
    config: &Map<String, Value>,
    active: &Value,
    filaments: &[PresetId],
) -> ApiResult<Vec<Option<Map<String, Value>>>> {
    let count = raw_items(config.get("filament_colour")).len();
    let (merged, _) = super::slice::resolve_active_config(state, user_id, active).await?;
    let merged_json = super::presets::config_to_json(&merged);
    let merged_self_index: Vec<usize> = match merged_json.get("filament_self_index") {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|v| v.as_u64().map(|n| n as usize))
            .collect(),
        _ => Vec::new(),
    };
    let different = raw_items(config.get("different_settings_to_system"));
    let self_index: Vec<usize> = raw_items(config.get("filament_self_index"))
        .iter()
        .filter_map(|v| v.trim().parse().ok())
        .collect();
    let mut out = Vec::with_capacity(filaments.len());
    for index in 0..filaments.len() {
        if index >= count {
            out.push(None);
            continue;
        }
        let values = filament_slot_values(config, index, count, &self_index);
        let project = engine::presets::resolve_preset_chain(&[engine::api::RawPreset {
            name: "3MF".into(),
            inherits: None,
            values,
        }])
        .map_err(|e| {
            tracing::warn!(error = %e, "filament 3MF non résolu");
            ApiError::validation("configuration 3MF illisible", json!({}))
        })?;
        // Élément i de la config effective, même découpe que le projet.
        let effective_values =
            filament_slot_values(&merged_json, index, filaments.len(), &merged_self_index);
        let preset = engine::presets::resolve_preset_chain(&[engine::api::RawPreset {
            name: "effective".into(),
            inherits: None,
            values: effective_values,
        }])
        .map_err(|e| {
            tracing::error!(error = %e, "config filament effective");
            ApiError::internal()
        })?;
        let listed: Vec<String> = different
            .get(1 + index)
            .map(|keys| {
                keys.split(';')
                    .map(|k| k.trim().trim_matches('"').to_string())
                    .filter(|k| !k.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        out.push(super::overrides::modified_values(
            &project,
            &preset,
            &ORCA_FILAMENT_OPTIONS,
            &listed,
            &["filament_extruder_variant", "default_filament_colour"],
        ));
    }
    Ok(out)
}

/// Preset process visible nommé par le projet (`print_settings_id`, sinon
/// `inherits_group[0]`), anciens noms compris (`find_preset_renamed`) ; preset
/// utilisateur du même nom prioritaire, comme pour l'imprimante.
async fn named_process_preset(
    state: &AppState,
    user_id: UserId,
    config: &Map<String, Value>,
) -> ApiResult<Option<PresetId>> {
    let inherits = raw_items(config.get("inherits_group"));
    let candidates = [
        raw_text(config.get("print_settings_id")).filter(|n| !n.is_empty()),
        inherits.first().cloned().filter(|n| !n.is_empty()),
    ];
    let visible = state
        .storage
        .presets()
        .list_by_kind(PresetKind::Process, user_id)
        .await?;
    Ok(candidates.iter().flatten().find_map(|name| {
        visible
            .iter()
            .filter(|p| p.instantiation && (&p.name == name || p.renamed_from.contains(name)))
            .max_by_key(|p| (p.name == *name, p.origin == PresetOrigin::User))
            .map(|p| p.id)
    }))
}

/// Preset utilisateur à partir de valeurs brutes Orca (clés hors registre
/// filtrées comme pour le process importé), nom rendu unique par suffixe.
async fn create_user_preset_from_raw(
    state: &AppState,
    user_id: UserId,
    kind: PresetKind,
    base: &str,
    inherits: Option<String>,
    filament_id: Option<String>,
    raw: Map<String, Value>,
) -> ApiResult<Preset> {
    let resolved = engine::presets::resolve_preset_chain(&[engine::api::RawPreset {
        name: base.into(),
        inherits: None,
        values: raw.clone(),
    }])
    .map_err(|e| {
        tracing::warn!(error = %e, "preset 3MF non résolu");
        ApiError::validation("configuration 3MF illisible", json!({}))
    })?;
    let values: Map<String, Value> = raw
        .into_iter()
        .filter(|(key, _)| resolved.0.contains_key(key))
        .collect();
    let existing = state.storage.presets().list_by_kind(kind, user_id).await?;
    let mut name = base.to_string();
    let mut suffix = 2;
    while existing.iter().any(|preset| preset.name == name) {
        name = format!("{base} {suffix}");
        suffix += 1;
    }
    Ok(state
        .storage
        .presets()
        .create_user_preset(
            user_id,
            Preset {
                id: PresetId::new(),
                kind,
                name,
                origin: PresetOrigin::User,
                user_id: Some(user_id),
                vendor: None,
                inherits,
                instantiation: true,
                setting_id: None,
                filament_id,
                compatible_printers: None,
                renamed_from: Vec::new(),
                values: Value::Object(values),
            },
        )
        .await?)
}

/// Conserve la configuration embarquée dans un preset utilisateur autonome.
/// Le projet 3MF contient une configuration aplatie mêlant machine/process/
/// filament ; le registre Orca filtre les clés inconnues et le brut d'origine
/// est conservé pour que la résolution ultérieure garde les bons types.
async fn create_imported_preset(
    state: &AppState,
    user_id: UserId,
    project_name: &str,
    raw: Map<String, Value>,
) -> ApiResult<Preset> {
    let raw = split_print_slot(raw);
    let resolved = engine::presets::resolve_preset_chain(&[engine::api::RawPreset {
        name: "3MF importé".into(),
        inherits: None,
        values: raw.clone(),
    }])
    .map_err(|e| {
        tracing::warn!(error = %e, "configuration 3MF non résolue");
        ApiError::validation("configuration 3MF illisible", json!({}))
    })?;
    let values: Map<String, Value> = raw
        .into_iter()
        .filter(|(key, _)| resolved.0.contains_key(key))
        .collect();
    let existing = state
        .storage
        .presets()
        .list_by_kind(PresetKind::Process, user_id)
        .await?;
    let base = format!("{project_name} (configuration 3MF)");
    let mut name = base.clone();
    let mut suffix = 2;
    while existing.iter().any(|preset| preset.name == name) {
        name = format!("{base} {suffix}");
        suffix += 1;
    }
    Ok(state
        .storage
        .presets()
        .create_user_preset(
            user_id,
            Preset {
                id: PresetId::new(),
                kind: PresetKind::Process,
                name,
                origin: PresetOrigin::User,
                user_id: Some(user_id),
                vendor: None,
                inherits: None,
                instantiation: true,
                setting_id: None,
                filament_id: None,
                compatible_printers: None,
                renamed_from: Vec::new(),
                values: Value::Object(values),
            },
        )
        .await?)
}

/// Part « process » de `PresetBundle::load_config_file_config` (Orca, import
/// d'un 3MF) : `inherits` et `compatible_printers_condition` du preset
/// d'impression viennent de l'entrée 0 de `inherits_group` /
/// `compatible_machine_expression_group`, `compatible_printers` de
/// `print_compatible_printers` ; les valeurs brutes du fichier sont écrasées.
/// Les vecteurs `*_group` restent portés par le preset importé (entrées
/// filaments/imprimante) et sont regroupés à l'identique à l'export.
fn split_print_slot(mut raw: Map<String, Value>) -> Map<String, Value> {
    let first = |value: Option<&Value>| match value {
        Some(Value::Array(items)) => items
            .first()
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        Some(Value::String(s)) => s
            .split(';')
            .next()
            .unwrap_or("")
            .trim_matches('"')
            .to_string(),
        _ => String::new(),
    };
    let condition = first(raw.get("compatible_machine_expression_group"));
    for key in [
        "inherits",
        "compatible_prints",
        "compatible_prints_condition",
    ] {
        raw.remove(key);
    }
    raw.insert(
        "compatible_printers_condition".into(),
        Value::String(condition),
    );
    match raw.get("print_compatible_printers").cloned() {
        Some(Value::Array(items)) if !items.is_empty() => {
            raw.insert("compatible_printers".into(), Value::Array(items));
        }
        _ => {
            raw.remove("compatible_printers");
        }
    }
    raw
}

fn details(filename: &str) -> Value {
    json!({ "filename": filename })
}

/// Nom de projet dérivé du nom de fichier (sans extension, repli si vide).
fn project_name_from(filename: &str) -> String {
    let stem = std::path::Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .trim();
    if stem.is_empty() {
        "Projet importé".to_string()
    } else {
        stem.to_string()
    }
}

/// `GET /api/projects/{id}` — ouvre un projet (404 si absent ou d'autrui).
pub async fn get(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ProjectResponse>> {
    let id = parse_project_id(&id)?;
    let project = state.storage.projects().get(user.id, id).await?;
    Ok(Json(with_gcode_thumbnail(&state, user.id, project).await))
}

/// `PUT /api/projects/{id}` — sauvegarde (verrou optimiste, 409 si décalé).
pub async fn save(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<SaveProjectRequest>,
) -> ApiResult<Json<ProjectResponse>> {
    let id = parse_project_id(&id)?;
    let project = state
        .storage
        .projects()
        .update(
            user.id,
            id,
            body.expected_version,
            body.scene,
            // Surcharges (réglages modifiés non enregistrés, lot 40) vérifiées.
            super::overrides::normalize_active_presets(body.active_presets)?,
            None,
        )
        .await?;
    spawn_scene_thumbnail(&state, user.id, &project);
    Ok(Json(project.into()))
}

/// `PATCH /api/projects/{id}` — mise à jour partielle (lot 26, phase 26 E-a :
/// la route répondait 405). `name` → renommage ; `scene` / `active_presets`
/// → sauvegarde du seul champ fourni (l'autre est conservé) sous verrou
/// optimiste (`expected_version` obligatoire, 409 si décalé).
pub async fn patch(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<PatchProjectRequest>,
) -> ApiResult<Json<ProjectResponse>> {
    let id = parse_project_id(&id)?;
    let touches_content = body.scene.is_some() || body.active_presets.is_some();
    if body.name.is_none() && !touches_content {
        return Err(ApiError::validation(
            "aucun champ à modifier (name, scene, active_presets)",
            json!({}),
        ));
    }
    let name = match body.name.as_deref().map(str::trim) {
        Some("") => {
            return Err(ApiError::validation(
                "le nom du projet est requis",
                json!({ "field": "name" }),
            ))
        }
        other => other.map(str::to_string),
    };
    // Projet d'autrui ou absent → 404 avant toute écriture.
    let mut project = state.storage.projects().get(user.id, id).await?;
    let active_presets = body
        .active_presets
        .map(super::overrides::normalize_active_presets)
        .transpose()?;
    if touches_content {
        let Some(expected_version) = body.expected_version else {
            return Err(ApiError::validation(
                "expected_version est requis pour modifier scene ou active_presets",
                json!({ "field": "expected_version" }),
            ));
        };
        project = state
            .storage
            .projects()
            .update(
                user.id,
                id,
                expected_version,
                body.scene.unwrap_or(project.scene),
                active_presets.unwrap_or(project.active_presets),
                None,
            )
            .await?;
        spawn_scene_thumbnail(&state, user.id, &project);
    }
    if let Some(name) = name {
        project = rename_project(&state, user.id, id, &name).await?;
    }
    Ok(Json(with_gcode_thumbnail(&state, user.id, project).await))
}

/// Vignette de la bibliothèque recalculée à chaque sauvegarde (comme Orca,
/// Metadata/plate_1.png). En tâche de fond : un modèle encore en conversion ou
/// une scène vide laissent l'ancienne vignette.
pub(crate) fn spawn_scene_thumbnail(
    state: &AppState,
    user: UserId,
    project: &crate::domain::Project,
) {
    let state = state.clone();
    let (project_id, scene) = (project.id, project.scene.clone());
    tokio::spawn(async move {
        let model =
            match crate::slicing::build_scene_model(&state, user, project_id, &scene, None).await {
                Ok(model) => model,
                Err(e) => {
                    tracing::debug!(error = %e, "vignette de projet non rendue");
                    return;
                }
            };
        let Ok(Some(png)) =
            tokio::task::spawn_blocking(move || crate::thumbnail::render(&model)).await
        else {
            return;
        };
        if let Err(e) = state
            .files
            .write_project_thumbnail(user, project_id.0, &png)
            .await
        {
            tracing::warn!(error = %e, "écriture de la vignette de projet");
        }
    });
}

fn has_scene_thumbnail(state: &AppState, user: UserId, project: ProjectId) -> bool {
    state
        .files
        .project_thumbnail_path(user, project.0)
        .is_file()
}

/// `DELETE /api/projects/{id}` — supprime un projet.
pub async fn delete(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let id = parse_project_id(&id)?;
    state.storage.projects().delete(user.id, id).await?;
    super::metadata::remove(&state, user.id, id).await;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/projects/{id}/duplicate` — copie isolée du projet, de ses
/// modèles/fichiers/maillages et de toutes les références de la scène.
pub async fn duplicate(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<(StatusCode, Json<ProjectResponse>)> {
    let id = parse_project_id(&id)?;
    let source = state.storage.projects().get(user.id, id).await?;
    let models = state.storage.models().list(user.id, Some(id)).await?;
    // Quota de stockage (lot 32) : la copie duplique les fichiers des modèles.
    let copied: i64 = models.iter().map(|m| m.size_bytes.max(0)).sum();
    crate::http::quota::ensure_storage_room(&state, user.id, copied).await?;

    let thumbnail_path = match source.thumbnail_path.as_deref() {
        Some(path) => {
            let bytes = state
                .files
                .read(std::path::Path::new(path))
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "lecture de la vignette à dupliquer");
                    ApiError::internal()
                })?;
            let copy = state
                .files
                .write_thumbnail(user.id, GcodeId::new(), &bytes)
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "écriture de la vignette dupliquée");
                    ApiError::internal()
                })?;
            Some(copy.to_string_lossy().into_owned())
        }
        None => None,
    };
    let copy = create_unique_project(
        &state,
        user.id,
        NewProject {
            name: format!("{} (copie)", source.name),
            scene: default_scene(),
            active_presets: source.active_presets.clone(),
            thumbnail_path,
        },
    )
    .await?;

    let mut id_map = std::collections::HashMap::new();
    let mut pending = Vec::new();
    for model in &models {
        match super::models::clone_model_record(&state, user.id, copy.id, model).await {
            Ok(cloned) => {
                id_map.insert(model.id.to_string(), cloned.id.to_string());
                if cloned.mesh_path.is_none() && cloned.conversion_error.is_none() {
                    pending.push(cloned);
                }
            }
            Err(error) => {
                let _ = state.storage.projects().delete(user.id, copy.id).await;
                return Err(error);
            }
        }
    }
    let scene = remap_scene(source.scene, &id_map);
    if let Some(metadata) = super::metadata::load(&state, user.id, id).await {
        super::metadata::store(&state, user.id, copy.id, &metadata).await?;
    }
    let copy = state
        .storage
        .projects()
        .update(
            user.id,
            copy.id,
            copy.version,
            scene,
            source.active_presets,
            copy.thumbnail_path,
        )
        .await?;
    for model in pending {
        super::models::spawn_conversion(&state, model);
    }
    Ok((StatusCode::CREATED, Json(copy.into())))
}

/// Régénère les identifiants d'objets et de plateaux et remplace les références
/// aux modèles. Les autres propriétés du document sont copiées à l'identique.
fn remap_scene(mut scene: Value, model_ids: &std::collections::HashMap<String, String>) -> Value {
    let mut object_ids = std::collections::HashMap::new();
    if let Some(objects) = scene.get_mut("objects").and_then(Value::as_array_mut) {
        for object in objects {
            let Some(document) = object.as_object_mut() else {
                continue;
            };
            let new_id = Uuid::new_v4().to_string();
            if let Some(old_id) = document.get("id").and_then(Value::as_str) {
                object_ids.insert(old_id.to_string(), new_id.clone());
            }
            document.insert("id".into(), Value::String(new_id));
            for key in ["modelId", "model_id"] {
                if let Some(old) = document.get(key).and_then(Value::as_str) {
                    if let Some(new) = model_ids.get(old) {
                        document.insert(key.into(), Value::String(new.clone()));
                    }
                }
            }
            // Volumes maillés (« Split to parts ») : leur maillage est un modèle
            // du projet, copié avec un nouvel identifiant.
            if let Some(volumes) = document.get_mut("volumes").and_then(Value::as_array_mut) {
                for volume in volumes.iter_mut().filter_map(Value::as_object_mut) {
                    if let Some(new) = volume
                        .get("modelId")
                        .and_then(Value::as_str)
                        .and_then(|old| model_ids.get(old))
                    {
                        volume.insert("modelId".into(), Value::String(new.clone()));
                    }
                }
            }
        }
    }

    let mut plate_ids = std::collections::HashMap::new();
    if let Some(plates_value) = scene.get_mut("plates") {
        let plates = match plates_value {
            Value::Array(plates) => Some(plates),
            Value::Object(document) => document.get_mut("plates").and_then(Value::as_array_mut),
            _ => None,
        };
        if let Some(plates) = plates {
            for plate in plates {
                let Some(document) = plate.as_object_mut() else {
                    continue;
                };
                let new_id = Uuid::new_v4().to_string();
                if let Some(old_id) = document.get("id").and_then(Value::as_str) {
                    plate_ids.insert(old_id.to_string(), new_id.clone());
                }
                document.insert("id".into(), Value::String(new_id));
                if let Some(ids) = document.get_mut("objectIds").and_then(Value::as_array_mut) {
                    for id in ids {
                        if let Some(new) = id.as_str().and_then(|old| object_ids.get(old)) {
                            *id = Value::String(new.clone());
                        }
                    }
                }
            }
        }
        if let Value::Object(document) = plates_value {
            if let Some(active) = document.get("activeId").and_then(Value::as_str) {
                if let Some(new) = plate_ids.get(active) {
                    document.insert("activeId".into(), Value::String(new.clone()));
                }
            }
        }
    }
    scene
}

/// `PATCH /api/projects/{id}/rename` — renomme un projet.
pub async fn rename(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<RenameProjectRequest>,
) -> ApiResult<Json<ProjectResponse>> {
    let id = parse_project_id(&id)?;
    let name = body.name.trim();
    if name.is_empty() {
        return Err(ApiError::validation(
            "le nom du projet est requis",
            json!({ "field": "name" }),
        ));
    }
    let project = rename_project(&state, user.id, id, name).await?;
    Ok(Json(with_gcode_thumbnail(&state, user.id, project).await))
}

/// `GET /api/projects/{id}/thumbnail` — vignette du projet si elle existe.
pub async fn thumbnail(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let id = parse_project_id(&id)?;
    // Scopé par compte : un projet d'autrui → 404 avant toute lecture.
    let project = state.storage.projects().get(user.id, id).await?;
    if let Some(path) = project.thumbnail_path.as_deref() {
        if let Ok(bytes) = state.files.read(std::path::Path::new(path)).await {
            return Ok((
                [
                    (header::CONTENT_TYPE, "image/png"),
                    (header::CACHE_CONTROL, "no-cache"),
                ],
                bytes,
            )
                .into_response());
        }
    }
    // Vignette de la scène rendue à la sauvegarde (bibliothèque, F21).
    if let Ok(bytes) =
        tokio::fs::read(state.files.project_thumbnail_path(user.id, project.id.0)).await
    {
        return Ok((
            [
                (header::CONTENT_TYPE, "image/png"),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            bytes,
        )
            .into_response());
    }
    // Repli : vignette embarquée dans le dernier G-code réussi du projet.
    let jobs = state.storage.jobs().list(user.id).await?;
    let thumb = latest_gcode_thumbnail_from(&state, user.id, project.id, &jobs)
        .await
        .ok_or_else(|| ApiError::not_found("Vignette"))?;
    Ok((
        [
            (header::CONTENT_TYPE, thumb.content_type),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        thumb.bytes,
    )
        .into_response())
}

/// Vignette affichable décodée d'un G-code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DecodedThumbnail {
    pub content_type: &'static str,
    pub width: u64,
    pub height: u64,
    pub bytes: Vec<u8>,
}

/// Type MIME d'une image affichable par un navigateur (PNG/JPEG) ; `None`
/// pour les autres formats Orca (QOI, …) ou des octets invalides.
fn image_content_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else {
        None
    }
}

/// Choisit la plus grande vignette affichable parmi le JSON `gcodes.thumbnails`
/// (`[{width,height,data(base64)}]`, cf. `gcode::extract_thumbnails`).
pub(crate) fn best_thumbnail(thumbnails: &Value) -> Option<DecodedThumbnail> {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    let list = thumbnails.as_array()?;
    let mut best: Option<DecodedThumbnail> = None;
    for t in list {
        let Some(data) = t.get("data").and_then(Value::as_str) else {
            continue;
        };
        let clean: String = data.chars().filter(|c| !c.is_whitespace()).collect();
        let Ok(bytes) = STANDARD.decode(clean.as_bytes()) else {
            continue;
        };
        let Some(content_type) = image_content_type(&bytes) else {
            continue;
        };
        let width = t.get("width").and_then(Value::as_u64).unwrap_or(0);
        let height = t.get("height").and_then(Value::as_u64).unwrap_or(0);
        let better = best
            .as_ref()
            .is_none_or(|b| width * height > b.width * b.height);
        if better {
            best = Some(DecodedThumbnail {
                content_type,
                width,
                height,
                bytes,
            });
        }
    }
    best
}

/// Vignette du dernier job réussi (G-code présent) du projet, parmi `jobs`
/// (déjà scopés par compte). Le G-code est relu via le repo scopé `owner`.
async fn latest_gcode_thumbnail_from(
    state: &AppState,
    owner: UserId,
    project: ProjectId,
    jobs: &[crate::domain::SlicingJob],
) -> Option<DecodedThumbnail> {
    let job = jobs
        .iter()
        .filter(|j| {
            j.project_id == project
                && j.user_id == owner
                && j.status == crate::domain::JobStatus::Succeeded
                && j.gcode_id.is_some()
        })
        .max_by_key(|j| (j.created_at, j.updated_at))?;
    let gcode = state
        .storage
        .gcodes()
        .get(owner, job.gcode_id?)
        .await
        .ok()?;
    best_thumbnail(&gcode.thumbnails)
}

/// `ProjectResponse` dont `has_thumbnail` tient compte du dernier G-code.
async fn with_gcode_thumbnail(
    state: &AppState,
    owner: UserId,
    project: crate::domain::Project,
) -> ProjectResponse {
    let id = project.id;
    let scene_thumb = has_scene_thumbnail(state, owner, id);
    let needs_gcode = project.thumbnail_path.is_none() && !scene_thumb;
    let mut resp: ProjectResponse = project.into();
    resp.has_thumbnail |= scene_thumb;
    if needs_gcode {
        if let Ok(jobs) = state.storage.jobs().list(owner).await {
            resp.has_thumbnail = latest_gcode_thumbnail_from(state, owner, id, &jobs)
                .await
                .is_some();
        }
    }
    resp
}

#[cfg(test)]
mod thumbnail_tests {
    use super::*;
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;

    fn png(extra: u8) -> Vec<u8> {
        let mut v = b"\x89PNG\r\n\x1a\n".to_vec();
        v.push(extra);
        v
    }

    #[test]
    fn picks_largest_displayable_thumbnail() {
        let thumbs = json!([
            { "width": 32, "height": 32, "data": STANDARD.encode(png(1)) },
            { "width": 300, "height": 300, "data": "cW9pZgAAAA==" }, // QOI : ignoré
            { "width": 128, "height": 128, "data": STANDARD.encode(png(2)) },
            { "width": 64, "height": 64, "data": "%%%invalid%%%" },
        ]);
        let best = best_thumbnail(&thumbs).unwrap();
        assert_eq!(best.content_type, "image/png");
        assert_eq!((best.width, best.height), (128, 128));
        assert_eq!(best.bytes, png(2));
    }

    #[test]
    fn accepts_jpeg_and_tolerates_whitespace() {
        let jpg = STANDARD.encode([0xFF, 0xD8, 0xFF, 0xE0, 0, 1]);
        let (a, b) = jpg.split_at(4);
        let thumbs = json!([{ "width": 2, "height": 2, "data": format!("{a}\n {b}") }]);
        assert_eq!(best_thumbnail(&thumbs).unwrap().content_type, "image/jpeg");
    }

    #[test]
    fn none_without_usable_thumbnail() {
        assert!(best_thumbnail(&json!([])).is_none());
        assert!(best_thumbnail(&json!(null)).is_none());
        assert!(best_thumbnail(&json!([{ "width": 1, "height": 1 }])).is_none());
    }
}

#[cfg(test)]
mod name_tests {
    use super::*;

    #[test]
    fn unique_names_follow_the_interface_rule() {
        assert_eq!(name_base("cube (2)"), "cube");
        assert_eq!(name_base("cube (12)"), "cube");
        assert_eq!(name_base("cube (1)"), "cube (1)");
        assert_eq!(name_base("cube (copie)"), "cube (copie)");
        assert_eq!(name_base("(2)"), "(2)");
        assert_eq!(name_base("Sans titre"), "Sans titre");
        let taken: HashSet<String> = ["Sans titre", "Sans titre (2)", "cube (copie)"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(next_free_name("Sans titre", &taken), "Sans titre (3)");
        assert_eq!(next_free_name("Sans titre (2)", &taken), "Sans titre (3)");
        assert_eq!(next_free_name("cube (copie)", &taken), "cube (copie) (2)");
    }
}

#[cfg(test)]
mod import_3mf_tests {
    use super::*;

    fn config(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    fn base() -> Value {
        json!({
            "filament_diameter": ["1.75", "1.75", "1.75"],
            "printer_settings_id": "Bambu Lab X1 Carbon 0.4 nozzle",
            "filament_settings_id": ["Bambu PLA Basic @BBL X1C", "Bambu PETG HF @BBL X1C", ""],
            "inherits_group": ["", "", "", "", ""],
            "different_settings_to_system": ["", "", "", "", ""]
        })
    }

    fn trusted(name: &str) -> bool {
        name.starts_with("Bambu")
    }

    #[test]
    fn validate_presets_success_when_all_system() {
        assert!(validate_presets(&config(base()), trusted, trusted).is_none());
    }

    #[test]
    fn validate_presets_customized_printer_and_filament() {
        let mut v = base();
        v["printer_settings_id"] = json!("Mon imprimante");
        v["filament_settings_id"][1] = json!("Mon PETG");
        let w = validate_presets(&config(v.clone()), trusted, trusted).unwrap();
        assert_eq!(w.kind, "customized_presets");
        assert_eq!(
            w.items,
            vec!["Mon PETG".to_string(), "Mon imprimante".to_string()]
        );
        // Héritage d'un preset système (inherits_group[i + 1], imprimante en dernier).
        v["inherits_group"] = json!(["", "", "Bambu PETG HF @BBL X1C", "", "Bambu Lab X1C"]);
        assert!(validate_presets(&config(v), trusted, trusted).is_none());
    }

    #[test]
    fn validate_presets_modified_gcodes_like_orca_loop() {
        let mut v = base();
        // Index 1 (filament 1) examiné ; index 3 (dernier filament) et 4
        // (imprimante) ignorés par la boucle d'Orca (1 ≤ i < filament_count).
        v["different_settings_to_system"] = json!([
            "machine_start_gcode",
            "filament_start_gcode;nozzle_temperature;filament_end_gcode",
            "",
            "change_filament_gcode",
            "machine_end_gcode"
        ]);
        let w = validate_presets(&config(v), trusted, trusted).unwrap();
        assert_eq!(w.kind, "modified_gcodes");
        assert_eq!(
            w.items,
            vec![
                "filament_end_gcode".to_string(),
                "filament_start_gcode".to_string()
            ]
        );
    }

    #[test]
    fn load_preferences() {
        assert_eq!(preferred_load_mode(&json!({})), ImportLoadMode::Project);
        assert_eq!(
            preferred_load_mode(&json!({ "project_load_behaviour": "load_geometry_only" })),
            ImportLoadMode::Geometry
        );
        assert_eq!(
            preferred_load_mode(&json!({ "projectLoadBehaviour": "always_ask" })),
            ImportLoadMode::Project
        );
        assert!(no_warn_when_modified_gcodes(
            &json!({ "no_warn_when_modified_gcodes": "true" })
        ));
        assert!(no_warn_when_modified_gcodes(
            &json!({ "noWarnWhenModifiedGcodes": true })
        ));
        assert!(!no_warn_when_modified_gcodes(&json!({})));
    }

    #[test]
    fn geometry_only_drops_config_plates_and_metadata() {
        let mut imported = crate::models::threemf::ImportedProject {
            objects: Vec::new(),
            plates: vec![crate::models::threemf::ImportedPlate {
                name: "Plateau A".into(),
                plate_type: "Textured PEI Plate".into(),
                settings: config(json!({ "print_sequence": "by object" })),
                instances: vec![("1".into(), 0)],
            }],
            config: config(base()),
            metadata: Some(engine::threemf::ProjectMetadata::default()),
            filament_colours: Vec::new(),
        };
        geometry_only(&mut imported);
        assert!(imported.config.is_empty());
        assert!(imported.metadata.is_none());
        assert_eq!(imported.plates[0].name, "");
        assert!(imported.plates[0].settings.is_empty());
        assert_eq!(
            imported.plates[0].instances.len(),
            1,
            "affectation conservée"
        );
        assert_eq!(
            imported.plates[0].plate_type,
            crate::models::threemf::DEFAULT_PLATE_TYPE
        );
    }

    /// Cube 20 mm centré sur (x, y) : matrice `<build>` d'un 3MF.
    fn cube_object(
        source: &str,
        instance: usize,
        x: f64,
        y: f64,
        extruder: u64,
    ) -> crate::models::threemf::ImportedObject {
        crate::models::threemf::ImportedObject {
            name: source.into(),
            mesh: engine::api::TriangleMesh {
                vertices: vec![
                    [-10.0, -10.0, 0.0],
                    [10.0, -10.0, 0.0],
                    [10.0, 10.0, 0.0],
                    [-10.0, 10.0, 0.0],
                    [-10.0, -10.0, 20.0],
                    [10.0, -10.0, 20.0],
                    [10.0, 10.0, 20.0],
                    [-10.0, 10.0, 20.0],
                ],
                indices: vec![
                    [0, 1, 2],
                    [0, 2, 3],
                    [4, 6, 5],
                    [4, 7, 6],
                    [0, 4, 5],
                    [0, 5, 1],
                    [1, 5, 6],
                    [1, 6, 2],
                    [2, 6, 7],
                    [2, 7, 3],
                    [3, 7, 4],
                    [3, 4, 0],
                ],
            },
            matrix: glam::DMat4::from_translation(glam::DVec3::new(x, y, 0.0)),
            printable: true,
            extruder,
            settings: Map::new(),
            layer_profile: Vec::new(),
            source_id: source.into(),
            instance_index: instance,
            painting: Map::new(),
            emboss: None,
            volumes: Vec::new(),
            parts: Vec::new(),
            layer_ranges: Vec::new(),
            mesh_name: None,
            cut_id: None,
            mesh_stats: None,
        }
    }

    /// `reload_all_objects` d'Orca : l'affectation déclarée par le 3MF est
    /// ignorée, chaque objet rejoint le plateau que croise sa position —
    /// grille `stride = 200 × 1,2 = 240` → un cube à x = 250 est sur le
    /// plateau 1 même si le fichier le déclarait sur le plateau 0.
    #[test]
    fn geometry_only_reassigns_plates_by_position() {
        let mut cfg = base();
        cfg["printable_area"] = json!(["0x0", "200x0", "200x200", "0x200"]);
        let mut imported = crate::models::threemf::ImportedProject {
            objects: vec![
                cube_object("1", 0, 30.0, 30.0, 1),
                cube_object("2", 0, 250.0, 30.0, 1),
            ],
            plates: vec![
                crate::models::threemf::ImportedPlate {
                    name: "Plateau 1".into(),
                    plate_type: "Textured PEI Plate".into(),
                    settings: Map::new(),
                    // Les deux déclarés sur le plateau 0 (affectation fausse).
                    instances: vec![("1".into(), 0), ("2".into(), 0)],
                },
                crate::models::threemf::ImportedPlate {
                    name: "Plateau 2".into(),
                    plate_type: "Cool Plate / PLA Plate".into(),
                    settings: Map::new(),
                    instances: Vec::new(),
                },
            ],
            config: config(cfg),
            metadata: None,
            filament_colours: Vec::new(),
        };
        geometry_only(&mut imported);
        assert_eq!(imported.plates[0].instances, vec![("1".into(), 0)]);
        assert_eq!(imported.plates[1].instances, vec![("2".into(), 0)]);
    }

    /// Filaments manquants (Plater.cpp « add extruder … if only import
    /// geometry ») : extrudeur 3 utilisé → 2 couleurs de la palette d'Orca ;
    /// plafond `MAXIMUM_EXTRUDER_NUMBER` (16).
    #[test]
    fn geometry_only_adds_missing_filaments() {
        let mut imported = crate::models::threemf::ImportedProject {
            objects: vec![cube_object("1", 0, 30.0, 30.0, 3)],
            plates: Vec::new(),
            config: config(base()),
            metadata: None,
            filament_colours: Vec::new(),
        };
        geometry_only(&mut imported);
        assert_eq!(
            imported.filament_colours,
            vec!["#00C1AE".to_string(), "#F4E2C1".to_string()]
        );
        imported.objects[0].extruder = 20;
        imported.config = config(base());
        geometry_only(&mut imported);
        assert_eq!(imported.filament_colours.len(), 15, "plafond 16 filaments");
    }
}
