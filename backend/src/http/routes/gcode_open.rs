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

use axum::body::Bytes;
use axum::extract::Query;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;

use crate::gcode::parse_preview;
use crate::http::dto::PreviewMeta;
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;

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
