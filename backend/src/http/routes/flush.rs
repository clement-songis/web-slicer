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
    if body.colours.len() > 64 {
        return Err(invalid("64 couleurs au plus"));
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
    // Aucun filament : matrice 0 × 0 (réponse valide, sans worker). L'interface
    // sonde la disponibilité de la route par une liste vide à chaque session :
    // un 422 ici apparaissait comme une erreur en console (audit J2-i).
    if body.colours.is_empty() {
        return Ok(Json(FlushVolumesResponse {
            matrix: Vec::new(),
            multiplier,
        }));
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
    let matrix = worker_flush_matrix(serde_json::json!({
        "colours": body.colours,
        "is_support": body.is_support,
        "min_flush": body.min_flush,
        "flush_dataset": body.nozzle_flush_dataset,
        "config": config,
    }))
    .await?;
    Ok(Json(FlushVolumesResponse { matrix, multiplier }))
}

/// `engine-worker flush-volumes` sur une requête `{ colours, is_support,
/// min_flush, flush_dataset, config? }` → matrice N×N (mm³, sans
/// multiplicateur).
async fn worker_flush_matrix(request: serde_json::Value) -> ApiResult<Vec<f64>> {
    let dir = crate::workdir::tempdir().map_err(|_| ApiError::internal())?;
    let path = dir.path().join("flush.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&request).map_err(|_| ApiError::internal())?,
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
    serde_json::from_value(value["matrix"].clone()).map_err(|_| ApiError::internal())
}

/// Lot 44 (audit 43 X-d) : matrice de purge d'une config **sans** matrice
/// valide (n filaments ≥ 2, taille ≠ n² × buses — typiquement les 4×4 du
/// registre complétés à l'export), calculée comme au tranchage : le moteur
/// applique alors `cli_update_flush_volumes` (OrcaSlicer.cpp, « compute the
/// flush volume » : FlushVolCalculator depuis `filament_colour`, minimum
/// `nozzle_volume[0]`, jeu de données `nozzle_flush_dataset`, filaments
/// support), une sous-matrice n × n par buse. Le 3MF exporté porte donc la
/// matrice réellement purgée par web-slicer, de la taille qu'Orca exige
/// (`PresetBundle::update_multi_material_filament_presets` ; la CLI refuse
/// sinon « Flush volumes matrix do not match to the correct size! »).
/// Matrice déjà de la bonne taille (projet, preset) ou un seul filament :
/// inchangée (Orca garde la 4×4 d'un projet mono-filament).
pub(crate) async fn complete_flush_matrix(
    config: &mut engine::api::DynamicPrintConfig,
) -> ApiResult<()> {
    use engine::api::ConfigValue;
    let colours = match config.get("filament_colour") {
        Some(ConfigValue::Strings(v)) => v.clone(),
        _ => return Ok(()),
    };
    let n = colours.len();
    if n < 2 {
        return Ok(());
    }
    let nozzles = match config.get("nozzle_diameter") {
        Some(ConfigValue::Floats(v)) => v.len().max(1),
        _ => 1,
    };
    if let Some(ConfigValue::Floats(m)) = config.get("flush_volumes_matrix") {
        if m.len() == n * n * nozzles || (nozzles > 1 && m.len() == n * n) {
            return Ok(());
        }
    }
    let first_float = |key: &str| match config.get(key) {
        Some(ConfigValue::Floats(v)) => v.first().copied(),
        Some(ConfigValue::FloatsNullable(v)) => v.first().copied().flatten(),
        Some(ConfigValue::Float(f)) => Some(*f),
        _ => None,
    };
    let min_flush = first_float("nozzle_volume").unwrap_or(0.0) as i32;
    let datasets: Vec<i64> = match config.get("nozzle_flush_dataset") {
        Some(ConfigValue::Ints(v)) => v.clone(),
        _ => Vec::new(),
    };
    let is_support = match config.get("filament_is_support") {
        Some(ConfigValue::Bools(v)) => v.clone(),
        _ => Vec::new(),
    };
    let mut matrix = Vec::with_capacity(n * n * nozzles);
    for nozzle in 0..nozzles {
        let dataset = datasets
            .get(nozzle)
            .or(datasets.first())
            .copied()
            .unwrap_or(0);
        let sub = worker_flush_matrix(serde_json::json!({
            "colours": colours,
            "is_support": is_support,
            "min_flush": vec![min_flush; n],
            "flush_dataset": dataset,
        }))
        .await?;
        if sub.len() != n * n {
            return Err(ApiError::internal());
        }
        matrix.extend(sub);
    }
    config
        .0
        .insert("flush_volumes_matrix".into(), ConfigValue::Floats(matrix));
    let mut multipliers = match config.get("flush_multiplier") {
        Some(ConfigValue::Floats(v)) => v.clone(),
        _ => Vec::new(),
    };
    if multipliers.len() != nozzles {
        // cli_update_flush_volumes : complété à 1 par buse.
        multipliers.resize(nozzles, 1.0);
        config
            .0
            .insert("flush_multiplier".into(), ConfigValue::Floats(multipliers));
    }
    Ok(())
}
