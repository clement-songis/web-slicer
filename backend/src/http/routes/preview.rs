//! Prévisualisation d'un G-code (contrat http-api.md, T067, R6) :
//!   - `GET /api/gcodes/{id}/preview/meta` : couches, types présents, échelles ;
//!   - `GET /api/gcodes/{id}/preview/layers?from=&to=` : buffers binaires des
//!     segments d'une plage de couches (format `WSPv`).
//!
//! Le modèle couches → segments est reconstruit à la volée depuis le fichier
//! G-code stocké (`gcode.file_path`). Isolation (SC-008) : un G-code d'un autre
//! compte est traité comme inexistant (404).

use std::path::Path as FsPath;

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;

use crate::domain::GcodeId;
use crate::gcode::parse_preview;
use crate::http::dto::PreviewMeta;
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;

fn parse_gcode_id(raw: &str) -> ApiResult<GcodeId> {
    uuid::Uuid::parse_str(raw)
        .map(GcodeId)
        .map_err(|_| ApiError::not_found("G-code"))
}

/// Charge et décode le texte du G-code d'un compte (404 si absent/autre compte).
async fn load_gcode_text(
    state: &AppState,
    gcode_id: GcodeId,
    user: crate::domain::UserId,
) -> ApiResult<String> {
    let gcode = state.storage.gcodes().get(user, gcode_id).await?; // 404, SC-008
    let bytes = state
        .files
        .read(FsPath::new(&gcode.file_path))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "lecture du G-code (préviz)");
            ApiError::internal()
        })?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// `GET /api/gcodes/{id}/preview/meta` — méta-données de prévisualisation.
pub async fn meta(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<PreviewMeta>> {
    let id = parse_gcode_id(&id)?;
    let text = load_gcode_text(&state, id, user.id).await?;
    let summary = parse_preview(&text).summary();
    Ok(Json(PreviewMeta::from(summary)))
}

/// Plage de couches demandée (indices inclusifs ; bornes optionnelles).
#[derive(Debug, Deserialize)]
pub struct LayerRange {
    from: Option<usize>,
    to: Option<usize>,
}

/// `GET /api/gcodes/{id}/preview/layers?from=&to=` — buffers binaires (`WSPv`).
pub async fn layers(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(range): Query<LayerRange>,
) -> ApiResult<Response> {
    let id = parse_gcode_id(&id)?;
    let text = load_gcode_text(&state, id, user.id).await?;
    let model = parse_preview(&text);

    let from = range.from.unwrap_or(0);
    let to = range
        .to
        .unwrap_or_else(|| model.layer_count().saturating_sub(1));
    let buffer = model.encode_range(from, to);

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/octet-stream")],
        buffer,
    )
        .into_response())
}
