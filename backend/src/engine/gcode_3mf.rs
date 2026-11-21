//! .gcode.3mf d'un ou de tous les plateaux tranchés (« Export plate sliced
//! file » / « Export all plate sliced file » d'Orca), écrit par libslic3r
//! (`store_bbs_3mf`, WithGcode | SkipModel) via `engine-worker export-gcode-3mf`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use engine::api::{DynamicPrintConfig, Model};

use super::scene::{result_json, write_json_bounded, SceneWorkerError};
use super::worker::{run_worker_at, DEFAULT_TIMEOUT};

/// G-code stocké d'un plateau (index 0-based).
#[derive(Debug, Clone, serde::Serialize)]
pub struct PlateGcode {
    pub plate_index: u32,
    pub gcode_path: PathBuf,
}

#[derive(serde::Serialize)]
struct Request<'a> {
    model: &'a Model,
    config: &'a DynamicPrintConfig,
    gcodes: &'a [PlateGcode],
    /// Réglages de chaque plateau (index = plateau) : `bed_type`… dans
    /// `model_settings.config` (lot 44).
    plate_settings: &'a [DynamicPrintConfig],
    export_plate: Option<u32>,
    output_path: PathBuf,
}

#[derive(serde::Deserialize)]
struct Response {
    path: PathBuf,
}

/// Octets du .gcode.3mf (`export_plate` = None : tous les plateaux de `gcodes`).
pub async fn write_gcode_3mf(
    model: &Model,
    config: &DynamicPrintConfig,
    gcodes: &[PlateGcode],
    plate_settings: &[DynamicPrintConfig],
    export_plate: Option<u32>,
) -> Result<Vec<u8>, SceneWorkerError> {
    write_gcode_3mf_at(
        &super::worker::worker_binary(),
        model,
        config,
        gcodes,
        plate_settings,
        export_plate,
        DEFAULT_TIMEOUT,
    )
    .await
}

pub async fn write_gcode_3mf_at(
    binary: &Path,
    model: &Model,
    config: &DynamicPrintConfig,
    gcodes: &[PlateGcode],
    plate_settings: &[DynamicPrintConfig],
    export_plate: Option<u32>,
    timeout: Duration,
) -> Result<Vec<u8>, SceneWorkerError> {
    if gcodes.is_empty() {
        return Err(SceneWorkerError::InvalidInput(
            "aucun plateau tranché à exporter".into(),
        ));
    }
    let dir = crate::workdir::tempdir()?;
    let output_path = dir.path().join("plates.gcode.3mf");
    let request_path = dir.path().join("export-gcode-3mf.json");
    write_json_bounded(
        &request_path,
        &Request {
            model,
            config,
            gcodes,
            plate_settings,
            export_plate,
            output_path: output_path.clone(),
        },
    )?;
    let stdout = run_worker_at(
        binary,
        &["export-gcode-3mf", &request_path.to_string_lossy()],
        timeout,
    )
    .await
    .map_err(super::scene::map_worker_error)?;
    let response: Response = serde_json::from_str(result_json(&stdout)?)
        .map_err(|e| SceneWorkerError::Protocol(e.to_string()))?;
    if response.path != output_path {
        return Err(SceneWorkerError::Protocol(
            "fichier hors du répertoire de travail".into(),
        ));
    }
    Ok(tokio::fs::read(&output_path).await?)
}
