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
    if req.items.is_empty() {
        return Ok(Json(ArrangeResponse {
            placements: Vec::new(),
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
    let mut instance_ids = Vec::with_capacity(req.items.len());

    for item in &req.items {
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
        })
        .collect();
    Ok(Json(ArrangeResponse { placements }))
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
/// degrés, scale par composante et translation en mm.
fn scene_transform_matrix(value: &Value) -> glam::DMat4 {
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
    let [rx, ry, rz] = vector("rotation", [0.0; 3]).map(|v| v.to_radians());
    let [sx, sy, sz] = vector("scale", [1.0; 3]);
    let (s1, c1) = rx.sin_cos();
    let (s2, c2) = ry.sin_cos();
    let (s3, c3) = rz.sin_cos();
    let rotation = [
        [c2 * c3, -c2 * s3, s2],
        [c1 * s3 + c3 * s1 * s2, c1 * c3 - s1 * s2 * s3, -c2 * s1],
        [s1 * s3 - c1 * c3 * s2, c3 * s1 + c1 * s2 * s3, c1 * c2],
    ];
    DMat4::from_cols(
        DVec3::new(
            rotation[0][0] * sx,
            rotation[0][1] * sy,
            rotation[0][2] * sx,
        )
        .extend(0.0),
        DVec3::new(
            rotation[1][0] * sx,
            rotation[1][1] * sy,
            rotation[1][2] * sz,
        )
        .extend(0.0),
        DVec3::new(
            rotation[2][0] * sz,
            rotation[2][1] * sz,
            rotation[2][2] * sz,
        )
        .extend(0.0),
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

async fn load_converted_mesh(state: &AppState, model: &Model) -> ApiResult<TriangleMesh> {
    let Some(path) = model.mesh_path.as_deref() else {
        return Err(if model.conversion_error.is_some() {
            ApiError::conversion_failed("échec de la conversion du modèle par le moteur")
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
