//! Ouverture d'un G-code local (menu « Open G-code… ») : aperçu sans stockage.
//!
//! `POST /api/gcode/preview` (corps = texte G-code brut) renvoie, comme les
//! routes `/api/gcodes/{id}/preview/*` :
//! - `?meta=1` : la méta JSON (`PreviewMeta`) ;
//! - sinon `?from=&to=` : le buffer binaire `WSPv` de la plage de couches.
//!
//! Sans état : le client renvoie le fichier pour chaque fenêtre de couches
//! (chargement paresseux) ; rien n'est écrit côté serveur. Même parseur que
//! l'aperçu des tranchages (`crate::gcode::parse_preview`).
//!
//! Simulation moteur (lot 31, comme `GUI_App` en mode visionneuse :
//! `GCodeProcessor::process_file` sur le fichier ouvert) :
//! - `POST /api/gcode/process` (corps = G-code brut) : le fichier est gardé
//!   dans l'espace de l'utilisateur (`opened-gcodes/<empreinte>.gcode`, les
//!   `OPENED_KEEP` derniers), simulé par `engine-worker gcode-process` (cache
//!   `.wspg`) → `{ token, meta, stats, footer }` ;
//! - `GET /api/gcode/opened/{token}/meta|moves|seams` : mêmes réponses que
//!   `/api/gcodes/{id}/preview/processed/*`, sans renvoyer le fichier.

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;

use crate::gcode::parse_preview;
use crate::http::dto::PreviewMeta;
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::routes::preview::{engine_stats, seams_json, LayerRange};
use crate::http::state::AppState;

/// Nombre de G-code externes conservés par compte (avec leur cache).
pub const OPENED_KEEP: usize = 4;

/// `POST /api/gcode/process` — simulation GCodeProcessor d'un G-code ouvert.
pub async fn process(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    body: Bytes,
) -> ApiResult<Response> {
    use md5::{Digest, Md5};
    if body.is_empty() {
        return Err(ApiError::validation("G-code vide", serde_json::json!({})));
    }
    let token: String = Md5::digest(&body)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let path = state
        .files
        .write_opened_gcode(user.id, &token, &body, OPENED_KEEP)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "écriture du G-code ouvert");
            ApiError::internal()
        })?;
    let p = simulate(&path).await?;
    if p.layers.is_empty() {
        return Err(ApiError::validation(
            "aucune couche simulée : ce fichier n'est pas un G-code imprimable",
            serde_json::json!({}),
        ));
    }
    let footer = crate::gcode::parse_stats(&String::from_utf8_lossy(&body));
    Ok(Json(serde_json::json!({
        "token": token,
        "stats": engine_stats(&p),
        "footer": footer,
        "meta": p.meta,
    }))
    .into_response())
}

async fn simulate(
    path: &std::path::Path,
) -> ApiResult<std::sync::Arc<engine::api::gcode_process::ProcessedGcode>> {
    crate::engine::gcode_process::processed(path)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "GCodeProcessor (G-code ouvert)");
            ApiError::conversion_failed("simulation du G-code par le moteur impossible")
        })
}

/// G-code ouvert du compte (404 si jeton inconnu, invalide ou d'un autre compte).
async fn opened(
    state: &AppState,
    user: crate::domain::UserId,
    token: &str,
) -> ApiResult<std::sync::Arc<engine::api::gcode_process::ProcessedGcode>> {
    let path = state
        .files
        .opened_gcode_path(user, token)
        .map_err(|_| ApiError::not_found("G-code"))?;
    if tokio::fs::metadata(&path).await.is_err() {
        return Err(ApiError::not_found("G-code"));
    }
    simulate(&path).await
}

/// `GET /api/gcode/opened/{token}/meta`.
pub async fn opened_meta(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let p = opened(&state, user.id, &token).await?;
    Ok(Json(p.meta.clone()))
}

/// `GET /api/gcode/opened/{token}/moves?from=&to=` (format `WSPm`).
pub async fn opened_moves(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(range): Query<LayerRange>,
) -> ApiResult<Response> {
    let p = opened(&state, user.id, &token).await?;
    let buffer = p.encode_range(range.from.unwrap_or(0), range.to.unwrap_or(usize::MAX));
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/octet-stream")],
        buffer,
    )
        .into_response())
}

/// `GET /api/gcode/opened/{token}/seams?from=&to=`.
pub async fn opened_seams(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(range): Query<LayerRange>,
) -> ApiResult<Json<serde_json::Value>> {
    let p = opened(&state, user.id, &token).await?;
    Ok(Json(seams_json(&p, &range)))
}

#[derive(Debug, Deserialize)]
pub struct OpenQuery {
    #[serde(default)]
    meta: Option<u8>,
    from: Option<usize>,
    to: Option<usize>,
}

/// `POST /api/gcode/preview` — aperçu d'un G-code fourni par le client.
pub async fn preview(
    CurrentUser(_user): CurrentUser,
    Query(query): Query<OpenQuery>,
    body: Bytes,
) -> ApiResult<Response> {
    if body.is_empty() {
        return Err(ApiError::validation("G-code vide", serde_json::json!({})));
    }
    let text = String::from_utf8_lossy(&body);
    let model = parse_preview(&text);
    if model.layer_count() == 0 {
        return Err(ApiError::validation(
            "aucune couche reconnue : ce fichier n'est pas un G-code OrcaSlicer",
            serde_json::json!({}),
        ));
    }
    if query.meta.unwrap_or(0) != 0 {
        return Ok(Json(PreviewMeta::from(model.summary())).into_response());
    }
    let from = query.from.unwrap_or(0);
    let to = query
        .to
        .unwrap_or_else(|| model.layer_count().saturating_sub(1));
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/octet-stream")],
        model.encode_range(from, to),
    )
        .into_response())
}
