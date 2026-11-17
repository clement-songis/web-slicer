//! Projets de calibration d'Orca (calib_project.cpp) : portage des
//! `Plater::calib_*` de la GUI — modèle de resources/calib, coupes, mise à
//! l'échelle, réglages d'objet, surcharges de presets et paramètres du mode
//! calibration.

use std::path::Path;

pub use crate::api::calib::{CalibProject, CalibRequest, CALIB_KINDS};
use crate::api::{
    CalibMode, CalibParams, DynamicPrintConfig, EngineError, EngineErrorCode, EngineResult, Model,
};

use super::bridge::ffi;
use super::model::{config_from_orca_json, config_to_orca_json, ffi_guard, to_object};

fn mode_from_code(code: i32) -> CalibMode {
    use CalibMode::*;
    [
        None,
        PaLine,
        PaPattern,
        PaTower,
        AutoPaLine,
        FlowRate,
        TempTower,
        VolSpeedTower,
        VfaTower,
        RetractionTower,
        InputShapingFreq,
        InputShapingDamp,
        Cornering,
    ]
    .get(code.max(0) as usize)
    .copied()
    .unwrap_or(None)
}

pub fn calib_project(
    request: &CalibRequest,
    config: &DynamicPrintConfig,
    resources: &Path,
) -> EngineResult<CalibProject> {
    if !CALIB_KINDS.contains(&request.kind.as_str()) {
        return Err(EngineError::new(
            EngineErrorCode::InvalidConfig,
            format!("calibration inconnue : {}", request.kind),
        ));
    }
    let needs_range = !request.kind.starts_with("flow_");
    if needs_range
        && !(request.start.is_finite() && request.end.is_finite() && request.step.is_finite())
    {
        return Err(EngineError::new(
            EngineErrorCode::InvalidConfig,
            "plage invalide",
        ));
    }
    let stepless = matches!(
        request.kind.as_str(),
        "temp_tower" | "pa_line" | "is_freq" | "is_damp" | "cornering"
    );
    if needs_range && !stepless && request.step <= 0.0 {
        return Err(EngineError::new(
            EngineErrorCode::InvalidConfig,
            "pas de calibration nul ou négatif",
        ));
    }
    let json = config_to_orca_json(config);
    let _guard = ffi_guard();
    let raw = ffi::calib_project_raw(
        &request.kind,
        request.start,
        request.end,
        request.step,
        &json,
        &resources.to_string_lossy(),
        &serde_json::json!({
            "option": request.option,
            "test_model": request.test_model,
            "print_numbers": request.print_numbers,
            "speeds": request.speeds,
            "accelerations": request.accelerations,
        })
        .to_string(),
    )
    .map_err(|e| EngineError::new(EngineErrorCode::InvalidConfig, e.to_string()))?;
    let mode = mode_from_code(raw.calib_mode);
    let calib = (mode != CalibMode::None).then_some(CalibParams {
        mode,
        start: raw.calib_start,
        end: raw.calib_end,
        step: raw.calib_step,
        print_numbers: request.print_numbers,
        freq_start_x: request.freq_start_x,
        freq_end_x: request.freq_end_x,
        freq_start_y: request.freq_start_y,
        freq_end_y: request.freq_end_y,
        test_model: request.test_model,
        shaper_type: request.shaper_type,
    });
    Ok(CalibProject {
        name: raw.name.clone(),
        model: Model {
            objects: raw.objects.into_iter().map(to_object).collect(),
            custom_gcodes: serde_json::from_str(&raw.custom_gcodes_json).unwrap_or_default(),
        },
        overrides: config_from_orca_json(&raw.overrides_json),
        calib,
    })
}
