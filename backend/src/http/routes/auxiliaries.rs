//! Fichiers auxiliaires d'un projet (Orca : onglet « Fichiers auxiliaires »,
//! dossier Auxiliaries/ du 3MF — images du modèle, nomenclature, guide
//! d'assemblage, autres). Stockés par projet ; empaquetés dans l'export 3MF et
//! extraits à l'import 3MF par libslic3r (store_bbs_3mf / load_bbs_3mf).
//!
//! - GET    /api/projects/{id}/auxiliaries               → [{path, size}]
//! - PUT    /api/projects/{id}/auxiliaries/{folder}/{name} (corps = octets)
//! - GET    /api/projects/{id}/auxiliaries/{folder}/{name} → octets
//! - DELETE /api/projects/{id}/auxiliaries/{folder}/{name}
//!   folder ∈ « Model Pictures », « Bill of Materials », « Assembly Guide », « Others ».

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};

use crate::adapters::files::AUXILIARY_FOLDERS;
use crate::domain::{ProjectId, UserId};
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;

/// 50 Mo par fichier auxiliaire.
const MAX_AUX_BYTES: usize = 50 * 1024 * 1024;

async fn owned_project(state: &AppState, user: UserId, raw: &str) -> ApiResult<ProjectId> {
    let id = uuid::Uuid::parse_str(raw)
        .map(ProjectId)
        .map_err(|_| ApiError::not_found("Projet"))?;
    state.storage.projects().get(user, id).await?;
    Ok(id)
}

fn aux_path(
    state: &AppState,
    user: UserId,
    project: ProjectId,
    folder: &str,
    name: &str,
) -> ApiResult<std::path::PathBuf> {
    state
        .files
        .auxiliary_path(user, project, &format!("{folder}/{name}"))
        .map_err(|_| {
            ApiError::validation(
                "chemin auxiliaire invalide",
                json!({ "folder": folder, "name": name, "folders": AUXILIARY_FOLDERS }),
            )
        })
}

pub async fn list(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(raw): Path<String>,
) -> ApiResult<Json<Value>> {
    let project = owned_project(&state, user.id, &raw).await?;
    let files = state
        .files
        .list_auxiliaries(user.id, project)
        .await
        .map_err(|_| ApiError::internal())?;
    Ok(Json(Value::Array(
        files
            .into_iter()
            .map(|(path, size)| json!({ "path": path, "size": size }))
            .collect(),
    )))
}

pub async fn put(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path((raw, folder, name)): Path<(String, String, String)>,
    body: Bytes,
) -> ApiResult<StatusCode> {
    let project = owned_project(&state, user.id, &raw).await?;
    let path = aux_path(&state, user.id, project, &folder, &name)?;
    if body.len() > MAX_AUX_BYTES {
        return Err(ApiError::validation(
            "fichier trop volumineux (50 Mo max.)",
            json!({ "size": body.len() }),
        ));
    }
    let existed = path.exists();
    state
        .files
        .write_auxiliary(&path, &body)
        .await
        .map_err(|_| ApiError::internal())?;
    Ok(if existed {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::CREATED
    })
}

pub async fn get(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path((raw, folder, name)): Path<(String, String, String)>,
) -> ApiResult<Response> {
    let project = owned_project(&state, user.id, &raw).await?;
    let path = aux_path(&state, user.id, project, &folder, &name)?;
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|_| ApiError::not_found("Fichier"))?;
    let mime = mime_guess(&name);
    Ok(([(header::CONTENT_TYPE, mime)], bytes).into_response())
}

pub async fn delete(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path((raw, folder, name)): Path<(String, String, String)>,
) -> ApiResult<StatusCode> {
    let project = owned_project(&state, user.id, &raw).await?;
    let path = aux_path(&state, user.id, project, &folder, &name)?;
    tokio::fs::remove_file(&path)
        .await
        .map_err(|_| ApiError::not_found("Fichier"))?;
    Ok(StatusCode::NO_CONTENT)
}

fn mime_guess(name: &str) -> &'static str {
    match name
        .rsplit('.')
        .next()
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("pdf") => "application/pdf",
        Some("txt" | "md") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}
