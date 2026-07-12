//! Tranchage via le process `engine-worker` (T019, FR-014/R1/R9).
//!
//! Deux extrémités :
//! - [`slice`] (côté parent/backend) sérialise la `SliceRequest`, lance le
//!   worker et reconstruit le [`SliceResult`] à partir de la ligne `R` ;
//! - [`run_in_worker`] (côté worker) convertit la requête en scène brute,
//!   appelle le bridge C++ `slice_raw` (qui émet la progression sur stdout)
//!   et renvoie le résultat, sérialisé en `R` par le binaire worker.

use std::path::{Path, PathBuf};

use crate::api::{
    CancelToken, DynamicPrintConfig, EngineError, EngineErrorCode, EngineResult, ProgressSink,
    SliceRequest, SliceResult, SliceStats,
};
use crate::params::{self, orca_values};

use super::bridge::ffi;
use super::model::{ffi_guard, to_raw_objects};
use super::worker;

const REQUEST_FILE: &str = "slice-request.json";

/// Côté parent : tranche `req` dans un worker isolé. La progression est
/// répercutée sur `progress`, l'annulation (`cancel`) tue le worker.
pub fn slice(
    req: SliceRequest,
    progress: ProgressSink,
    cancel: CancelToken,
) -> EngineResult<SliceResult> {
    std::fs::create_dir_all(&req.work_dir)?;
    let request_path = req.work_dir.join(REQUEST_FILE);
    let json = serde_json::to_string(&req)
        .map_err(|e| EngineError::new(EngineErrorCode::Internal, e.to_string()))?;
    std::fs::write(&request_path, json)?;

    let result_json = worker::drive(
        &["slice", &request_path.to_string_lossy()],
        &progress,
        &cancel,
    )?;
    serde_json::from_str(&result_json)
        .map_err(|e| EngineError::new(EngineErrorCode::Internal, format!("résultat worker : {e}")))
}

/// Côté worker : lit la requête, appelle le bridge C++ et renvoie le résultat.
/// La progression est émise par `slice_raw` (C++) directement sur stdout.
pub fn run_in_worker(request_path: &Path) -> EngineResult<SliceResult> {
    // Initialise le runtime FFI (répertoires + `print_config_static_initializer`)
    // avant tout appel : sans quoi les caches statiques de config peuvent rester
    // vides selon l'élagage `--gc-sections` du binaire, et la validation rejette
    // à tort (ex. `layer_gcode` perdu → « G92 E0 requis »). Même garde que
    // `read_project_3mf` / `load_model`.
    let _guard = ffi_guard();
    let raw = std::fs::read_to_string(request_path)?;
    let req: SliceRequest = serde_json::from_str(&raw)
        .map_err(|e| EngineError::new(EngineErrorCode::Internal, format!("requête : {e}")))?;

    let objects = to_raw_objects(&req.model);
    let config_json = config_to_orca_json(&req.config);

    let result = ffi::slice_raw(&objects, &config_json, &req.work_dir.to_string_lossy())
        .map_err(|e| EngineError::new(EngineErrorCode::EngineCrashed, e.to_string()))?;

    Ok(SliceResult {
        gcode_path: PathBuf::from(result.gcode_path),
        stats: SliceStats {
            estimated_time_s: result.estimated_time_s,
            filament_mm: result.filament_mm,
            filament_g: vec![result.filament_g],
            layer_count: result.layer_count,
            tool_changes: result.tool_changes,
        },
        thumbnails: result.thumbnails.into_iter().map(PathBuf::from).collect(),
    })
}

/// Config typée → JSON {clé → valeur sérialisée Orca} (format attendu par le
/// bridge C++, symétrique de la lecture 3MF).
fn config_to_orca_json(config: &DynamicPrintConfig) -> String {
    let map: std::collections::BTreeMap<&str, String> = config
        .0
        .iter()
        .map(|(k, v)| {
            // Sérialisation pilotée par le registre : réimpose `%` aux pourcentages
            // (que `ConfigValue::Float` ne porte pas), sinon repli sur la forme nue.
            let s = match params::get(k) {
                Some(def) => orca_values::serialize_orca_value_for(def, v),
                None => orca_values::serialize_orca_value(v),
            };
            (k.as_str(), s)
        })
        .collect();
    serde_json::to_string(&map).expect("map de chaînes toujours sérialisable")
}
