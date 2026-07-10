//! Outils moteur de scène (`http-api.md` : arrange / orient / repair, T054).
//!
//! Ces endpoints font l'aller-retour serveur des opérations de scène (FR-012,
//! FR-013). Les implémentations vivent dans `crate::scene` (pures, testées) :
//! arrangement en grille, analyse de maillage pour le rapport de réparation, et
//! suggestion d'orientation (plus grande facette vers le bas). Les versions
//! haute-fidélité (nesting, recousu, auto-orient support-aware) et les
//! opérations booléennes passeront par le worker FFI `SlicerEngine` une fois
//! celui-ci câblé (phase tranchage P5) — non exposées ici pour ne pas dévier du
//! contrat http-api.md.

use std::path::Path as FsPath;

use axum::extract::{Path, State};
use axum::Json;

use crate::domain::{Model, ModelFormat, ModelId, ProjectId};
use crate::http::dto::{
    ArrangeRequest, ArrangeResponse, OrientRequest, OrientResponse, Placement, RepairResponse,
};
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;
use crate::mesh::parse_stl;
use crate::scene::{analyze_mesh, arrange_grid, suggest_orientation, Footprint};

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

/// `POST /api/projects/{id}/arrange` — dispose les empreintes fournies sans
/// collision dans le plateau (FR-013).
pub async fn arrange(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(project_raw): Path<String>,
    Json(req): Json<ArrangeRequest>,
) -> ApiResult<Json<ArrangeResponse>> {
    let project_id = parse_project_id(&project_raw)?;
    // Isolation : le projet doit appartenir au compte (sinon 404, SC-008).
    state.storage.projects().get(user.id, project_id).await?;

    if !(req.bed_width > 0.0 && req.bed_depth > 0.0) {
        return Err(ApiError::validation(
            "dimensions de plateau invalides",
            serde_json::json!({ "bed_width": req.bed_width, "bed_depth": req.bed_depth }),
        ));
    }
    let spacing = if req.spacing.is_finite() && req.spacing >= 0.0 {
        req.spacing
    } else {
        0.0
    };

    let items: Vec<Footprint> = req
        .items
        .iter()
        .map(|i| Footprint {
            id: i.id.clone(),
            width: i.width.max(0.0),
            depth: i.depth.max(0.0),
        })
        .collect();

    let placements = arrange_grid(&items, req.bed_width, req.bed_depth, spacing)
        .into_iter()
        .map(|p| Placement {
            id: p.id,
            x: p.x,
            y: p.y,
        })
        .collect();

    Ok(Json(ArrangeResponse { placements }))
}

/// `POST /api/projects/{id}/orient` — suggère la rotation posant la plus grande
/// facette du modèle à plat (FR-013).
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

/// `POST /api/models/{id}/repair` — analyse le maillage et renvoie un rapport
/// (triangles, facettes dégénérées, arêtes de bord, étanchéité) (FR-012).
pub async fn repair(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<RepairResponse>> {
    let model_id = parse_model_id(&id)?;
    let model = state.storage.models().get(user.id, model_id).await?; // 404 si autre compte
    let mesh = load_stl_mesh(&state, &model).await?;

    let report = analyze_mesh(&mesh);
    Ok(Json(RepairResponse {
        triangles: report.triangles as u32,
        degenerate: report.degenerate as u32,
        open_edges: report.open_edges as u32,
        watertight: report.watertight(),
    }))
}

/// Charge un modèle en vérifiant qu'il appartient au compte **et** au projet.
async fn load_project_model(
    state: &AppState,
    user_id: crate::domain::UserId,
    project_id: ProjectId,
    model_raw: &str,
) -> ApiResult<Model> {
    let model_id = parse_model_id(model_raw)?;
    let model = state.storage.models().get(user_id, model_id).await?; // 404 si autre compte
    if model.project_id != Some(project_id) {
        return Err(ApiError::not_found("Modèle"));
    }
    Ok(model)
}

/// Lit et décode le maillage STL d'un modèle. Les formats dépendant du moteur
/// (STEP à convertir, OBJ/3MF via aperçu client) ne sont pas encore analysables
/// côté serveur — mêmes réponses que `GET …/mesh`.
async fn load_stl_mesh(state: &AppState, model: &Model) -> ApiResult<crate::mesh::Mesh> {
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
