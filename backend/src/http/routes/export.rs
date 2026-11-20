//! Export de projet 3MF (`GET /api/projects/{id}/export/3mf`, T072, FR-044).
//!
//! Assemble les maillages du projet et la configuration figée (presets actifs
//! résolus) en un projet 3MF **compatible OrcaSlicer** via `engine::threemf`.
//! La géométrie convertie par le moteur, les transformations et les plateaux du
//! document de scène sont communs à l'export et au tranchage. Les surcharges
//! par objet (`settings`, `layerProfile`) et les plateaux (nom, `plateType`,
//! réglages par plateau Orca) sont écrits dans `Metadata/model_settings.config`
//! / `layer_heights_profile.txt` et relus par l'import 3MF.
//!
//! Isolation (SC-008) : un projet d'autrui répond 404.

use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use glam::{DMat4, DVec3};
use serde_json::Value;
use std::path::Path as FsPath;

use crate::domain::{ModelFormat, ProjectId};
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;
use crate::mesh::{parse_stl, Mesh};

fn parse_project_id(raw: &str) -> ApiResult<ProjectId> {
    uuid::Uuid::parse_str(raw)
        .map(ProjectId)
        .map_err(|_| ApiError::not_found("Projet"))
}

/// `GET /api/projects/{id}/export/3mf` — projet 3MF (pièce jointe).
pub async fn export_3mf(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(project_raw): Path<String>,
) -> ApiResult<Response> {
    let project_id = parse_project_id(&project_raw)?;
    let project = state.storage.projects().get(user.id, project_id).await?; // 404, SC-008

    // Géométrie : maillages STL décodés côté serveur.
    let mut model = build_project_model(&state, user.id, project_id, None).await?;

    // Configuration figée : presets actifs résolus.
    let (mut config, _warnings) =
        super::slice::resolve_active_config(&state, user.id, &project.active_presets).await?;
    // Réglages du projet (couleurs, purges) par-dessus les presets, comme au tranchage.
    super::slice::apply_project_config(&mut config, &project.scene);
    // G-code personnalisés par couche (Metadata/custom_gcode_per_layer.xml).
    let mode = crate::slicing::custom_gcode_mode(&config);
    model.custom_gcodes = crate::slicing::scene_custom_gcodes(&project.scene)
        .into_iter()
        .map(|mut plate| {
            plate.mode = mode;
            plate
        })
        .collect();
    // Config complète comme `project_settings.config` d'Orca (`full_print_config`) :
    // clés absentes de la chaîne = défauts du registre.
    super::presets::fill_registry_defaults(&mut config);
    // Clés « par preset » regroupées comme `PresetBundle::full_fff_config`.
    let groups = preset_groups(&state, user.id, &project.active_presets).await?;
    apply_full_config_groups(&mut config, &groups);

    let mut plates = scene_project_plates(&project.scene);
    attach_plate_thumbnails(&state, user.id, project_id, &model, &config, &mut plates).await;
    // Écrit par libslic3r (store_bbs_3mf) comme « Save project » d'Orca.
    // Fichiers auxiliaires du projet (Auxiliaries/), comme « Save project » d'Orca.
    let aux_dir = state.files.auxiliaries_dir(user.id, project.id);
    // Métadonnées du projet (Title, Designer, License…) : store_bbs_3mf.
    let metadata = super::metadata::load(&state, user.id, project.id).await;
    let bytes = crate::engine::project_3mf::write_project_3mf_with_aux(
        &model,
        &config,
        &plates,
        &aux_dir,
        metadata.as_ref(),
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "écriture du projet 3MF");
        ApiError::internal()
    })?;

    let filename = format!("{}.3mf", sanitize(&project.name));
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "model/3mf".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        bytes,
    )
        .into_response())
}

/// Valeurs propres à chaque preset actif qu'Orca regroupe en vecteurs dans
/// `PresetBundle::full_fff_config` (PresetBundle.cpp:3895-4147) avant
/// `save_to_json` du 3MF (`bbs_3mf.cpp`, `_add_project_config_file_to_archive`) :
/// - `compatible_machine_expression_group` = [process, filaments…]
///   (`compatible_printers_condition`, pas d'entrée imprimante) ;
/// - `compatible_process_expression_group` = [filaments…]
///   (`compatible_prints_condition`) ;
/// - `inherits_group` = [process, filaments…, imprimante] (`inherits`).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct PresetGroups {
    pub machine_conditions: Vec<String>,
    pub process_conditions: Vec<String>,
    pub inherits: Vec<String>,
}

/// `(compatible_printers_condition, compatible_prints_condition, inherits)` d'un
/// preset actif, au sens d'Orca : un preset système a `inherits` vide (la chaîne
/// vendeur est aplatie au chargement), un preset utilisateur hérite de son
/// ancêtre système le plus proche.
async fn preset_slot(
    state: &AppState,
    user: crate::domain::UserId,
    raw_id: Option<&str>,
) -> ApiResult<(String, String, String)> {
    let Some(raw_id) = raw_id else {
        return Ok(Default::default());
    };
    let Ok(uuid) = uuid::Uuid::parse_str(raw_id) else {
        return Ok(Default::default());
    };
    let leaf = super::presets::load_visible(state, user, crate::domain::PresetId(uuid)).await?;
    let chain = super::presets::load_chain(state, user, &leaf).await?;
    let resolved = engine::presets::resolve_preset_chain(&chain).map_err(|e| {
        tracing::error!(error = %e, "résolution de preset pour l'export 3MF");
        ApiError::internal()
    })?;
    let text = |key: &str| match resolved.0.get(key) {
        Some(engine::api::ConfigValue::String(s)) => s.clone(),
        Some(engine::api::ConfigValue::Strings(v)) => v.first().cloned().unwrap_or_default(),
        _ => String::new(),
    };
    let mut inherits = String::new();
    if leaf.origin == crate::domain::PresetOrigin::User {
        // Chaîne racine → feuille : on remonte depuis le parent direct.
        for raw in chain.iter().rev().skip(1) {
            let named = state
                .storage
                .presets()
                .list_by_name(leaf.kind, &raw.name, user)
                .await?;
            if named
                .iter()
                .any(|p| p.origin == crate::domain::PresetOrigin::System)
            {
                inherits = raw.name.clone();
                break;
            }
        }
    }
    Ok((
        text("compatible_printers_condition"),
        text("compatible_prints_condition"),
        inherits,
    ))
}

/// Regroupe les presets actifs du projet (`{printer, process, filaments}`).
pub(crate) async fn preset_groups(
    state: &AppState,
    user: crate::domain::UserId,
    active: &Value,
) -> ApiResult<PresetGroups> {
    let mut groups = PresetGroups::default();
    let (process_cond, _, process_inherits) =
        preset_slot(state, user, active.get("process").and_then(Value::as_str)).await?;
    groups.machine_conditions.push(process_cond);
    groups.inherits.push(process_inherits);
    let filaments: Vec<&str> = active
        .get("filaments")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    for id in filaments {
        let (cond, prints_cond, inherits) = preset_slot(state, user, Some(id)).await?;
        groups.machine_conditions.push(cond);
        groups.process_conditions.push(prints_cond);
        groups.inherits.push(inherits);
    }
    let (_, _, printer_inherits) =
        preset_slot(state, user, active.get("printer").and_then(Value::as_str)).await?;
    groups.inherits.push(printer_inherits);
    Ok(groups)
}

/// Applique à la config exportée ce que fait `full_fff_config` : les clés
/// propres à un preset (`compatible_prints(_condition)`,
/// `compatible_printers(_condition)`, `inherits`) sont retirées et remplacées
/// par leurs vecteurs `*_group`, écrits seulement si une entrée est non vide
/// (`add_if_some_non_empty`). Une entrée vide côté presets actifs reprend la
/// valeur du groupe déjà porté par la config (projet 3MF importé : Orca la
/// redistribue dans ses presets à l'import, `load_config_file_config`, puis la
/// regroupe à l'identique à l'export). `upward_compatible_machine` est une clé
/// du preset imprimante : conservée telle quelle.
pub(crate) fn apply_full_config_groups(
    config: &mut engine::api::DynamicPrintConfig,
    groups: &PresetGroups,
) {
    for key in [
        "compatible_prints",
        "compatible_prints_condition",
        "compatible_printers",
        "compatible_printers_condition",
        "inherits",
    ] {
        config.0.remove(key);
    }
    for (key, computed) in [
        (
            "compatible_machine_expression_group",
            &groups.machine_conditions,
        ),
        (
            "compatible_process_expression_group",
            &groups.process_conditions,
        ),
        ("inherits_group", &groups.inherits),
    ] {
        let carried = match config.0.remove(key) {
            Some(engine::api::ConfigValue::Strings(v)) => v,
            Some(engine::api::ConfigValue::String(s)) if !s.is_empty() => vec![s],
            _ => Vec::new(),
        };
        let mut out = computed.clone();
        if out.len() < carried.len() {
            out.resize(carried.len(), String::new());
        }
        for (slot, value) in out.iter_mut().zip(carried) {
            if slot.is_empty() {
                *slot = value;
            }
        }
        if out.iter().any(|v| !v.is_empty()) {
            config
                .0
                .insert(key.to_string(), engine::api::ConfigValue::Strings(out));
        }
    }
}

/// Corps de `POST /api/projects/{id}/export/drc`.
#[derive(Debug, serde::Deserialize)]
pub struct DrcExportRequest {
    /// Un `.drc` par objet (archive ZIP) plutôt qu'un seul fichier fusionné.
    #[serde(default)]
    pub split: bool,
    /// Objets de scène à exporter (menu objet) ; absent = toute la scène.
    #[serde(default)]
    pub object_ids: Option<Vec<String>>,
    /// DRC seulement : préférence « Quality level for Draco export » d'Orca
    /// (`drc_bits`, bits de quantification ; 0 ou absent = aucune). Normalisée
    /// comme la préférence (> 30 → 30, 1–7 → 8) ; négative → 422. Ignorée en STL.
    #[serde(default)]
    pub bits: Option<i64>,
}

/// `POST /api/projects/{id}/export/drc` — export Draco d'Orca
/// (`Plater::export_stl(..., FT_DRC)` → `store_drc`) : pièces des objets en
/// repère plateau (transformations appliquées comme l'export STL), un fichier
/// fusionné ou un ZIP d'un `.drc` par objet.
pub async fn export_drc(
    user: CurrentUser,
    state: State<AppState>,
    path: Path<String>,
    axum::Json(body): axum::Json<DrcExportRequest>,
) -> ApiResult<Response> {
    export_meshes_as(user, state, path, body, MeshExportFormat::Drc).await
}

/// `POST /api/projects/{id}/export/stl` — « Export all objects as one STL /
/// as STLs » et menu objet d'Orca (`Plater::export_stl` → `store_stl`) :
/// maillages calculés par libslic3r (volumes négatifs soustraits, toutes les
/// instances), un STL binaire fusionné ou un ZIP d'un STL par objet.
pub async fn export_stl(
    user: CurrentUser,
    state: State<AppState>,
    path: Path<String>,
    axum::Json(body): axum::Json<DrcExportRequest>,
) -> ApiResult<Response> {
    export_meshes_as(user, state, path, body, MeshExportFormat::Stl).await
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MeshExportFormat {
    Stl,
    Drc,
}

impl MeshExportFormat {
    fn ext(self) -> &'static str {
        match self {
            Self::Stl => "stl",
            Self::Drc => "drc",
        }
    }
}

async fn export_meshes_as(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(project_raw): Path<String>,
    body: DrcExportRequest,
    format: MeshExportFormat,
) -> ApiResult<Response> {
    let project_id = parse_project_id(&project_raw)?;
    let project = state.storage.projects().get(user.id, project_id).await?; // 404, SC-008
    let drc_bits = match body.bits {
        None => crate::engine::drc::DRC_BITS_DEFAULT,
        Some(bits) => crate::engine::drc::normalize_drc_bits(bits).ok_or_else(|| {
            ApiError::validation(
                "Quantification Draco invalide (0 ou 8 à 30 bits)",
                serde_json::json!({ "field": "bits" }),
            )
        })?,
    };
    let model = build_project_model_selected(
        &state,
        user.id,
        project_id,
        None,
        body.object_ids.as_deref(),
    )
    .await?;
    if model.objects.is_empty() {
        return Err(ApiError::validation(
            "Aucun objet à exporter",
            serde_json::json!({}),
        ));
    }
    let worker_error = |error: crate::engine::scene::SceneWorkerError| {
        if error.is_invalid_input() {
            return ApiError::validation(error.to_string(), serde_json::json!({}));
        }
        tracing::error!(error = %error, "export de maillages dans engine-worker");
        ApiError::internal()
    };
    // Maillages d'export calculés par Orca (export_mesh.cpp).
    let orca_meshes = crate::engine::export_meshes::export_meshes(&model)
        .await
        .map_err(worker_error)?;
    let named: Vec<(String, engine::api::TriangleMesh)> = model
        .objects
        .iter()
        .map(|object| object.name.clone())
        .zip(orca_meshes)
        .filter(|(_, mesh)| !mesh.is_empty())
        .collect();
    if named.is_empty() {
        return Err(ApiError::validation(
            "Aucun objet à exporter",
            serde_json::json!({}),
        ));
    }
    let meshes: Vec<engine::api::TriangleMesh> = if body.split {
        named.iter().map(|(_, mesh)| mesh.clone()).collect()
    } else {
        vec![merge_meshes(named.iter().map(|(_, mesh)| mesh))]
    };
    let files = match format {
        MeshExportFormat::Drc => crate::engine::drc::export_drc(&meshes, drc_bits)
            .await
            .map_err(worker_error)?,
        MeshExportFormat::Stl => meshes
            .iter()
            .map(engine::api::split::encode_binary_stl)
            .collect(),
    };
    let base = sanitize(&project.name);
    let ext = format.ext();
    let (filename, content_type, bytes) = if body.split {
        let names: Vec<&str> = named.iter().map(|(name, _)| name.as_str()).collect();
        let zip = mesh_zip(&names, files, ext).map_err(|e| {
            tracing::error!(error = %e, "archive des exports");
            ApiError::internal()
        })?;
        (format!("{base}.zip"), "application/zip", zip)
    } else {
        let name = if named.len() == 1 && body.object_ids.is_some() {
            sanitize(
                FsPath::new(&named[0].0)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&named[0].0),
            )
        } else {
            base
        };
        let bytes = files.into_iter().next().unwrap_or_default();
        (format!("{name}.{ext}"), "application/octet-stream", bytes)
    };
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, content_type.to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        bytes,
    )
        .into_response())
}

/// Concatène des maillages (indices décalés), comme `Model::mesh()` d'Orca.
fn merge_meshes<'a>(
    meshes: impl Iterator<Item = &'a engine::api::TriangleMesh>,
) -> engine::api::TriangleMesh {
    let mut out = engine::api::TriangleMesh::default();
    for mesh in meshes {
        let offset = out.vertices.len() as u32;
        out.vertices.extend_from_slice(&mesh.vertices);
        out.indices
            .extend(mesh.indices.iter().map(|t| t.map(|i| i + offset)));
    }
    out
}

/// Archive ZIP d'un `.drc` par objet, noms uniques dérivés des objets.
fn mesh_zip(names: &[&str], files: Vec<Vec<u8>>, ext: &str) -> zip::result::ZipResult<Vec<u8>> {
    use std::io::Write;
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let mut used = std::collections::HashMap::<String, usize>::new();
    for (name, bytes) in names.iter().zip(files) {
        let stem = sanitize(
            FsPath::new(name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(name),
        );
        let count = used.entry(stem.clone()).or_insert(0);
        *count += 1;
        let entry = if *count == 1 {
            format!("{stem}.{ext}")
        } else {
            format!("{stem}_{count}.{ext}")
        };
        zip.start_file(entry, options)?;
        zip.write_all(&bytes)?;
    }
    Ok(zip.finish()?.into_inner())
}

/// Réglages explicites d'un plateau de scène (`plate.settings`, restreints à
/// `engine::threemf::PLATE_SETTING_KEYS`, seules clés qu'Orca persiste par
/// plateau) : surcharges appliquées au tranchage de ce plateau.
pub(crate) fn plate_explicit_settings(plate: &Value) -> engine::api::DynamicPrintConfig {
    let values: serde_json::Map<String, Value> = plate
        .get("settings")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter(|(key, _)| engine::threemf::PLATE_SETTING_KEYS.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    engine::presets::resolve_preset_chain(&[engine::api::RawPreset {
        name: "plate".into(),
        inherits: None,
        values,
    }])
    .unwrap_or_else(|_| engine::api::DynamicPrintConfig::new())
}

/// Réglages d'export d'un plateau : réglages explicites + `plateType` →
/// `curr_bed_type` s'il n'est pas déjà fixé explicitement.
fn plate_settings_config(plate: &Value) -> engine::api::DynamicPrintConfig {
    let mut settings = plate_explicit_settings(plate);
    if !settings.0.contains_key("curr_bed_type") {
        if let Some(bed) = plate
            .get("plateType")
            .and_then(Value::as_str)
            .and_then(crate::models::threemf::plate_type_to_bed_type)
        {
            settings.0.insert(
                "curr_bed_type".into(),
                engine::api::ConfigValue::String(bed.into()),
            );
        }
    }
    settings
}

/// Vignette de chaque plateau (Orca : Metadata/plate_<n>.png + _small, une par
/// plateau) : capture envoyée par l'interface
/// (`PUT /api/projects/{id}/plates/{n}/thumbnail`) sinon rendu serveur 512 × 512.
async fn attach_plate_thumbnails(
    state: &AppState,
    user: crate::domain::UserId,
    project: ProjectId,
    model: &engine::api::Model,
    config: &engine::api::DynamicPrintConfig,
    plates: &mut Vec<engine::threemf::ProjectPlate>,
) {
    let count = model
        .objects
        .iter()
        .flat_map(|o| o.instances.iter().map(|i| i.plate_index as usize + 1))
        .chain(std::iter::once(plates.len()))
        .max()
        .unwrap_or(0)
        .max(1);
    plates.resize_with(count.max(plates.len()), Default::default);
    let colour = match config.get("filament_colour") {
        Some(engine::api::ConfigValue::Strings(values)) => values.first().cloned(),
        Some(engine::api::ConfigValue::String(value)) => {
            value.split(';').next().map(str::to_string)
        }
        _ => None,
    };
    for (index, plate) in plates.iter_mut().enumerate() {
        let path = state
            .files
            .plate_thumbnail_path(user, project.0, index as u32);
        if let Ok(bytes) = tokio::fs::read(&path).await {
            if crate::thumbnail::check_png(&bytes).is_ok() {
                plate.thumbnail_png = bytes;
                continue;
            }
        }
        let (model, colour) = (model.clone(), colour.clone());
        if let Ok(Some(png)) = tokio::task::spawn_blocking(move || {
            crate::thumbnail::render_plate(&model, index as u32, colour.as_deref())
        })
        .await
        {
            plate.thumbnail_png = png;
        }
    }
}

/// `PUT /api/projects/{id}/plates/{index}/thumbnail` — capture de la vue 3D
/// envoyée par l'interface (corps = PNG brut, 8 bits RGB/RGBA, carré conseillé,
/// ramené à 512 × 512 à l'export) pour le plateau `index` (0-based) ; 204.
pub async fn put_plate_thumbnail(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path((project_raw, plate)): Path<(String, u32)>,
    body: axum::body::Bytes,
) -> ApiResult<StatusCode> {
    let project_id = parse_project_id(&project_raw)?;
    state.storage.projects().get(user.id, project_id).await?;
    if plate >= 64 {
        return Err(ApiError::validation(
            "plateau hors plage (0–63)",
            serde_json::json!({ "plate": plate }),
        ));
    }
    crate::thumbnail::check_png(&body).map_err(|reason| {
        ApiError::validation(
            format!("vignette invalide : {reason}"),
            serde_json::json!({ "plate": plate }),
        )
    })?;
    state
        .files
        .write_plate_thumbnail(user.id, project_id.0, plate, &body)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "écriture de la vignette de plateau");
            ApiError::internal()
        })?;
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /api/projects/{id}/plates/{index}/thumbnail` — retour au rendu
/// serveur ; 204 (même si aucune capture n'était enregistrée).
pub async fn delete_plate_thumbnail(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path((project_raw, plate)): Path<(String, u32)>,
) -> ApiResult<StatusCode> {
    let project_id = parse_project_id(&project_raw)?;
    state.storage.projects().get(user.id, project_id).await?;
    let _ = tokio::fs::remove_file(
        state
            .files
            .plate_thumbnail_path(user.id, project_id.0, plate),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

/// Plateaux du document de scène au format d'export : nom, type de plaque
/// (`curr_bed_type`) et réglages par plateau optionnels `plate.settings`
/// restreints à `engine::threemf::PLATE_SETTING_KEYS` (seules clés qu'Orca
/// persiste par plateau). Même ordre que `scene_object_specs` (index = plateau).
fn scene_project_plates(scene: &Value) -> Vec<engine::threemf::ProjectPlate> {
    let plates = match scene.get("plates") {
        Some(Value::Array(plates)) => plates,
        Some(Value::Object(document)) => match document.get("plates").and_then(Value::as_array) {
            Some(plates) => plates,
            None => return Vec::new(),
        },
        _ => return Vec::new(),
    };
    plates
        .iter()
        .enumerate()
        .map(|(index, plate)| engine::threemf::ProjectPlate {
            name: plate
                .get("name")
                .and_then(Value::as_str)
                .map_or_else(|| format!("Plateau {}", index + 1), str::to_string),
            settings: plate_settings_config(plate),
            thumbnail_png: Vec::new(),
        })
        .collect()
}

/// Une occurrence d'objet dans le document de scène.
#[derive(Debug, Clone)]
struct SceneObjectSpec {
    /// Identifiant de l'objet dans le document de scène (sélection d'export).
    object_id: Option<String>,
    model_id: String,
    plate_index: u32,
    matrix: DMat4,
    printable: bool,
    extruder: Option<u16>,
    settings: engine::api::DynamicPrintConfig,
    layer_height_profile: Vec<f64>,
    /// Le frontend affiche un maillage recentré avant d'appliquer cette matrice.
    centered: bool,
    /// Peinture (supports, couture, fuzzy, MMU) du volume pièce.
    paint: engine::api::VolumePaint,
    /// Volumes ajoutés (pièce, modificateur, négatif, bloqueur/forceur).
    volumes: Vec<engine::api::ModelVolume>,
    /// Volumes maillés (« Split to parts »), maillage chargé depuis leur modèle.
    mesh_volumes: Vec<crate::slicing::MeshVolumeRef>,
    /// Plages de hauteur à réglages propres (`layerRanges`).
    layer_config_ranges: Vec<engine::api::LayerConfigRange>,
    /// Nom du volume du maillage propre (`meshName`, Orca `<nom>_1` après un
    /// split) : écrit dans `Metadata/model_settings.config` par le writer 3MF.
    mesh_name: Option<String>,
    /// Relief ré-éditable de l'objet (`emboss`, texte ou SVG créé seul).
    emboss: Option<Value>,
}

/// Assemble le `Model` moteur depuis le document de scène et les maillages
/// convertis. `plate_index = None` exporte tout le projet ; le runner cible un
/// seul plateau. Un projet d'autrui répond 404 (SC-008).
pub(crate) async fn build_project_model(
    state: &AppState,
    user: crate::domain::UserId,
    project_id: ProjectId,
    plate_index: Option<u32>,
) -> ApiResult<engine::api::Model> {
    build_project_model_selected(state, user, project_id, plate_index, None).await
}

/// Comme [`build_project_model`], restreint aux objets de scène `selection`
/// (identifiants du document) quand elle est fournie.
pub(crate) async fn build_project_model_selected(
    state: &AppState,
    user: crate::domain::UserId,
    project_id: ProjectId,
    plate_index: Option<u32>,
    selection: Option<&[String]>,
) -> ApiResult<engine::api::Model> {
    let project = state.storage.projects().get(user, project_id).await?;
    let models = state.storage.models().list(user, Some(project_id)).await?;
    let specs = scene_object_specs(&project.scene, plate_index);
    let mut model = engine::api::Model::default();

    // Les projets historiques sans document d'objets conservent le comportement
    // initial : tous leurs modèles rattachés sont exportés sur le plateau 0.
    if specs.as_ref().is_some_and(Vec::is_empty) {
        return Ok(model);
    }
    let fallback = specs.is_none();
    if fallback && plate_index.is_some_and(|index| index > 0) {
        return Ok(model);
    }

    let specs = specs.unwrap_or_else(|| {
        models
            .iter()
            .map(|m| SceneObjectSpec {
                object_id: None,
                model_id: m.id.to_string(),
                plate_index: 0,
                matrix: DMat4::IDENTITY,
                printable: true,
                extruder: None,
                settings: engine::api::DynamicPrintConfig::new(),
                layer_height_profile: Vec::new(),
                centered: false,
                paint: engine::api::VolumePaint::default(),
                volumes: Vec::new(),
                mesh_volumes: Vec::new(),
                layer_config_ranges: Vec::new(),
                mesh_name: None,
                emboss: None,
            })
            .collect()
    });

    for spec in specs {
        if let Some(selection) = selection {
            if !spec
                .object_id
                .as_ref()
                .is_some_and(|id| selection.contains(id))
            {
                continue;
            }
        }
        let Some(m) = models.iter().find(|m| m.id.to_string() == spec.model_id) else {
            return Err(ApiError::validation(
                "Le projet référence un modèle supprimé",
                serde_json::json!({ "model": spec.model_id }),
            ));
        };
        let mut mesh = load_triangle_mesh(state, m).await?;
        if spec.centered {
            center_mesh(&mut mesh);
        }
        let object = model.add_object(m.filename.clone(), mesh);
        object.settings = spec.settings;
        object.layer_height_profile = spec.layer_height_profile;
        object.layer_config_ranges = spec.layer_config_ranges;
        object.instances = vec![engine::api::ModelInstance {
            matrix: spec.matrix,
            plate_index: spec.plate_index,
            printable: spec.printable,
        }];
        if let Some(extruder) = spec.extruder {
            for volume in &mut object.volumes {
                volume.extruder = Some(extruder);
            }
        }
        // Même lecture de scène que le tranchage : peinture du volume pièce et
        // volumes ajoutés, écrits par le writer 3MF d'Orca (`custom_supports`,
        // `custom_seam`, `volume_type`, réglages de volume).
        if let Some(volume) = object
            .volumes
            .iter_mut()
            .find(|v| v.role == engine::api::VolumeRole::ModelPart)
        {
            volume.paint = spec.paint;
            volume.emboss = spec.emboss;
            if let Some(name) = spec.mesh_name {
                volume.name = name;
            }
        }
        object.volumes.extend(spec.volumes);
        for reference in spec.mesh_volumes {
            let Some(part) = models
                .iter()
                .find(|m| m.id.to_string() == reference.model_id)
            else {
                return Err(ApiError::validation(
                    "Le projet référence un volume de modèle supprimé",
                    serde_json::json!({ "model": reference.model_id }),
                ));
            };
            let mut volume = reference.resolve(load_triangle_mesh(state, part).await?);
            if volume.extruder.is_none() {
                volume.extruder = spec.extruder;
            }
            object.volumes.push(volume);
        }
    }
    Ok(model)
}

/// Charge le maillage moteur quel que soit le format source. Les anciens STL
/// sans conversion persistée restent lisibles via le parseur de secours. Un
/// modèle non-STL dont la conversion tourne encore (projet 3MF tout juste
/// importé : conversion lancée après la réponse de l'import) est attendu comme
/// `GET /api/models/{id}/mesh?wait` plutôt que de répondre 409 : dans Orca,
/// l'export d'un projet chargé réussit toujours.
async fn load_triangle_mesh(
    state: &AppState,
    model: &crate::domain::Model,
) -> ApiResult<engine::api::TriangleMesh> {
    let converted;
    let model = if model.mesh_path.is_none() && model.format != ModelFormat::Stl {
        converted = super::models::wait_converted(
            state,
            model.user_id,
            model.id,
            std::time::Duration::from_secs(super::models::EXPORT_CONVERSION_WAIT_SECS),
        )
        .await?;
        &converted
    } else {
        model
    };
    if let Some(path) = model.mesh_path.as_deref() {
        let bytes = state.files.read(FsPath::new(path)).await.map_err(|e| {
            tracing::error!(error = %e, "lecture du maillage converti");
            ApiError::internal()
        })?;
        let display = engine::api::DisplayMesh::decode(&bytes).map_err(|e| {
            tracing::error!(error = %e, "décodage WSMh (assemblage moteur)");
            ApiError::validation(
                "Maillage converti illisible",
                serde_json::json!({ "model": model.id.to_string() }),
            )
        })?;
        return Ok(engine::api::TriangleMesh {
            vertices: display
                .positions
                .chunks_exact(3)
                .map(|c| [c[0], c[1], c[2]])
                .collect(),
            indices: display
                .indices
                .chunks_exact(3)
                .map(|c| [c[0], c[1], c[2]])
                .collect(),
        });
    }
    let bytes = state
        .files
        .read(FsPath::new(&model.file_path))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "lecture du modèle (assemblage moteur)");
            ApiError::internal()
        })?;
    let mesh = parse_stl(&bytes).map_err(|e| {
        tracing::error!(error = %e, "décodage STL (assemblage moteur)");
        ApiError::validation(
            "STL illisible",
            serde_json::json!({ "model": model.id.to_string() }),
        )
    })?;
    Ok(to_triangle_mesh(&mesh))
}

/// Extrait les occurrences réellement présentes dans la scène. `None` distingue
/// un projet historique sans tableau `objects` d'une scène volontairement vide.
fn scene_object_specs(scene: &Value, requested_plate: Option<u32>) -> Option<Vec<SceneObjectSpec>> {
    let objects = scene.get("objects")?.as_array()?;
    let plates = match scene.get("plates") {
        Some(Value::Array(plates)) => Some(plates),
        Some(Value::Object(document)) => document.get("plates").and_then(Value::as_array),
        _ => None,
    };
    let mut plate_by_object = std::collections::HashMap::new();
    for (index, plate) in plates.into_iter().flatten().enumerate() {
        for id in plate
            .get("objectIds")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            plate_by_object.insert(id, index as u32);
        }
    }

    Some(
        objects
            .iter()
            .filter_map(|object| {
                let id = object.get("id").and_then(Value::as_str);
                let plate_index = id
                    .and_then(|id| plate_by_object.get(id).copied())
                    .unwrap_or(0);
                if requested_plate.is_some_and(|requested| requested != plate_index) {
                    return None;
                }
                let model_id = object.get("modelId")?.as_str()?.to_string();
                let transform = object.get("transform");
                Some(SceneObjectSpec {
                    object_id: id.map(str::to_string),
                    model_id,
                    plate_index,
                    matrix: transform.map_or(DMat4::IDENTITY, scene_transform_matrix),
                    printable: object
                        .get("printable")
                        .and_then(Value::as_bool)
                        .unwrap_or(true),
                    extruder: object
                        .get("extruder")
                        .and_then(Value::as_u64)
                        .filter(|extruder| *extruder > 0)
                        .map(|extruder| extruder as u16),
                    settings: scene_object_settings(object),
                    layer_height_profile: object
                        .get("layerProfile")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_f64)
                        .collect(),
                    centered: transform.is_some(),
                    paint: crate::slicing::scene_object_paint(object),
                    volumes: crate::slicing::scene_object_volumes(object),
                    mesh_volumes: crate::slicing::scene_object_mesh_volumes(object),
                    layer_config_ranges: crate::slicing::scene_object_layer_ranges(object),
                    emboss: crate::slicing::scene_emboss(object),
                    mesh_name: object
                        .get("meshName")
                        .and_then(Value::as_str)
                        .filter(|name| !name.trim().is_empty())
                        .map(str::to_string),
                })
            })
            .collect(),
    )
}

/// Résout les surcharges JSON d'un objet avec le même parseur que les presets.
fn scene_object_settings(object: &Value) -> engine::api::DynamicPrintConfig {
    let values = object
        .get("settings")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    engine::presets::resolve_preset_chain(&[engine::api::RawPreset {
        name: "object".into(),
        inherits: None,
        values,
    }])
    .unwrap_or_else(|_| engine::api::DynamicPrintConfig::new())
}

/// Convertit la transformation Three.js (Euler XYZ, degrés) du document en
/// matrice colonne-major libslic3r : `T × R × S` (colonne j = `R[:,j] · s_j`).
/// `pub(crate)` : source unique, partagée avec le runner de tranchage
/// (`crate::slicing`) pour que slice et export placent les objets à l'identique.
pub(crate) fn scene_transform_matrix(value: &Value) -> DMat4 {
    let vector = |key: &str, default: [f64; 3]| -> [f64; 3] {
        let mut out = default;
        if let Some(values) = value.get(key).and_then(Value::as_array) {
            for (index, target) in out.iter_mut().enumerate() {
                if let Some(number) = values.get(index).and_then(Value::as_f64) {
                    *target = number;
                }
            }
        }
        out
    };
    let [x, y, z] = vector("position", [0.0; 3]);
    let [rx, ry, rz] = vector("rotation", [0.0; 3]).map(|v| v.to_radians());
    let [sx, sy, sz] = vector("scale", [1.0; 3]);
    let (s1, c1) = rx.sin_cos();
    let (s2, c2) = ry.sin_cos();
    let (s3, c3) = rz.sin_cos();
    // Même convention que `frontend/src/lib/scene/transform.ts` (Three.js XYZ).
    let rotation = [
        [c2 * c3, -c2 * s3, s2],
        [c1 * s3 + c3 * s1 * s2, c1 * c3 - s1 * s2 * s3, -c2 * s1],
        [s1 * s3 - c1 * c3 * s2, c3 * s1 + c1 * s2 * s3, c1 * c2],
    ];
    DMat4::from_cols(
        DVec3::new(
            rotation[0][0] * sx,
            rotation[1][0] * sx,
            rotation[2][0] * sx,
        )
        .extend(0.0),
        DVec3::new(
            rotation[0][1] * sy,
            rotation[1][1] * sy,
            rotation[2][1] * sy,
        )
        .extend(0.0),
        DVec3::new(
            rotation[0][2] * sz,
            rotation[1][2] * sz,
            rotation[2][2] * sz,
        )
        .extend(0.0),
        DVec3::new(x, y, z).extend(1.0),
    )
}

/// Reproduit le recentrage du maillage effectué à l'import par le frontend.
fn center_mesh(mesh: &mut engine::api::TriangleMesh) {
    let Some((min, max)) = mesh.bounding_box() else {
        return;
    };
    let center = std::array::from_fn::<_, 3, _>(|axis| (min[axis] + max[axis]) / 2.0);
    for vertex in &mut mesh.vertices {
        for axis in 0..3 {
            vertex[axis] -= center[axis];
        }
    }
}

/// Maillage backend (aplati) → `TriangleMesh` moteur (sommets/triangles indexés).
fn to_triangle_mesh(mesh: &Mesh) -> engine::api::TriangleMesh {
    engine::api::TriangleMesh {
        vertices: mesh
            .positions
            .chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect(),
        indices: mesh
            .indices
            .chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect(),
    }
}

/// Nom de fichier sûr (alphanumérique + `-_`), repli `project`.
fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('_');
    if trimmed.is_empty() {
        "project".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn scene_selects_only_the_requested_plate() {
        let scene = json!({
            "objects": [
                {"id":"a", "modelId":"ma"},
                {"id":"b", "modelId":"mb"},
                {"id":"c", "modelId":"mc"}
            ],
            "plates": {"activeId":"p2", "plates":[
                {"id":"p1", "objectIds":["a"]},
                {"id":"p2", "objectIds":["b", "c"]}
            ]}
        });
        let specs = scene_object_specs(&scene, Some(1)).unwrap();
        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].model_id, "mb");
        assert_eq!(specs[1].model_id, "mc");
        assert!(scene_object_specs(&scene, Some(0)).unwrap().len() == 1);
    }

    #[test]
    fn scene_transform_matches_three_js_xyz_order() {
        let matrix = scene_transform_matrix(&json!({
            "position":[10.0, 20.0, 30.0],
            "rotation":[0.0, 0.0, 90.0],
            "scale":[2.0, 3.0, 4.0]
        }));
        let point = matrix.transform_point3(DVec3::new(1.0, 0.0, 0.0));
        assert!((point.x - 10.0).abs() < 1e-9);
        assert!((point.y - 22.0).abs() < 1e-9);
        assert!((point.z - 30.0).abs() < 1e-9);
    }

    #[test]
    fn centering_reproduces_the_frontend_import() {
        let mut mesh = engine::api::TriangleMesh {
            vertices: vec![[10.0, 20.0, 30.0], [14.0, 24.0, 38.0], [12.0, 22.0, 34.0]],
            indices: vec![[0, 1, 2]],
        };
        center_mesh(&mut mesh);
        assert_eq!(mesh.vertices[0], [-2.0, -2.0, -4.0]);
        assert_eq!(mesh.vertices[1], [2.0, 2.0, 4.0]);
    }

    #[test]
    fn scene_plates_carry_bed_type_and_plate_settings() {
        let scene = json!({
            "objects": [],
            "plates": {"activeId": "p1", "plates": [
                {"id": "p1", "name": "Avant", "plateType": "Smooth High Temp Plate",
                 "objectIds": [], "settings": {"print_sequence": "by object",
                 "wall_loops": 9}},
                {"id": "p2", "name": "Arrière", "plateType": "Engineering Plate",
                 "objectIds": [], "settings": {"curr_bed_type": "Textured PEI Plate"}}
            ]}
        });
        let plates = scene_project_plates(&scene);
        assert_eq!(plates.len(), 2);
        assert_eq!(plates[0].name, "Avant");
        let string = |s: &str| engine::api::ConfigValue::String(s.into());
        assert_eq!(
            plates[0].settings.0.get("curr_bed_type"),
            Some(&string("High Temp Plate"))
        );
        assert_eq!(
            plates[0].settings.0.get("print_sequence"),
            Some(&string("by object"))
        );
        // Pas de représentation par plateau dans le format Orca : ignorée.
        assert!(!plates[0].settings.0.contains_key("wall_loops"));
        // Un réglage explicite prime sur le libellé `plateType`.
        assert_eq!(
            plates[1].settings.0.get("curr_bed_type"),
            Some(&string("Textured PEI Plate"))
        );
    }

    #[test]
    fn scene_reads_object_overrides_and_layer_profile() {
        let scene = json!({
            "objects":[{
                "id":"a", "modelId":"ma", "settings":{"wall_loops":5},
                "layerProfile":[0.0, 0.2, 10.0, 0.12]
            }]
        });
        let specs = scene_object_specs(&scene, None).unwrap();
        assert_eq!(specs.len(), 1);
        assert_eq!(
            specs[0].settings.0.get("wall_loops"),
            Some(&engine::api::ConfigValue::Int(5))
        );
        assert_eq!(specs[0].layer_height_profile, vec![0.0, 0.2, 10.0, 0.12]);
    }
}
