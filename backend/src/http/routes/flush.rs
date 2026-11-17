//! `POST /api/flush-volumes` — volumes de purge calculés depuis les couleurs
//! des filaments par libslic3r (FlushVolCalculator, comme
//! Sidebar::auto_calc_flushing_volumes / WipingDialog d'Orca), via
//! `engine-worker flush-volumes`.
//!
//! Corps : `{ colours: ["#RRGGBB", …], multiplier?: f64, is_support?: [bool],
//! min_flush?: [i32], nozzle_flush_dataset?: i32 }`. Réponse :
//! `{ matrix: [f64; N×N] (ligne = depuis, mm³, sans multiplicateur),
//! multiplier }` — comme Orca, la matrice stockée n'inclut pas
//! `flush_multiplier`, appliqué au tranchage.

use axum::Json;
use serde::{Deserialize, Serialize};

use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;
use axum::extract::State;

#[derive(Debug, Deserialize)]
pub struct FlushVolumesRequest {
    #[serde(default)]
    pub colours: Vec<String>,
    #[serde(default)]
    pub multiplier: Option<f64>,
    #[serde(default)]
    pub is_support: Vec<bool>,
    /// Minimum par filament source (défaut 0 = minimum du CLI sans
    /// nozzle_volume ; la GUI passe get_min_flush_volumes).
    #[serde(default)]
    pub min_flush: Vec<i32>,
    #[serde(default)]
    pub nozzle_flush_dataset: i32,
    /// Projet dont la config résolue (presets actifs + scene.projectConfig)
    /// fournit le minimum (get_min_flush_volumes de la GUI d'Orca), le jeu de
    /// données et les filaments support — comme WipingDialog dans Orca.
    #[serde(default)]
    pub project_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FlushVolumesResponse {
    pub matrix: Vec<f64>,
    pub multiplier: f64,
}

pub async fn flush_volumes(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Json(body): Json<FlushVolumesRequest>,
) -> ApiResult<Json<FlushVolumesResponse>> {
    let invalid = |m: &str| ApiError::validation(m, serde_json::json!({}));
    if body.colours.is_empty() || body.colours.len() > 64 {
        return Err(invalid("1 à 64 couleurs attendues"));
    }
    let hex = |c: &str| {
        let h = c.strip_prefix('#').unwrap_or(c);
        (h.len() == 6 || h.len() == 8) && h.chars().all(|ch| ch.is_ascii_hexdigit())
    };
    if let Some(bad) = body.colours.iter().find(|c| !hex(c)) {
        return Err(ApiError::validation(
            "couleur invalide (#RRGGBB attendu)",
            serde_json::json!({ "colour": bad }),
        ));
    }
    let multiplier = body.multiplier.unwrap_or(1.0);
    if !(multiplier.is_finite() && multiplier > 0.0) {
        return Err(invalid("multiplicateur invalide"));
    }
    let config = match body.project_id.as_deref() {
        None => None,
        Some(raw) => {
            let id = uuid::Uuid::parse_str(raw)
                .map(crate::domain::ProjectId)
                .map_err(|_| ApiError::not_found("Projet"))?;
            let project = state.storage.projects().get(user.id, id).await?; // 404, SC-008
            let (mut config, _) =
                super::slice::resolve_active_config(&state, user.id, &project.active_presets)
                    .await?;
            super::slice::apply_project_config(&mut config, &project.scene);
            Some(config)
        }
    };
    let dir = crate::workdir::tempdir().map_err(|_| ApiError::internal())?;
    let path = dir.path().join("flush.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&serde_json::json!({
            "colours": body.colours,
            "is_support": body.is_support,
            "min_flush": body.min_flush,
            "flush_dataset": body.nozzle_flush_dataset,
            "config": config,
        }))
        .map_err(|_| ApiError::internal())?,
    )
    .map_err(|_| ApiError::internal())?;
    let stdout = crate::engine::run_worker(
        &["flush-volumes", &path.to_string_lossy()],
        std::time::Duration::from_secs(30),
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "flush-volumes dans engine-worker");
        ApiError::internal()
    })?;
    let stdout = String::from_utf8_lossy(&stdout);
    let line = stdout
        .lines()
        .rev()
        .find_map(|l| l.strip_prefix("R "))
        .ok_or_else(ApiError::internal)?;
    let value: serde_json::Value = serde_json::from_str(line).map_err(|_| ApiError::internal())?;
    let matrix: Vec<f64> =
        serde_json::from_value(value["matrix"].clone()).map_err(|_| ApiError::internal())?;
    Ok(Json(FlushVolumesResponse { matrix, multiplier }))
}
