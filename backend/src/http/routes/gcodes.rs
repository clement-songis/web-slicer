//! Consultation d'un G-code produit (contrat http-api.md, T066) :
//!   - `GET /api/gcodes/{id}/download` : export du fichier (FR-044) ;
//!   - `GET /api/gcodes/{id}/stats` : statistiques d'impression (FR-043).
//!
//! Isolation (SC-008) : un G-code d'un autre compte est traité comme inexistant
//! (404), la propriété étant vérifiée par `GcodeRepo::get(owner, id)`.

use std::path::Path as FsPath;

use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::domain::GcodeId;
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;

fn parse_gcode_id(raw: &str) -> ApiResult<GcodeId> {
    uuid::Uuid::parse_str(raw)
        .map(GcodeId)
        .map_err(|_| ApiError::not_found("G-code"))
}

/// `GET /api/gcodes/{id}/download` — télécharge le fichier G-code (pièce jointe).
pub async fn download(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let id = parse_gcode_id(&id)?;
    let gcode = state.storage.gcodes().get(user.id, id).await?; // 404, SC-008

    let bytes = state
        .files
        .read(FsPath::new(&gcode.file_path))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "lecture du G-code");
            ApiError::internal()
        })?;

    let filename = format!("{}.gcode", gcode.id);
    Ok((
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                "text/plain; charset=utf-8".to_string(),
            ),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        bytes,
    )
        .into_response())
}

/// `GET /api/gcodes/{id}/stats` — statistiques d'impression figées (FR-043).
pub async fn stats(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let id = parse_gcode_id(&id)?;
    let gcode = state.storage.gcodes().get(user.id, id).await?; // 404, SC-008
    Ok((StatusCode::OK, Json(gcode.stats)).into_response())
}
