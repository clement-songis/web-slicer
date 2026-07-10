//! Endpoints projets (`/api/projects/*`). Toute ressource est scopée par le
//! compte connecté (`CurrentUser`) : un projet d'autrui répond **404** (SC-008,
//! jamais 403). Handlers minces : validation DTO → repo → réponse.
//!
//! La scène et les presets actifs sont des documents JSON versionnés
//! (`schema_version`) ; la sauvegarde utilise un verrou optimiste (409 en cas
//! de conflit multi-onglets).

use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use uuid::Uuid;

use crate::domain::{NewProject, ProjectId};
use crate::http::dto::{
    CreateProjectRequest, ProjectResponse, RenameProjectRequest, SaveProjectRequest,
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
    Ok(Json(projects.into_iter().map(Into::into).collect()))
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

/// `GET /api/projects/{id}` — ouvre un projet (404 si absent ou d'autrui).
pub async fn get(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ProjectResponse>> {
    let id = parse_project_id(&id)?;
    let project = state.storage.projects().get(user.id, id).await?;
    Ok(Json(project.into()))
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
    Ok(Json(project.into()))
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

/// `POST /api/projects/{id}/duplicate` — copie scène + presets sous un nouveau nom.
pub async fn duplicate(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<(StatusCode, Json<ProjectResponse>)> {
    let id = parse_project_id(&id)?;
    let source = state.storage.projects().get(user.id, id).await?;
    let copy = state
        .storage
        .projects()
        .create(
            user.id,
            NewProject {
                name: format!("{} (copie)", source.name),
                scene: source.scene,
                active_presets: source.active_presets,
                thumbnail_path: None,
            },
        )
        .await?;
    Ok((StatusCode::CREATED, Json(copy.into())))
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
    Ok(Json(project.into()))
}

/// `GET /api/projects/{id}/thumbnail` — vignette du projet si elle existe.
pub async fn thumbnail(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let id = parse_project_id(&id)?;
    let project = state.storage.projects().get(user.id, id).await?;
    let path = project
        .thumbnail_path
        .ok_or_else(|| ApiError::not_found("Vignette"))?;
    let bytes = state
        .files
        .read(std::path::Path::new(&path))
        .await
        .map_err(|_| ApiError::not_found("Vignette"))?;
    Ok(([(header::CONTENT_TYPE, "image/png")], bytes).into_response())
}
