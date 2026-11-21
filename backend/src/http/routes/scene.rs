//! Outils moteur de scène (`http-api.md` : arrange / orient / repair /
//! simplify, T054/T055).
//!
//! Arrangement, réparation et simplification passent exclusivement par les sous-commandes du
//! process `engine-worker` : aucun appel FFI/C++ n'est lié dans ce process
//! backend. La scène persistée fourni les transformations et les plateaux ; le
//! preset machine fournit le contour et les exclusions sauf surcharge explicite.

use std::path::Path as FsPath;

use axum::extract::{Path, State};
use axum::Json;
use engine::api::{ArrangeParams, BuildVolume, ConfigValue, Model as EngineModel, TriangleMesh};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use crate::domain::{Model, ModelFormat, ModelId, ProjectId, UserId};
use crate::engine::scene::{
    ArrangeWorkerRequest, RepairWorkerOutput, SceneWorkerError, SimplifyWorkerOutput,
};
use crate::http::dto::{
    ArrangeRequest, ArrangeResponse, OrientRequest, OrientResponse, Placement, RepairResponse,
};
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;
use crate::mesh::{parse_stl, Mesh};
use crate::scene::{analyze_mesh, suggest_orientation};

fn parse_project_id(raw: &str) -> ApiResult<ProjectId> {
    uuid::Uuid::parse_str(raw)
        .map(ProjectId)
        .map_err(|_| ApiError::not_found("Projet"))
}

fn parse_model_id(raw: &str) -> ApiResult<ModelId> {
    uuid::Uuid::parse_str(raw)
        .map(ModelId)
        .map_err(|_| ApiError::not_found("Modèle"))
}

/// `POST /api/projects/{id}/arrange` — nesting libslic3r dans un worker isolé.
/// Les ids du corps sont les ids d'objets de la scène. Une empreinte seule sans
/// objet persisté reste acceptée (compatibilité API) sous forme de boîte.
pub async fn arrange(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(project_raw): Path<String>,
    Json(req): Json<ArrangeRequest>,
) -> ApiResult<Json<ArrangeResponse>> {
    let project_id = parse_project_id(&project_raw)?;
    let project = state.storage.projects().get(user.id, project_id).await?;
    validate_arrange_request(&req)?;
    if req.options.is_none() && req.items.is_empty() {
        return Ok(Json(ArrangeResponse {
            placements: Vec::new(),
            plate_count: None,
        }));
    }

    let bed = build_volume(&state, user.id, &project.active_presets, &req).await?;
    // Arrangement séquentiel (Orca : ArrangeJob / CLI need_arrange) : la config
    // résolue pilote l'arrangement libslic3r (dégagements, brim, print_sequence).
    let (mut active_config, _) =
        super::slice::resolve_active_config(&state, user.id, &project.active_presets).await?;
    for (key, value) in [
        ("extruder_clearance_radius", req.extruder_clearance_radius),
        (
            "extruder_clearance_height_to_rod",
            req.extruder_clearance_height_to_rod,
        ),
        (
            "extruder_clearance_height_to_lid",
            req.extruder_clearance_height_to_lid,
        ),
        ("nozzle_height", req.nozzle_height),
    ] {
        if let Some(v) = value.filter(|v| v.is_finite() && *v >= 0.0) {
            active_config
                .0
                .insert(key.to_string(), ConfigValue::Float(v));
        }
    }
    let by_object = matches!(
        active_config.get("print_sequence"),
        Some(ConfigValue::String(s)) if s == "by object"
    );
    let sequential = req.is_seq_print.unwrap_or(by_object);
    let models = state
        .storage
        .models()
        .list(user.id, Some(project_id))
        .await?;
    let scene_objects = project
        .scene
        .get("objects")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let plate_by_object = plate_indices(&project.scene);
    let mut engine_model = EngineModel::default();
    // Arrangement GUI (ArrangeJob::prepare_all / prepare_partplate) : toutes
    // les instances de la scène, les items absents de la scène restant des
    // empreintes (compatibilité).
    let mut items = req.items.clone();
    if req.options.is_some() {
        for object in &scene_objects {
            let Some(id) = object.get("id").and_then(Value::as_str) else {
                continue;
            };
            if object.get("modelId").and_then(Value::as_str).is_some()
                && !items.iter().any(|item| item.id == id)
            {
                items.push(crate::http::dto::ArrangeItem {
                    id: id.to_string(),
                    width: 0.0,
                    depth: 0.0,
                });
            }
        }
    }
    let mut instance_ids = Vec::with_capacity(items.len());

    for item in &items {
        let scene_object = scene_objects
            .iter()
            .find(|object| object.get("id").and_then(Value::as_str) == Some(item.id.as_str()));
        let (mesh, matrix) = if let Some(object) = scene_object {
            let model_id = object
                .get("modelId")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    ApiError::validation(
                        "objet de scène sans modèle",
                        serde_json::json!({ "id": item.id }),
                    )
                })?;
            let source = models
                .iter()
                .find(|model| model.id.to_string() == model_id)
                .ok_or_else(|| ApiError::not_found("Modèle"))?;
            let mut mesh = load_converted_mesh(&state, source).await?;
            center_mesh(&mut mesh);
            (
                mesh,
                object
                    .get("transform")
                    .map_or(glam::DMat4::IDENTITY, scene_transform_matrix),
            )
        } else {
            (box_mesh(item.width, item.depth), glam::DMat4::IDENTITY)
        };
        let object = engine_model.add_object(item.id.clone(), mesh);
        object.instances[0].matrix = matrix;
        object.instances[0].plate_index =
            plate_by_object.get(item.id.as_str()).copied().unwrap_or(0);
        if let Some(scene_object) = scene_object {
            // ModelInstance::printable (instances non imprimables : dernier
            // plateau virtuel d'ArrangeJob::finalize) et extrudeur de l'objet
            // (ArrangePolygon::extrude_ids, multi-matériau par plateau).
            object.instances[0].printable = scene_object
                .get("printable")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            if let Some(extruder) = scene_object
                .get("extruder")
                .and_then(Value::as_u64)
                .filter(|e| *e > 0)
            {
                object.volumes[0].extruder = u16::try_from(extruder).ok();
            }
            object.settings = scene_object_settings(scene_object);
            object.layer_height_profile = scene_object
                .get("layerProfile")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_f64)
                .collect();
        }
        instance_ids.push(item.id.clone());
    }

    if let Some(options) = req.options.as_ref() {
        let plates = scene_plates(&project.scene);
        let current_plate = req
            .plate_index
            .or_else(|| active_plate_index(&project.scene))
            .unwrap_or(0);
        let plate_count = u32::try_from(plates.len()).unwrap_or(u32::MAX).max(1);
        let scope = req.scope.unwrap_or_default();
        if scope == crate::http::dto::ArrangeScope::Plate && current_plate >= plate_count {
            return Err(ApiError::validation(
                "plateau inexistant",
                serde_json::json!({ "plate_index": current_plate }),
            ));
        }
        let global_seq = global_print_sequence(&active_config);
        let real_seq = |plate: Option<&Value>| -> String {
            plate
                .and_then(|p| p.get("settings"))
                .and_then(|s| s.get("print_sequence"))
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty() && *s != "by default")
                .map_or_else(|| global_seq.clone(), str::to_string)
        };
        // prepare_all : plateaux verrouillés + plateaux dont la séquence
        // d'impression diffère de la séquence globale (m_uncompatible_plates).
        let locked_plates: Vec<u32> = plates
            .iter()
            .enumerate()
            .filter(|(_, plate)| {
                plate.get("locked").and_then(Value::as_bool) == Some(true)
                    || real_seq(Some(plate)) != global_seq
            })
            .map(|(index, _)| index as u32)
            .collect();
        let (is_seq_print, plate_seq_differs) = match scope {
            // init_arrange_params, PREPARE_STATE_MENU : séquence réelle du plateau.
            crate::http::dto::ArrangeScope::Plate => {
                let seq = real_seq(plates.get(current_plate as usize).copied());
                (seq == "by object", seq != global_seq)
            }
            crate::http::dto::ArrangeScope::All => (req.is_seq_print.unwrap_or(by_object), false),
        };
        if scope == crate::http::dto::ArrangeScope::Plate
            && plates
                .get(current_plate as usize)
                .and_then(|p| p.get("locked"))
                .and_then(Value::as_bool)
                == Some(true)
        {
            return Err(ApiError::validation(
                "plateau verrouillé : arrangement impossible",
                serde_json::json!({ "plate_index": current_plate }),
            ));
        }
        let is_bbl_vendor = active_printer_is_bbl(&state, user.id, &project.active_presets).await?;
        let settings = engine::api::GuiArrangeSettings {
            distance: options.spacing,
            enable_rotation: options.enable_rotation,
            allow_multi_materials_on_same_plate: options.allow_multi_materials_on_same_plate,
            avoid_extrusion_cali_region: options.avoid_extrusion_cali_region,
            // GLCanvas3D::_render_arrange_menu : alignement Y désactivé quand
            // la rotation automatique est active.
            align_to_y_axis: options.align_to_y_axis && !options.enable_rotation,
            is_seq_print,
            is_bbl_vendor,
            plate_scope: (scope == crate::http::dto::ArrangeScope::Plate).then_some(current_plate),
            current_plate,
            plate_count,
            plate_seq_differs,
            locked_plates,
        };
        let result =
            crate::engine::scene::arrange_gui(&crate::engine::scene::GuiArrangeWorkerRequest {
                model: engine_model,
                bed,
                config: active_config,
                settings,
                instance_ids,
            })
            .await
            .map_err(scene_worker_api_error)?;
        let placements = result
            .placements
            .into_iter()
            .map(|placement| Placement {
                id: placement.id,
                x: placement.matrix[12],
                y: placement.matrix[13],
                plate_index: Some(placement.plate_index),
                matrix: Some(placement.matrix.to_vec()),
                print_order: placement.print_order,
                unarranged: placement.unarranged,
            })
            .collect();
        return Ok(Json(ArrangeResponse {
            placements,
            plate_count: Some(result.plate_count),
        }));
    }

    if let Some(fill_id) = req.fill_object_id.as_deref() {
        if !instance_ids.iter().any(|id| id == fill_id) {
            return Err(ApiError::validation(
                "objet à dupliquer absent des items",
                serde_json::json!({ "fill_object_id": fill_id }),
            ));
        }
        // Plateau courant seulement (FillBedJob : plaque de l'objet sélectionné).
        let plate = plate_by_object.get(fill_id).copied().unwrap_or(0);
        let mut plate_model = EngineModel::default();
        for object in engine_model.objects {
            if object.instances.first().map(|i| i.plate_index) == Some(plate) {
                plate_model.objects.push(object);
            }
        }
        let object_index = plate_model
            .objects
            .iter()
            .position(|o| o.name == fill_id)
            .expect("objet cible conservé");
        let filled = crate::engine::scene::fill_bed(&crate::engine::scene::FillBedWorkerRequest {
            model: plate_model,
            bed,
            config: active_config,
            object_index,
            clearance: if req.spacing.is_finite() && req.spacing > 0.0 {
                req.spacing
            } else {
                0.0
            },
        })
        .await
        .map_err(scene_worker_api_error)?;
        let plate_index = plate_by_object.get(fill_id).copied().unwrap_or(0);
        let placements = filled
            .instances
            .into_iter()
            .enumerate()
            .map(|(k, matrix)| Placement {
                id: if k == 0 {
                    fill_id.to_string()
                } else {
                    format!("{fill_id}#fill{k}")
                },
                x: matrix[12],
                y: matrix[13],
                plate_index: Some(plate_index),
                matrix: Some(matrix.to_vec()),
                print_order: None,
                unarranged: false,
            })
            .collect();
        return Ok(Json(ArrangeResponse {
            placements,
            plate_count: None,
        }));
    }

    let result = crate::engine::scene::arrange(&ArrangeWorkerRequest {
        model: engine_model,
        bed,
        params: ArrangeParams {
            clearance: if req.spacing.is_finite() && req.spacing >= 0.0 {
                req.spacing
            } else {
                0.0
            },
            // Conservé dans le protocole moteur, mais non pris en charge par le
            // bridge `arrange_raw` actuel (voir limites documentées).
            use_machine_clearance: false,
            // L'outil UI conserve les orientations choisies par l'utilisateur ;
            // seul le harness de parité CLI active les rotations du nester.
            allow_rotations: false,
            orca_cli_mode: false,
            print_config: sequential.then_some(active_config),
            is_seq_print: sequential,
        },
        instance_ids,
    })
    .await
    .map_err(scene_worker_api_error)?;

    let placements = result
        .placements
        .into_iter()
        .map(|placement| Placement {
            id: placement.id,
            x: placement.matrix[12],
            y: placement.matrix[13],
            plate_index: Some(placement.plate_index),
            matrix: Some(placement.matrix.to_vec()),
            print_order: placement.print_order,
            unarranged: false,
        })
        .collect();
    Ok(Json(ArrangeResponse {
        placements,
        plate_count: None,
    }))
}

/// `GET /api/projects/{id}/arrange/options` — valeurs du bouton « Reset » du
/// menu « Options d'arrangement » d'Orca et disponibilité des cases pour
/// l'imprimante active (GLCanvas3D::_render_arrange_menu).
pub async fn arrange_options(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(project_raw): Path<String>,
) -> ApiResult<Json<crate::http::dto::ArrangeOptionsInfo>> {
    let project_id = parse_project_id(&project_raw)?;
    let project = state.storage.projects().get(user.id, project_id).await?;
    let (config, _) =
        super::slice::resolve_active_config(&state, user.id, &project.active_presets).await?;
    let is_bbl = active_printer_is_bbl(&state, user.id, &project.active_presets).await?;
    Ok(Json(arrange_options_info(&config, is_bbl)))
}

/// Réinitialisation des ArrangeSettings (bouton « Reset ») + disponibilité.
pub(crate) fn arrange_options_info(
    config: &engine::api::DynamicPrintConfig,
    is_bbl_vendor: bool,
) -> crate::http::dto::ArrangeOptionsInfo {
    let is_seq_print = global_print_sequence(config) == "by object";
    // Reset : align_to_y_axis = (printer_structure == i3).
    let align_to_y_axis = matches!(
        config.get("printer_structure"),
        Some(ConfigValue::String(s)) if s == "i3"
    );
    let scan_first_layer = matches!(
        config.get("scan_first_layer"),
        Some(ConfigValue::Bool(true))
    );
    crate::http::dto::ArrangeOptionsInfo {
        defaults: crate::http::dto::ArrangeOptions {
            spacing: 0.0,
            enable_rotation: false,
            allow_multi_materials_on_same_plate: true,
            avoid_extrusion_cali_region: true,
            align_to_y_axis,
        },
        is_seq_print,
        min_spacing: min_object_distance(config),
        show_avoid_extrusion_cali_region: is_bbl_vendor && scan_first_layer,
    }
}

/// `min_object_distance` (PrintConfig.cpp) : max(6, extruder_clearance_radius)
/// en séquentiel, 6 sinon (FFF).
fn min_object_distance(config: &engine::api::DynamicPrintConfig) -> f64 {
    const DUPLICATE_DISTANCE: f64 = 6.0;
    let ecr = match config.get("extruder_clearance_radius") {
        Some(ConfigValue::Float(v)) => Some(*v),
        Some(ConfigValue::Int(v)) => Some(*v as f64),
        _ => None,
    };
    match (ecr, config.get("print_sequence")) {
        (Some(ecr), Some(ConfigValue::String(seq))) => {
            if seq == "by object" && ecr > DUPLICATE_DISTANCE {
                ecr
            } else {
                DUPLICATE_DISTANCE
            }
        }
        _ => 0.0,
    }
}

/// `print_sequence` global du jeu de réglages résolu (défaut Orca : by layer).
fn global_print_sequence(config: &engine::api::DynamicPrintConfig) -> String {
    match config.get("print_sequence") {
        Some(ConfigValue::String(s)) if !s.is_empty() => s.clone(),
        _ => "by layer".to_string(),
    }
}

/// Plateaux de la scène (document `{plates, activeId}` ou tableau historique).
fn scene_plates(scene: &Value) -> Vec<&Value> {
    match scene.get("plates") {
        Some(Value::Array(plates)) => plates.iter().collect(),
        Some(Value::Object(document)) => document
            .get("plates")
            .and_then(Value::as_array)
            .map(|p| p.iter().collect())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Index du plateau actif (`plates.activeId`).
fn active_plate_index(scene: &Value) -> Option<u32> {
    let active = scene.get("plates")?.get("activeId")?.as_str()?;
    scene_plates(scene)
        .iter()
        .position(|p| p.get("id").and_then(Value::as_str) == Some(active))
        .map(|i| i as u32)
}

/// `PresetBundle::is_bbl_vendor` : le preset imprimante actif (ou le preset
/// système dont il hérite) appartient au fabricant « BBL ».
async fn active_printer_is_bbl(
    state: &AppState,
    user: UserId,
    active_presets: &Value,
) -> ApiResult<bool> {
    let Some(id) = active_presets.get("printer").and_then(Value::as_str) else {
        return Ok(false);
    };
    let machines = state
        .storage
        .presets()
        .list_by_kind(crate::domain::PresetKind::Machine, user)
        .await?;
    let Some(leaf) = machines.iter().find(|p| p.id.to_string() == id) else {
        return Ok(false);
    };
    Ok(printer_vendor(&machines, leaf).as_deref() == Some("BBL"))
}

/// Fabricant d'un preset machine : le sien, sinon celui du plus proche
/// ancêtre système de la chaîne d'héritage.
pub(crate) fn printer_vendor(
    visible: &[crate::domain::Preset],
    leaf: &crate::domain::Preset,
) -> Option<String> {
    if let Some(vendor) = leaf.vendor.clone().filter(|v| !v.is_empty()) {
        return Some(vendor);
    }
    super::presets::build_chain(visible, leaf)
        .into_iter()
        .rev()
        .filter_map(|raw| {
            visible
                .iter()
                .find(|p| p.name == raw.name && p.origin == crate::domain::PresetOrigin::System)
        })
        .find_map(|p| p.vendor.clone().filter(|v| !v.is_empty()))
}

/// `POST /api/projects/{id}/orient` — suggère la rotation posant la plus grande
/// facette du modèle à plat (FR-013). Cette heuristique reste pure Rust.
pub async fn orient(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(project_raw): Path<String>,
    Json(req): Json<OrientRequest>,
) -> ApiResult<Json<OrientResponse>> {
    let project_id = parse_project_id(&project_raw)?;
    state.storage.projects().get(user.id, project_id).await?;
    let model = load_project_model(&state, user.id, project_id, &req.model_id).await?;
    let mesh = load_stl_mesh(&state, &model).await?;
    Ok(Json(OrientResponse {
        rotation: suggest_orientation(&mesh).to_vec(),
    }))
}

/// `POST /api/models/{id}/repair` — répare via admesh dans un worker, remplace
/// le maillage WSMh stocké et persiste le rapport complet (état + corrections).
pub async fn repair(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<RepairResponse>> {
    let model_id = parse_model_id(&id)?;
    let model = state.storage.models().get(user.id, model_id).await?;
    let mesh = load_converted_mesh(&state, &model).await?;
    let RepairWorkerOutput {
        mesh,
        display_bytes,
        repairs,
    } = crate::engine::scene::repair(&mesh)
        .await
        .map_err(scene_worker_api_error)?;

    let report = analyze_mesh(&Mesh {
        positions: mesh.vertices.iter().flatten().copied().collect(),
        normals: Vec::new(),
        indices: mesh.indices.iter().flatten().copied().collect(),
    });
    let response = RepairResponse {
        triangles: report.triangles as u32,
        degenerate: report.degenerate as u32,
        open_edges: report.open_edges as u32,
        watertight: report.watertight(),
        repairs: repairs.into(),
    };
    let report_json = serde_json::to_value(&response).map_err(|e| {
        tracing::error!(error = %e, "sérialisation du rapport de réparation");
        ApiError::internal()
    })?;
    keep_previous_mesh(&model).await;
    let mesh_path = state
        .files
        .write_mesh(user.id, model.id, &display_bytes)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "écriture du maillage réparé");
            ApiError::internal()
        })?;
    state
        .storage
        .models()
        .set_repair_result(
            user.id,
            model.id,
            &mesh_path.to_string_lossy(),
            i64::from(report.triangles as u32),
            report_json,
        )
        .await?;
    state.events.publish_model_converted(
        user.id,
        &model.id.to_string(),
        &format!("/api/models/{}/mesh", model.id),
    );
    Ok(Json(response))
}

/// Corps de `POST /api/models/{id}/simplify` : les deux modes de l'outil
/// « Simplifier » d'Orca. `ratio` = pourcentage de triangles à retirer dans
/// ]0, 100[ ; `detail` = niveau 0 (« Extra high ») … 4 (« Extra low »).
#[derive(Debug, Clone, Copy, Deserialize, TS)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
#[ts(export)]
pub enum SimplifyRequest {
    Ratio { decimate_ratio: f32 },
    Detail { level: u8 },
}

impl From<SimplifyRequest> for engine::api::mesh::SimplifyParams {
    fn from(req: SimplifyRequest) -> Self {
        match req {
            SimplifyRequest::Ratio { decimate_ratio } => Self::Ratio { decimate_ratio },
            SimplifyRequest::Detail { level } => Self::Detail { level },
        }
    }
}

/// Réponse de `POST /api/models/{id}/simplify` : comptes d'entrée/sortie,
/// erreur quadrique retenue et état du maillage **après** simplification.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct SimplifyResponse {
    pub triangles_before: u32,
    pub triangles_after: u32,
    pub vertices_before: u32,
    pub vertices_after: u32,
    pub max_error: f32,
    pub open_edges: u32,
    pub watertight: bool,
}

/// `POST /api/models/{id}/simplify` — simplification quadrique libslic3r dans
/// un worker isolé ; remplace le maillage WSMh stocké du modèle (le fichier
/// source reste inchangé). Paramètre invalide → 422 sans lancer le worker.
pub async fn simplify(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<SimplifyRequest>,
) -> ApiResult<Json<SimplifyResponse>> {
    let model_id = parse_model_id(&id)?;
    let model = state.storage.models().get(user.id, model_id).await?;
    let params = engine::api::mesh::SimplifyParams::from(req);
    // Domaine du paramètre vérifié avant toute lecture : 422 explicite même si
    // la conversion est en cours. Les contraintes liées au nombre de triangles
    // sont vérifiées ensuite (avant le lancement du worker).
    params
        .validate()
        .map_err(|e| ApiError::validation(e.to_string(), serde_json::json!({})))?;
    let mesh = load_converted_mesh(&state, &model).await?;
    let SimplifyWorkerOutput {
        mesh,
        display_bytes,
        report,
    } = crate::engine::scene::simplify(&mesh, &params)
        .await
        .map_err(scene_worker_api_error)?;

    let analysis = analyze_mesh(&Mesh {
        positions: mesh.vertices.iter().flatten().copied().collect(),
        normals: Vec::new(),
        indices: mesh.indices.iter().flatten().copied().collect(),
    });
    let response = SimplifyResponse {
        triangles_before: report.triangles_before,
        triangles_after: report.triangles_after,
        vertices_before: report.vertices_before,
        vertices_after: report.vertices_after,
        max_error: report.max_error,
        open_edges: analysis.open_edges as u32,
        watertight: analysis.watertight(),
    };
    keep_previous_mesh(&model).await;
    let mesh_path = state
        .files
        .write_mesh(user.id, model.id, &display_bytes)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "écriture du maillage simplifié");
            ApiError::internal()
        })?;
    state
        .storage
        .models()
        .set_mesh(
            user.id,
            model.id,
            &mesh_path.to_string_lossy(),
            i64::from(report.triangles_after),
        )
        .await?;
    state.events.publish_model_converted(
        user.id,
        &model.id.to_string(),
        &format!("/api/models/{}/mesh", model.id),
    );
    Ok(Json(response))
}

fn validate_arrange_request(req: &ArrangeRequest) -> ApiResult<()> {
    if !(req.bed_width.is_finite()
        && req.bed_depth.is_finite()
        && req.bed_width > 0.0
        && req.bed_depth > 0.0)
    {
        return Err(ApiError::validation(
            "dimensions de plateau invalides",
            serde_json::json!({ "bed_width": req.bed_width, "bed_depth": req.bed_depth }),
        ));
    }
    if req.items.iter().any(|item| {
        !item.width.is_finite() || !item.depth.is_finite() || item.width < 0.0 || item.depth < 0.0
    }) {
        return Err(ApiError::validation(
            "empreinte d'objet invalide",
            serde_json::json!({}),
        ));
    }
    if let Some(options) = &req.options {
        // Orca : espacement ≥ 0 (« 0 means auto spacing »).
        if !options.spacing.is_finite() || options.spacing < 0.0 || options.spacing > 1000.0 {
            return Err(ApiError::validation(
                "espacement d'arrangement invalide",
                serde_json::json!({ "spacing": options.spacing }),
            ));
        }
        if req.fill_object_id.is_some() {
            return Err(ApiError::validation(
                "options d'arrangement incompatibles avec le remplissage",
                serde_json::json!({}),
            ));
        }
    }
    let unique = req
        .items
        .iter()
        .map(|item| item.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    if unique.len() != req.items.len() {
        return Err(ApiError::validation(
            "identifiants d'objets dupliqués",
            serde_json::json!({}),
        ));
    }
    Ok(())
}

async fn build_volume(
    state: &AppState,
    user: UserId,
    active_presets: &Value,
    req: &ArrangeRequest,
) -> ApiResult<BuildVolume> {
    let (config, _) = super::slice::resolve_active_config(state, user, active_presets).await?;
    let config_polygon = |key| -> Option<Vec<[f64; 2]>> {
        match config.get(key) {
            Some(ConfigValue::Points(points)) => Some(
                points
                    .iter()
                    .filter(|point| point.len() == 2)
                    .map(|point| [point[0], point[1]])
                    .collect(),
            ),
            _ => None,
        }
    };
    let bed_shape = req
        .bed_shape
        .clone()
        .or_else(|| config_polygon("printable_area"))
        .filter(|polygon| polygon.len() >= 3)
        .unwrap_or_else(|| {
            vec![
                [0.0, 0.0],
                [req.bed_width, 0.0],
                [req.bed_width, req.bed_depth],
                [0.0, req.bed_depth],
            ]
        });
    let excluded = req
        .excluded
        .clone()
        .or_else(|| config_polygon("bed_exclude_area").map(|polygon| vec![polygon]))
        .unwrap_or_default()
        .into_iter()
        // Orca : `bed_exclude_area = 0x0` (un seul point) signifie « aucune zone ».
        .filter(|polygon| polygon.len() >= 3)
        .collect();
    let max_height = req.max_height.or(match config.get("printable_height") {
        Some(ConfigValue::Float(value)) => Some(*value),
        _ => None,
    });
    Ok(BuildVolume {
        bed_shape,
        max_height: max_height.unwrap_or(200.0),
        excluded,
    })
}

fn plate_indices(scene: &Value) -> std::collections::HashMap<&str, u32> {
    let plates = match scene.get("plates") {
        Some(Value::Array(plates)) => Some(plates),
        Some(Value::Object(document)) => document.get("plates").and_then(Value::as_array),
        _ => None,
    };
    let mut result = std::collections::HashMap::new();
    for (index, plate) in plates.into_iter().flatten().enumerate() {
        for id in plate
            .get("objectIds")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            result.insert(id, index as u32);
        }
    }
    result
}

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

/// Même convention Transform Three.js que le runner de slicing : Euler XYZ en
/// degrés, scale par composante et translation en mm. Matrice d'instance
/// T·R·S (Geometry::Transformation d'Orca) : la colonne j est la colonne j de
/// la rotation multipliée par l'échelle s_j (Matrix4.compose de Three.js).
pub(crate) fn scene_transform_matrix(value: &Value) -> glam::DMat4 {
    use glam::{DMat4, DVec3};
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
    let rotation = crate::scene::euler_xyz_deg_rotation(vector("rotation", [0.0; 3]));
    let scale = vector("scale", [1.0; 3]);
    let column = |j: usize| DVec3::new(rotation[0][j], rotation[1][j], rotation[2][j]) * scale[j];
    DMat4::from_cols(
        column(0).extend(0.0),
        column(1).extend(0.0),
        column(2).extend(0.0),
        DVec3::new(x, y, z).extend(1.0),
    )
}

fn center_mesh(mesh: &mut TriangleMesh) {
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

/// Boîte de compatibilité pour les anciens appels qui ne référencent pas encore
/// un objet de scène persisté. Le nesting reste réel ; seule la géométrie est
/// synthétique et sa hauteur (1 mm) n'influence pas l'arrangement XY.
fn box_mesh(width: f64, depth: f64) -> TriangleMesh {
    let (x, y) = ((width / 2.0) as f32, (depth / 2.0) as f32);
    TriangleMesh {
        vertices: vec![
            [-x, -y, 0.0],
            [x, -y, 0.0],
            [x, y, 0.0],
            [-x, y, 0.0],
            [-x, -y, 1.0],
            [x, -y, 1.0],
            [x, y, 1.0],
            [-x, y, 1.0],
        ],
        indices: vec![
            [0, 2, 1],
            [0, 3, 2],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ],
    }
}

/// Suffixe du maillage précédent conservé pour « garder la peinture ».
const PREVIOUS_MESH_SUFFIX: &str = ".prev";

/// Conserve le maillage courant (avant simplification / réparation) à côté du
/// nouveau, comme ModelVolume::save_painting garde le maillage source pour
/// TriangleSelector::remap_painting. Échec silencieux : seul le remappage
/// ultérieur sera indisponible.
async fn keep_previous_mesh(model: &Model) {
    if let Some(path) = model.mesh_path.as_deref() {
        let prev = format!("{path}{PREVIOUS_MESH_SUFFIX}");
        if let Err(e) = tokio::fs::copy(path, &prev).await {
            tracing::warn!(error = %e, "copie du maillage précédent");
        }
    }
}

/// Corps de `POST /api/models/{id}/remap-paint` : peinture (chaînes 3MF par
/// triangle) de l'ancien maillage, par canal `supports` | `seam` | `mmu` | `fuzzy`.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct RemapPaintHttpRequest {
    #[ts(type = "Record<string, Record<number, string>>")]
    pub channels: std::collections::BTreeMap<String, engine::api::paint::PaintFacets>,
}

/// Réponse : mêmes canaux sur les triangles du maillage courant.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct RemapPaintHttpResponse {
    #[ts(type = "Record<string, Record<number, string>>")]
    pub channels: std::collections::BTreeMap<String, engine::api::paint::PaintFacets>,
}

/// `POST /api/models/{id}/remap-paint` — « Garder la peinture » d'Orca
/// (préférence keep_painting : Simplify / Fix model / Cut) : la peinture
/// posée sur le maillage d'avant la dernière simplification ou réparation est
/// reportée sur le maillage courant (TriangleSelector::remap_painting, dans
/// engine-worker). 409 si aucun maillage précédent n'est conservé.
pub async fn remap_paint(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<RemapPaintHttpRequest>,
) -> ApiResult<Json<RemapPaintHttpResponse>> {
    let model_id = parse_model_id(&id)?;
    let model = state.storage.models().get(user.id, model_id).await?;
    if let Some(bad) = req
        .channels
        .keys()
        .find(|k| !engine::api::paint::PAINT_CHANNEL_KEYS.contains(&k.as_str()))
    {
        return Err(ApiError::validation(
            "canal de peinture inconnu (supports, seam, mmu, fuzzy)",
            serde_json::json!({ "channel": bad }),
        ));
    }
    let target = load_converted_mesh(&state, &model).await?;
    let prev_path = model
        .mesh_path
        .as_deref()
        .map(|p| format!("{p}{PREVIOUS_MESH_SUFFIX}"))
        .ok_or_else(|| {
            ApiError::conflict("aucun maillage précédent (simplifier ou réparer d'abord)")
        })?;
    let bytes = tokio::fs::read(&prev_path).await.map_err(|_| {
        ApiError::conflict("aucun maillage précédent (simplifier ou réparer d'abord)")
    })?;
    let source = decode_display_mesh(&bytes)?;
    let channels = remap_paint_via_worker(source, target, req.channels).await?;
    Ok(Json(RemapPaintHttpResponse { channels }))
}

/// Remappage de peinture (« Garder la peinture ») dans engine-worker.
pub(crate) async fn remap_paint_via_worker(
    source: TriangleMesh,
    target: TriangleMesh,
    channels: std::collections::BTreeMap<String, engine::api::paint::PaintFacets>,
) -> ApiResult<std::collections::BTreeMap<String, engine::api::paint::PaintFacets>> {
    let dir = crate::workdir::tempdir().map_err(|e| {
        tracing::error!(error = %e, "répertoire de travail");
        ApiError::internal()
    })?;
    let request_path = dir.path().join("remap-paint.json");
    let request = engine::api::paint::RemapPaintRequest {
        source,
        target,
        channels,
    };
    std::fs::write(
        &request_path,
        serde_json::to_vec(&request).map_err(|_| ApiError::internal())?,
    )
    .map_err(|_| ApiError::internal())?;
    let stdout = crate::engine::run_worker(
        &["remap-paint", &request_path.to_string_lossy()],
        crate::engine::DEFAULT_TIMEOUT,
    )
    .await
    .map_err(|e| match e {
        crate::engine::WorkerError::Failed { stdout, .. } => ApiError::validation(
            String::from_utf8_lossy(&stdout)
                .lines()
                .find_map(|l| l.strip_prefix("E "))
                .unwrap_or("remappage de peinture refusé")
                .to_string(),
            serde_json::json!({}),
        ),
        other => {
            tracing::error!(error = %other, "worker remap-paint");
            ApiError::internal()
        }
    })?;
    let text = String::from_utf8_lossy(&stdout);
    let payload = text
        .lines()
        .rev()
        .find_map(|l| l.strip_prefix("R "))
        .ok_or_else(ApiError::internal)?;
    let response: engine::api::paint::RemapPaintResponse =
        serde_json::from_str(payload).map_err(|_| ApiError::internal())?;
    Ok(response.channels)
}

/// WSMh (maillage d'affichage) → TriangleMesh.
fn decode_display_mesh(bytes: &[u8]) -> ApiResult<TriangleMesh> {
    let display = engine::api::DisplayMesh::decode(bytes).map_err(|e| {
        tracing::error!(error = %e, "décodage du maillage");
        ApiError::internal()
    })?;
    Ok(TriangleMesh {
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
    })
}

async fn load_converted_mesh(state: &AppState, model: &Model) -> ApiResult<TriangleMesh> {
    let Some(path) = model.mesh_path.as_deref() else {
        return Err(if let Some(error) = model.conversion_error.as_deref() {
            ApiError::conversion_failed_for(model.id, error)
        } else {
            ApiError::conflict("conversion en cours (voir l'événement model.converted)")
        });
    };
    let bytes = state.files.read(FsPath::new(path)).await.map_err(|e| {
        tracing::error!(error = %e, "lecture du maillage converti");
        ApiError::internal()
    })?;
    let display = engine::api::DisplayMesh::decode(&bytes).map_err(|e| {
        tracing::error!(error = %e, "décodage du maillage converti");
        ApiError::internal()
    })?;
    let mesh = TriangleMesh {
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
    };
    if mesh
        .indices
        .iter()
        .flatten()
        .any(|&index| index as usize >= mesh.vertices.len())
    {
        return Err(ApiError::validation(
            "maillage converti invalide",
            serde_json::json!({ "model_id": model.id.to_string() }),
        ));
    }
    Ok(mesh)
}

fn scene_worker_api_error(error: SceneWorkerError) -> ApiError {
    if error.is_invalid_input() {
        return ApiError::validation(error.to_string(), serde_json::json!({}));
    }
    tracing::error!(error = %error, "opération de scène dans engine-worker");
    ApiError::internal()
}

/// Charge un modèle en vérifiant qu'il appartient au compte **et** au projet.
async fn load_project_model(
    state: &AppState,
    user_id: UserId,
    project_id: ProjectId,
    model_raw: &str,
) -> ApiResult<Model> {
    let model_id = parse_model_id(model_raw)?;
    let model = state.storage.models().get(user_id, model_id).await?;
    if model.project_id != Some(project_id) {
        return Err(ApiError::not_found("Modèle"));
    }
    Ok(model)
}

/// Lecture STL conservée uniquement pour l'heuristique d'orientation historique.
async fn load_stl_mesh(state: &AppState, model: &Model) -> ApiResult<Mesh> {
    if model.format != ModelFormat::Stl {
        return Err(match model.format {
            ModelFormat::Step => {
                ApiError::conflict("conversion STEP en cours (voir l'événement model.converted)")
            }
            _ => ApiError::not_implemented("analyse serveur indisponible (aperçu côté client)"),
        });
    }
    let bytes = state
        .files
        .read(FsPath::new(&model.file_path))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "lecture du modèle");
            ApiError::internal()
        })?;
    parse_stl(&bytes).map_err(|e| {
        tracing::error!(error = %e, "décodage STL");
        ApiError::validation("STL illisible", serde_json::json!({}))
    })
}

// --- Erreurs de maillage (GUI_ObjectList::get_mesh_errors_info) -------------

/// Erreurs de maillage d'un modèle ou d'un objet, comme l'icône d'alerte et
/// l'infobulle de la liste d'objets d'Orca (`ObjectList::get_mesh_errors_info`,
/// `get_warning_icon_name`) : arêtes ouvertes restantes (non-manifold) et
/// erreurs corrigées automatiquement à la lecture (`RepairedMeshErrors` :
/// arêtes soudées, facettes dégénérées / retirées / inversées, arêtes à
/// l'envers). Statistiques calculées par libslic3r (`TriangleMesh::stats()`
/// du fichier lu comme Orca, `<mesh_stat>` d'un 3MF) ; après « Réparer »,
/// maillage réparé (aucune erreur réparée, comme après fix_model_with_cgal).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct MeshErrorsInfo {
    /// Arêtes ouvertes restantes (`stats.open_edges`).
    pub open_edges: u32,
    pub edges_fixed: u32,
    pub degenerate_facets: u32,
    pub facets_removed: u32,
    /// Facettes inversées (normales retournées) corrigées.
    pub facets_reversed: u32,
    pub backwards_edges: u32,
    /// `get_repaired_errors_count` : somme des 5 compteurs ci-dessus.
    pub repaired_errors: u32,
    /// `stats.manifold()` : aucune arête ouverte.
    pub manifold: bool,
    /// `stats.repaired()` : au moins une erreur corrigée.
    pub repaired: bool,
    /// Icône d'alerte `obj_warning` affichée (non manifold ou réparé).
    pub warning: bool,
    /// Infobulle d'Orca (msgids anglais, à traduire côté client) ; vide si
    /// aucune alerte.
    pub tooltip: String,
}

impl MeshErrorsInfo {
    pub(crate) fn from_stats(stats: &engine::api::MeshErrorStats) -> Self {
        let r = &stats.repaired;
        let manifold = stats.manifold();
        let repaired = stats.repaired();
        let errors = stats.repaired_errors_count();
        let plural = |n: u32, one: &str, many: &str| {
            if n == 1 {
                one.replace("%1$d", &n.to_string())
            } else {
                many.replace("%1$d", &n.to_string())
            }
        };
        let mut tooltip = String::new();
        if repaired || !manifold {
            if repaired {
                tooltip += &plural(errors, "%1$d error repaired", "%1$d errors repaired");
                tooltip += "\n";
            }
            if !manifold {
                tooltip += "Remaining errors:\n\t";
                tooltip += &plural(
                    stats.open_edges,
                    "%1$d non-manifold edge",
                    "%1$d non-manifold edges",
                );
                tooltip += "\n";
            }
            tooltip += "\nClick the icon to repair model object";
        }
        Self {
            open_edges: stats.open_edges,
            edges_fixed: r.edges_fixed,
            degenerate_facets: r.degenerate_facets,
            facets_removed: r.facets_removed,
            facets_reversed: r.facets_reversed,
            backwards_edges: r.backwards_edges,
            repaired_errors: errors,
            manifold,
            repaired,
            warning: !manifold || repaired,
            tooltip,
        }
    }
}

/// Statistiques d'erreurs d'un modèle (somme de ses volumes,
/// `ModelObject::get_object_stl_stats`).
pub(crate) async fn model_mesh_stats(
    state: &AppState,
    model: &Model,
) -> ApiResult<engine::api::MeshErrorStats> {
    let repaired_by_user = model
        .repair_report
        .as_ref()
        .is_some_and(|report| report.get("repairs").is_some());
    if !repaired_by_user {
        if let Some(raw) =
            crate::engine::raw_model::load_with_stats(FsPath::new(&model.file_path), model.format)
                .await
        {
            let mut total = engine::api::MeshErrorStats::default();
            for stats in raw
                .objects
                .iter()
                .flat_map(|o| &o.volumes)
                .filter_map(|v| v.mesh_stats.as_ref())
            {
                total.merge(stats);
            }
            return Ok(total);
        }
    }
    // Maillage remplacé par « Réparer » (ou format non relu par le moteur) :
    // TriangleMesh(its) neuf → arêtes ouvertes du maillage stocké, rien de réparé.
    let mesh = load_converted_mesh(state, model).await?;
    let report = analyze_mesh(&Mesh {
        positions: mesh.vertices.iter().flatten().copied().collect(),
        normals: Vec::new(),
        indices: mesh.indices.iter().flatten().copied().collect(),
    });
    Ok(engine::api::MeshErrorStats {
        open_edges: report.open_edges as u32,
        repaired: Default::default(),
    })
}

/// `GET /api/models/{id}/mesh-errors` — erreurs de maillage d'un modèle.
pub async fn model_mesh_errors(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<MeshErrorsInfo>> {
    let model = state
        .storage
        .models()
        .get(user.id, parse_model_id(&id)?)
        .await?;
    let stats = model_mesh_stats(&state, &model).await?;
    Ok(Json(MeshErrorsInfo::from_stats(&stats)))
}

/// Erreurs de maillage d'un objet de la scène.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ObjectMeshErrors {
    /// `objects[].id` de la scène.
    pub object_id: String,
    #[serde(flatten)]
    #[ts(flatten)]
    pub info: MeshErrorsInfo,
}

/// `GET /api/projects/{id}/mesh-errors` — alerte d'erreurs de maillage de
/// chaque objet de la scène (liste d'objets d'Orca) : modèle de l'objet +
/// volumes maillés (`shape: "mesh"`), cumulés comme get_object_stl_stats.
/// Objets sans alerte inclus (`warning: false`).
pub async fn project_mesh_errors(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(project_raw): Path<String>,
) -> ApiResult<Json<Vec<ObjectMeshErrors>>> {
    let project_id = parse_project_id(&project_raw)?;
    let project = state.storage.projects().get(user.id, project_id).await?;
    let objects = project
        .scene
        .get("objects")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    objects_mesh_errors(&state, user.id, project_id, &objects).await
}

/// Corps de `POST /api/projects/{id}/mesh-errors` : objets de la scène COURANTE
/// du client (même forme que `scene.objects`), y compris ceux importés et pas
/// encore enregistrés. Seuls `id`, `modelId` et `volumes[].shape/modelId` sont lus.
#[derive(Debug, Clone, Default, Deserialize, TS)]
#[ts(export)]
pub struct MeshErrorsRequest {
    #[serde(default)]
    #[ts(
        type = "Array<{ id: string; modelId?: string; volumes?: Array<{ shape?: string; modelId?: string }> }>"
    )]
    pub objects: Vec<Value>,
}

/// `POST /api/projects/{id}/mesh-errors` — comme le GET, mais sur les objets
/// fournis (scène non enregistrée) : Orca affiche l'icône d'erreurs dès le
/// chargement du modèle (ObjectList::add_object_to_list → get_mesh_errors_info),
/// avant tout enregistrement du projet. Modèles hors du projet ou d'un autre
/// compte ignorés (aucune fuite) ; projet inconnu → 404.
pub async fn project_mesh_errors_for_scene(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(project_raw): Path<String>,
    Json(body): Json<MeshErrorsRequest>,
) -> ApiResult<Json<Vec<ObjectMeshErrors>>> {
    let project_id = parse_project_id(&project_raw)?;
    state.storage.projects().get(user.id, project_id).await?;
    if body.objects.len() > 10_000 {
        return Err(ApiError::validation(
            "trop d'objets",
            serde_json::json!({ "field": "objects" }),
        ));
    }
    objects_mesh_errors(&state, user.id, project_id, &body.objects).await
}

/// Erreurs de maillage par objet de scène (modèle + volumes maillés cumulés,
/// get_object_stl_stats) ; seuls les modèles du projet du compte sont lus.
async fn objects_mesh_errors(
    state: &AppState,
    user_id: UserId,
    project_id: ProjectId,
    objects: &[Value],
) -> ApiResult<Json<Vec<ObjectMeshErrors>>> {
    let models = state
        .storage
        .models()
        .list(user_id, Some(project_id))
        .await?;
    let mut cache: std::collections::HashMap<String, engine::api::MeshErrorStats> =
        std::collections::HashMap::new();
    let mut out = Vec::new();
    for object in objects {
        let (Some(id), Some(model_id)) = (
            object.get("id").and_then(Value::as_str),
            object.get("modelId").and_then(Value::as_str),
        ) else {
            continue;
        };
        let volume_models = object
            .get("volumes")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|v| v.get("shape").and_then(Value::as_str) == Some("mesh"))
            .filter_map(|v| v.get("modelId").and_then(Value::as_str));
        let mut total = engine::api::MeshErrorStats::default();
        for model_id in std::iter::once(model_id).chain(volume_models) {
            let Some(model) = models.iter().find(|m| m.id.to_string() == model_id) else {
                continue;
            };
            if !cache.contains_key(model_id) {
                let stats = model_mesh_stats(state, model).await?;
                cache.insert(model_id.to_string(), stats);
            }
            total.merge(&cache[model_id]);
        }
        out.push(ObjectMeshErrors {
            object_id: id.to_string(),
            info: MeshErrorsInfo::from_stats(&total),
        });
    }
    Ok(Json(out))
}

// --- Informations de coupe (« Invalidate cut info ») -------------------------

/// Corps de `POST /api/projects/{id}/objects/{object_id}/invalidate-cut`.
#[derive(Debug, Clone, Default, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct InvalidateCutRequest {
    /// Verrou optimiste (version courante si absent).
    #[serde(default)]
    #[ts(optional)]
    pub expected_version: Option<i64>,
}

/// Réponse : objets dont les informations de coupe ont été retirées et projet
/// sauvegardé (nouvelle version).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct InvalidateCutResponse {
    pub object_ids: Vec<String>,
    pub project: crate::http::dto::ProjectResponse,
}

/// `POST /api/projects/{id}/objects/{object_id}/invalidate-cut` — « Invalidate
/// cut info » du menu contextuel d'Orca (MenuFactory::append_menu_item_
/// invalidate_cut_info, ObjectList::invalidate_cut_info_for_object) : retire
/// `cutId` de tous les objets liés (même cut_id) et `cutInfo` de leurs
/// volumes ; les pièces redeviennent des objets indépendants (manipulation
/// libre, plus de cut_information.xml). Objet non coupé → 422 (Orca n'affiche
/// pas l'entrée) ; objet absent → 404.
pub async fn invalidate_cut(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path((project_raw, object_id)): Path<(String, String)>,
    body: Option<Json<InvalidateCutRequest>>,
) -> ApiResult<Json<InvalidateCutResponse>> {
    let project_id = parse_project_id(&project_raw)?;
    let project = state.storage.projects().get(user.id, project_id).await?;
    let expected = body
        .and_then(|Json(b)| b.expected_version)
        .unwrap_or(project.version);
    let mut scene = project.scene.clone();
    let changed = crate::scene::invalidate_cut_info(&mut scene, &object_id)
        .ok_or_else(|| ApiError::not_found("Objet"))?;
    if changed.is_empty() {
        return Err(ApiError::validation(
            "objet sans informations de coupe",
            serde_json::json!({ "object_id": object_id }),
        ));
    }
    let saved = state
        .storage
        .projects()
        .update(
            user.id,
            project_id,
            expected,
            scene,
            project.active_presets.clone(),
            None,
        )
        .await?;
    Ok(Json(InvalidateCutResponse {
        object_ids: changed,
        project: saved.into(),
    }))
}

#[cfg(test)]
mod arrange_options_tests {
    use super::*;

    fn config(pairs: &[(&str, ConfigValue)]) -> engine::api::DynamicPrintConfig {
        let mut c = engine::api::DynamicPrintConfig::new();
        for (k, v) in pairs {
            c.0.insert((*k).to_string(), v.clone());
        }
        c
    }

    #[test]
    fn reset_values_and_visibility_like_orca() {
        let c = config(&[
            ("print_sequence", ConfigValue::String("by layer".into())),
            ("extruder_clearance_radius", ConfigValue::Float(40.0)),
            ("printer_structure", ConfigValue::String("i3".into())),
            ("scan_first_layer", ConfigValue::Bool(true)),
        ]);
        let info = arrange_options_info(&c, true);
        assert!(!info.is_seq_print);
        assert_eq!(info.min_spacing, 6.0);
        assert_eq!(info.defaults.spacing, 0.0);
        assert!(!info.defaults.enable_rotation);
        assert!(info.defaults.allow_multi_materials_on_same_plate);
        assert!(info.defaults.avoid_extrusion_cali_region);
        assert!(info.defaults.align_to_y_axis, "i3 → aligné sur Y");
        assert!(info.show_avoid_extrusion_cali_region);
        assert!(!arrange_options_info(&c, false).show_avoid_extrusion_cali_region);
    }

    #[test]
    fn sequential_min_spacing_is_the_clearance_radius() {
        let c = config(&[
            ("print_sequence", ConfigValue::String("by object".into())),
            ("extruder_clearance_radius", ConfigValue::Float(40.0)),
        ]);
        let info = arrange_options_info(&c, false);
        assert!(info.is_seq_print);
        assert_eq!(info.min_spacing, 40.0);
        assert!(!info.defaults.align_to_y_axis);
        assert_eq!(
            min_object_distance(&engine::api::DynamicPrintConfig::new()),
            0.0
        );
    }

    /// Matrice d'instance de l'arrangeur = T·R·S de Three.js (Matrix4.compose,
    /// quaternion Euler XYZ), implémentation indépendante via glam ; le point
    /// transformé = rotation (rotate_vector_by_euler_deg) de l'échelle + position.
    #[test]
    fn scene_transform_matrix_is_translation_rotation_scale() {
        let cases = [
            ([10.0, -4.0, 3.0], [0.0, 0.0, 45.0], [1.0, 1.0, 1.0]),
            ([0.0, 0.0, 0.0], [0.0, 0.0, 30.0], [1.5, 1.0, 1.0]),
            ([5.0, 6.0, 7.0], [90.0, 0.0, 30.0], [1.0, 2.0, 0.5]),
            ([1.0, 2.0, 3.0], [12.0, -35.0, 160.0], [0.7, 1.3, 2.1]),
        ];
        for (position, rotation, scale) in cases {
            let m = scene_transform_matrix(&serde_json::json!({
                "position": position, "rotation": rotation, "scale": scale
            }));
            let [rx, ry, rz] = rotation.map(f64::to_radians);
            let expected = glam::DMat4::from_scale_rotation_translation(
                glam::DVec3::from_array(scale),
                glam::DQuat::from_euler(glam::EulerRot::XYZ, rx, ry, rz),
                glam::DVec3::from_array(position),
            );
            assert!(
                m.abs_diff_eq(expected, 1e-12),
                "{rotation:?} {scale:?} : {m:?} ≠ {expected:?}"
            );
            let v = [3.0, -2.0, 1.5];
            let scaled = [v[0] * scale[0], v[1] * scale[1], v[2] * scale[2]];
            let rotated = crate::scene::rotate_vector_by_euler_deg(rotation, scaled);
            let got = m.transform_point3(glam::DVec3::from_array(v));
            for axis in 0..3 {
                assert!((got[axis] - (rotated[axis] + position[axis])).abs() < 1e-9);
            }
        }
        // Cas de la phase 36 (diag-arrange) : cube à +45° → colonne X = (cos, sin).
        let m = scene_transform_matrix(&serde_json::json!({ "rotation": [0.0, 0.0, 45.0] }));
        assert!(
            (m.x_axis.y - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-12,
            "{m:?}"
        );
    }

    /// Infobulle et icône de GUI_ObjectList::get_mesh_errors_info.
    #[test]
    fn mesh_errors_info_like_object_list() {
        use engine::api::{MeshErrorStats, RepairedMeshErrors};
        let clean = MeshErrorsInfo::from_stats(&MeshErrorStats::default());
        assert!(!clean.warning && clean.manifold && clean.tooltip.is_empty());
        let stats = MeshErrorStats {
            open_edges: 3,
            repaired: RepairedMeshErrors {
                facets_reversed: 1,
                degenerate_facets: 1,
                ..Default::default()
            },
        };
        let info = MeshErrorsInfo::from_stats(&stats);
        assert!(info.warning && !info.manifold && info.repaired);
        assert_eq!(info.repaired_errors, 2);
        assert_eq!(
            info.tooltip,
            "2 errors repaired\nRemaining errors:\n\t3 non-manifold edges\n\nClick the icon to repair model object"
        );
        let one = MeshErrorsInfo::from_stats(&MeshErrorStats {
            open_edges: 1,
            repaired: Default::default(),
        });
        assert_eq!(
            one.tooltip,
            "Remaining errors:\n\t1 non-manifold edge\n\nClick the icon to repair model object"
        );
    }

    #[test]
    fn scene_plate_helpers() {
        let scene = serde_json::json!({
            "plates": { "plates": [ { "id": "a" }, { "id": "b", "locked": true } ], "activeId": "b" }
        });
        assert_eq!(scene_plates(&scene).len(), 2);
        assert_eq!(active_plate_index(&scene), Some(1));
        assert_eq!(
            active_plate_index(&serde_json::json!({ "plates": [] })),
            None
        );
    }
}
