//! `GCodeProcessor` d'Orca via libslic3r (gcode_process.cpp) — appelé depuis
//! `engine-worker gcode-process`.

use std::path::Path;

use crate::api::gcode_process::ProcessedGcode;
use crate::api::{EngineError, EngineErrorCode, EngineResult};

use super::bridge::ffi;
use super::model::ffi_guard;

pub fn process_gcode(path: &Path) -> EngineResult<ProcessedGcode> {
    let invalid = |m: String| EngineError::new(EngineErrorCode::InvalidModel, m);
    let raw = {
        let _guard = ffi_guard();
        ffi::gcode_process_raw(&path.to_string_lossy()).map_err(|e| invalid(e.to_string()))?
    };
    ProcessedGcode::from_columns(raw.bytes, raw.floats, raw.ints, &raw.summary_json)
        .map_err(invalid)
}
