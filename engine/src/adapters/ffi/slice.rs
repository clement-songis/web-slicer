//! Tranchage via le process `engine-worker` (T019, FR-014/R1/R9).
//!
//! Deux extrémités :
//! - [`slice`] (côté parent/backend) sérialise la `SliceRequest`, lance le
//!   worker et reconstruit le [`SliceResult`] à partir de la ligne `R` ;
//! - [`run_in_worker`] (côté worker) convertit la requête en scène brute,
//!   appelle le bridge C++ `slice_raw` (qui émet la progression sur stdout)
//!   et renvoie le résultat, sérialisé en `R` par le binaire worker.

use std::path::{Path, PathBuf};

use super::bridge::ffi;
use super::model::{config_to_orca_json, ffi_guard, to_raw_objects};
use super::worker;
use crate::api::{
    CancelToken, EngineError, EngineErrorCode, EngineResult, ProgressSink, SliceRequest,
    SliceResult, SliceStats, ValidateIssue, ValidateIssueKind, ValidateResult,
};

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

/// Côté worker : validation avant tranchage (`Print::validate` d'Orca) de la
/// même requête que `slice` (`work_dir` ignoré), sans `Print::process`.
pub fn validate_in_worker(request_path: &Path) -> EngineResult<ValidateResult> {
    let _guard = ffi_guard();
    let raw = std::fs::read_to_string(request_path)?;
    let req: SliceRequest = serde_json::from_str(&raw)
        .map_err(|e| EngineError::new(EngineErrorCode::Internal, format!("requête : {e}")))?;
    let objects = to_raw_objects(&req.model);
    let config_json = config_to_orca_json(&req.config);
    let calib = raw_calib(&req)?;
    let custom_gcodes_json = plate_custom_gcodes_json(&req)?;
    let issues = ffi::validate_raw(
        &objects,
        &config_json,
        &calib,
        &custom_gcodes_json,
        req.allow_mixed_temperatures,
    )
    .map_err(|e| EngineError::new(EngineErrorCode::InvalidConfig, e.to_string()))?;
    Ok(ValidateResult {
        issues: issues
            .into_iter()
            .map(|issue| ValidateIssue {
                message: issue.message,
                kind: ValidateIssueKind::from_code(issue.kind),
                is_warning: issue.is_warning,
                opt_key: (!issue.opt_key.is_empty()).then_some(issue.opt_key),
                object_index: usize::try_from(issue.object_index).ok(),
                params: issue.params,
            })
            .collect(),
    })
}

/// `Calib_Params` du pont (défaut = aucun mode), paramètres vérifiés.
fn raw_calib(req: &SliceRequest) -> EngineResult<ffi::RawCalibParams> {
    Ok(match req.calib {
        Some(params) => {
            params
                .validate()
                .map_err(|e| EngineError::new(EngineErrorCode::InvalidConfig, e))?;
            ffi::RawCalibParams {
                mode: params.mode.code(),
                start: params.start,
                end: params.end,
                step: params.step,
                print_numbers: params.print_numbers,
                freq_start_x: params.freq_start_x,
                freq_end_x: params.freq_end_x,
                freq_start_y: params.freq_start_y,
                freq_end_y: params.freq_end_y,
                test_model: params.test_model,
                shaper_type: params.shaper_type.as_orca().to_string(),
            }
        }
        None => ffi::RawCalibParams::default(),
    })
}

/// G-code personnalisés du plateau tranché seulement (rangés sous le plateau
/// courant du modèle C++), entrées vérifiées.
fn plate_custom_gcodes_json(req: &SliceRequest) -> EngineResult<String> {
    let plate_gcodes: Vec<_> = req
        .model
        .custom_gcodes
        .iter()
        .filter(|p| p.plate_index == req.plate_index)
        .collect();
    for plate in &plate_gcodes {
        for item in &plate.items {
            item.validate()
                .map_err(|e| EngineError::new(EngineErrorCode::InvalidConfig, e))?;
        }
    }
    Ok(serde_json::to_string(&plate_gcodes).expect("sérialisable"))
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

    let calib = raw_calib(&req)?;
    let custom_gcodes_json = plate_custom_gcodes_json(&req)?;
    let result = ffi::slice_raw(
        &objects,
        &config_json,
        &req.work_dir.to_string_lossy(),
        &calib,
        &custom_gcodes_json,
        req.allow_mixed_temperatures,
    )
    .map_err(|e| {
        let message = e.to_string();
        // Refus explicites du pont (validation Orca / cohérence multi-filament)
        // : erreur de configuration, pas un plantage du moteur.
        let code = if message.starts_with("configuration invalide")
            || message.starts_with("rien à trancher")
        {
            EngineErrorCode::InvalidConfig
        } else if message.starts_with("tranchage impossible") {
            // Slic3r::SlicingError(s) d'Orca (objet non imprimable tel quel) :
            // refus du modèle, pas un plantage du moteur.
            EngineErrorCode::InvalidModel
        } else {
            EngineErrorCode::EngineCrashed
        };
        EngineError::new(code, message)
    })?;

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
        output_filename: (!result.output_filename.is_empty()).then_some(result.output_filename),
        conflict: result.has_conflict.then(|| {
            engine_conflict(
                result.conflict_object1,
                result.conflict_object2,
                result.conflict_height,
                result.conflict_layer,
            )
        }),
        gcode_check: (result.gcode_check_code != 0).then(|| crate::api::GcodeCheck {
            error_code: result.gcode_check_code,
            message: result.gcode_check_message,
            issues: result
                .gcode_check_issues
                .into_iter()
                .map(|issue| crate::api::GcodeCheckIssue {
                    kind: match issue.kind {
                        1 => "height",
                        2 => "plate_area",
                        3 => "plate_height",
                        _ => "range",
                    }
                    .to_string(),
                    extruder: issue.extruder,
                    nozzle: issue.nozzle,
                    filaments: issue.filaments,
                })
                .collect(),
        }),
    })
}

fn engine_conflict(
    object1: String,
    object2: String,
    height: f64,
    layer: i64,
) -> crate::api::GcodeConflict {
    crate::api::GcodeConflict {
        object1,
        object2,
        height,
        layer,
    }
}
