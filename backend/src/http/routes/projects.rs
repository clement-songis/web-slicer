//! Endpoints projets (`/api/projects/*`). Toute ressource est scopée par le
//! compte connecté (`CurrentUser`) : un projet d'autrui répond **404** (SC-008,
//! jamais 403). Handlers minces : validation DTO → repo → réponse.
//!
//! La scène et les presets actifs sont des documents JSON versionnés
//! (`schema_version`) ; la sauvegarde utilise un verrou optimiste (409 en cas
//! de conflit multi-onglets).

use axum::extract::{Multipart, Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use glam::{DMat4, DVec3, EulerRot};
use serde_json::{json, Map, Value};
use uuid::Uuid;

use crate::domain::{
    GcodeId, NewProject, Preset, PresetId, PresetKind, PresetOrigin, ProjectId, UserId,
};
use crate::http::dto::{
    CreateProjectRequest, PatchProjectRequest, ProjectResponse, RenameProjectRequest,
    SaveProjectRequest,
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
    let project = state
        .storage
        .projects()
        .create(
            user.id,
            NewProject {
                name: name.to_string(),
                scene: body.scene.unwrap_or_else(default_scene),
                active_presets: body.active_presets.unwrap_or_else(|| json!({})),
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
pub async fn import(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    multipart: Multipart,
) -> ApiResult<(StatusCode, Json<ProjectResponse>)> {
    // Validation avant toute création : aucun projet orphelin sur fichier
    // invalide. L'extraction ZIP/XML est CPU/IO bornée et hors worker async.
    let (filename, format, bytes) = super::models::read_model_upload(&state, multipart).await?;
    let name = project_name_from(&filename);

    if format != crate::domain::ModelFormat::ThreeMf {
        let project = state
            .storage
            .projects()
            .create(
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

    let imported = crate::models::threemf::read_project(&bytes)
        .await
        .map_err(|e| ApiError::validation(e.to_string(), details(&filename)))?;

    let project = state
        .storage
        .projects()
        .create(
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
    // l'ouverture d'un projet dans Orca. Un échec n'empêche pas l'import.
    let aux_dir = state.files.auxiliaries_dir(user.id, project.id);
    if let Err(error) = crate::engine::project_3mf::read_project_auxiliaries(&bytes, &aux_dir).await
    {
        tracing::warn!(%error, "fichiers auxiliaires du 3MF ignorés");
    }

    for (model, mesh) in models {
        let converter = state.converter.clone();
        tokio::spawn(async move {
            converter.convert_mesh(model, mesh).await;
        });
    }
    Ok((StatusCode::CREATED, Json(project.into())))
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
    let (filename, format, bytes) = super::models::read_model_upload(&state, multipart).await?;
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
async fn source_3mf(name: &str, mesh: &engine::api::TriangleMesh) -> ApiResult<Vec<u8>> {
    let mut model = engine::api::Model::default();
    model.add_object(name.to_string(), mesh.clone());
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
    // couleurs des filaments, matrice et multiplicateur de purge.
    let project_config = imported_project_config(&imported.config);
    if !imported.config.is_empty() {
        // Presets filament / imprimante du projet (lot 25, audit J2-a/J2-h),
        // avant le process qui consomme la configuration.
        let (printer, filaments) =
            import_project_presets(state, user_id, project_name, &imported.config).await?;
        let preset = create_imported_preset(state, user_id, project_name, imported.config).await?;
        active_presets = json!({ "process": preset.id.to_string() });
        if let Some(printer) = printer {
            active_presets["printer"] = json!(printer.to_string());
        }
        if !filaments.is_empty() {
            active_presets["filaments"] = json!(filaments
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>());
        }
    }

    let mut models: Vec<(crate::domain::Model, engine::api::TriangleMesh)> =
        Vec::with_capacity(imported.objects.len());
    let mut scene_objects = Vec::with_capacity(imported.objects.len());
    let mut object_ids = Vec::with_capacity(imported.objects.len());
    for object in &imported.objects {
        let source_bytes = source_3mf(&object.name, &object.mesh).await?;
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
            let part_source = source_3mf(&part.name, &part.mesh).await?;
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
        plates.push(json!({
            "id": Uuid::new_v4().to_string(),
            "name": plate.name,
            "plateType": plate.plate_type,
            "objectIds": ids,
        }));
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
        // Valeurs du filament `index` (load_config_file_config : scalaires
        // copiés, vecteurs → élément i ; vecteurs par variante d'extrudeur
        // → éléments dont filament_self_index vaut i + 1).
        let mut values = Map::new();
        for key in ORCA_FILAMENT_OPTIONS {
            // Clés propres au preset : jamais reprises des valeurs brutes du
            // projet (Orca les tire des vecteurs *_group, ci-dessous).
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
                Value::Array(items) if items.len() == count => {
                    Value::Array(vec![items[index].clone()])
                }
                Value::Array(items) if self_index.len() == items.len() && items.len() > count => {
                    Value::Array(
                        items
                            .iter()
                            .zip(&self_index)
                            .filter(|(_, owner)| **owner == index + 1)
                            .map(|(item, _)| item.clone())
                            .collect(),
                    )
                }
                other => other.clone(),
            };
            values.insert(key.to_string(), picked);
        }
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
            body.active_presets,
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
                body.active_presets.unwrap_or(project.active_presets),
                None,
            )
            .await?;
        spawn_scene_thumbnail(&state, user.id, &project);
    }
    if let Some(name) = name {
        project = state.storage.projects().rename(user.id, id, &name).await?;
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
    let copy = state
        .storage
        .projects()
        .create(
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
    let project = state.storage.projects().rename(user.id, id, name).await?;
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
