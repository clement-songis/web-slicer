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

/// Résultat GCodeProcessor d'Orca du G-code d'un compte (404 si autre compte).
async fn processed_for(
    state: &AppState,
    gcode_id: GcodeId,
    user: crate::domain::UserId,
) -> ApiResult<std::sync::Arc<engine::api::gcode_process::ProcessedGcode>> {
    let gcode = state.storage.gcodes().get(user, gcode_id).await?; // 404, SC-008
    crate::engine::gcode_process::processed(FsPath::new(&gcode.file_path))
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "GCodeProcessor (préviz)");
            ApiError::conversion_failed("simulation du G-code par le moteur impossible")
        })
}

/// `GET /api/gcodes/{id}/preview/processed/meta` — résumé GCodeProcessor :
/// couches (cote, plage de mouvements, temps Normal/Silencieux), temps par
/// rôle / extrudeur / déplacements, estimations, volumes, G-code personnalisés.
pub async fn processed_meta(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let id = parse_gcode_id(&id)?;
    let p = processed_for(&state, id, user.id).await?;
    Ok(Json(p.meta.clone()))
}

/// `GET /api/gcodes/{id}/preview/processed/moves?from=&to=` — mouvements
/// simulés d'une plage de couches (format binaire `WSPm`, cf. engine::api::gcode_process).
pub async fn processed_moves(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(range): Query<LayerRange>,
) -> ApiResult<Response> {
    let id = parse_gcode_id(&id)?;
    let p = processed_for(&state, id, user.id).await?;
    let from = range.from.unwrap_or(0);
    let to = range.to.unwrap_or(usize::MAX);
    let buffer = p.encode_range(from, to);
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/octet-stream")],
        buffer,
    )
        .into_response())
}

/// `GET /api/gcodes/{id}/preview/processed/seams?from=&to=` — coutures émises
/// par GCodeProcessor (`EMoveType::Seam`) des couches [from, to] :
/// `{ seams: [{ layer, move, x, y, z }] }` (marqueurs « Seams » d'Orca).
pub async fn processed_seams(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(range): Query<LayerRange>,
) -> ApiResult<Json<serde_json::Value>> {
    let id = parse_gcode_id(&id)?;
    let p = processed_for(&state, id, user.id).await?;
    Ok(Json(seams_json(&p, &range)))
}

pub(crate) fn seams_json(
    p: &engine::api::gcode_process::ProcessedGcode,
    range: &LayerRange,
) -> serde_json::Value {
    let seams = p.seams(range.from.unwrap_or(0), range.to.unwrap_or(usize::MAX));
    serde_json::json!({ "seams": seams })
}

/// Corps de `POST /api/gcodes/plates-stats`.
#[derive(Debug, Deserialize)]
pub struct PlatesStatsRequest {
    /// G-code de chaque plateau tranché (ordre des plateaux).
    pub gcode_ids: Vec<String>,
}

/// `POST /api/gcodes/plates-stats` — statistiques « tous les plateaux »
/// d'Orca (GCodeViewer::render_all_plates_stats) : pour chaque G-code, le
/// tableau « Filament » du plateau (modèle / support / purge / tour par
/// filament) et ses temps ; puis l'agrégat (volumes cumulés par filament,
/// convertis en m / g, temps et coût sommés). Coût d'un plateau = coût total
/// figé au tranchage (`PrintStatistics::total_cost`, pied du G-code).
pub async fn plates_stats(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Json(body): Json<PlatesStatsRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    if body.gcode_ids.is_empty() || body.gcode_ids.len() > 64 {
        return Err(ApiError::validation(
            "gcode_ids doit contenir de 1 à 64 G-code",
            serde_json::json!({ "field": "gcode_ids" }),
        ));
    }
    let mut processed = Vec::with_capacity(body.gcode_ids.len());
    let mut plates = Vec::with_capacity(body.gcode_ids.len());
    let mut total_cost = 0f64;
    for raw in &body.gcode_ids {
        let id = parse_gcode_id(raw)?;
        let gcode = state.storage.gcodes().get(user.id, id).await?; // 404, SC-008
        let p = processed_for(&state, id, user.id).await?;
        let cost = gcode.stats["total_filament_cost"].as_f64().unwrap_or(0.0);
        total_cost += cost;
        let modes = &p.meta["summary"]["modes"];
        plates.push(serde_json::json!({
            "gcode_id": raw,
            "time": [modes[0]["time"].clone(), modes[1]["time"].clone()],
            "total_cost": cost,
            "filament_table": p.meta["filament_table"].clone(),
        }));
        processed.push(p);
    }
    let metas: Vec<&serde_json::Value> = processed.iter().map(|p| &p.meta).collect();
    let mut all = engine::api::gcode_process::all_plates_filament_table(&metas);
    all["total_cost"] = serde_json::json!(total_cost);
    Ok(Json(serde_json::json!({ "plates": plates, "all": all })))
}

/// Statistiques d'un G-code calculées par GCodeProcessor (G-code externe
/// ouvert : Orca affiche le résultat du processeur, pas le pied de fichier) :
/// temps Normal / Silencieux, « prepare », couches, filament par extrudeur
/// (volume total simulé → mm, cm³, g ; coût = g · coût/kg · 0,001 comme
/// GCode.cpp), changements de filament. Mêmes clés que `GcodeStats` quand
/// elles existent.
pub(crate) fn engine_stats(p: &engine::api::gcode_process::ProcessedGcode) -> serde_json::Value {
    use engine::api::gcode_process::used_filament_from_volume;
    let s = &p.meta["summary"];
    let at = |key: &str, i: usize| s[key].get(i).and_then(|v| v.as_f64()).unwrap_or(0.0);
    let volumes = s["total_volumes_per_extruder"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    let count = volumes
        .keys()
        .filter_map(|k| k.parse::<usize>().ok())
        .map(|k| k + 1)
        .max()
        .unwrap_or(0);
    let (mut mm, mut cm3, mut g, mut cost) = (vec![], vec![], vec![], vec![]);
    for i in 0..count {
        let v = volumes
            .get(&i.to_string())
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let (m, grams) =
            used_filament_from_volume(v, at("filament_diameters", i), at("filament_densities", i));
        mm.push(m * 1000.0);
        cm3.push(v * 0.001);
        g.push(grams);
        cost.push(grams * at("filament_costs", i) * 0.001);
    }
    let time = s["modes"][0]["time"].as_f64().unwrap_or(0.0);
    serde_json::json!({
        "source": "gcode_processor",
        "estimated_time_seconds": time.round() as u64,
        "estimated_time_exact_seconds": time,
        "estimated_time_stealth_seconds": s["modes"][1]["time"].as_f64().unwrap_or(0.0),
        "prepare_time_seconds": s["modes"][0]["prepare_time"].as_f64().unwrap_or(0.0),
        "layer_count": p.layers.len(),
        "filament_used_mm": mm,
        "filament_used_cm3": cm3,
        "filament_used_g": g,
        "filament_cost": cost,
        "total_filament_weight_g": g.iter().sum::<f64>(),
        "total_filament_cost": cost.iter().sum::<f64>(),
        "total_toolchanges": s["total_filament_changes"].clone(),
        "total_extruder_changes": s["total_extruder_changes"].clone(),
    })
}

/// Plage de couches demandée (indices inclusifs ; bornes optionnelles).
#[derive(Debug, Deserialize)]
pub struct LayerRange {
    pub(crate) from: Option<usize>,
    pub(crate) to: Option<usize>,
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
