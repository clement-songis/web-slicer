//! `GCodeProcessor` d'Orca via libslic3r (gcode_process.cpp) — appelé depuis
//! `engine-worker gcode-process`.

use std::path::Path;

use crate::api::gcode_process::{filament_colours_from_gcode, ProcessedGcode};
use crate::api::{EngineError, EngineErrorCode, EngineResult};

use super::bridge::ffi;
use super::model::ffi_guard;

pub fn process_gcode(path: &Path) -> EngineResult<ProcessedGcode> {
    let invalid = |m: String| EngineError::new(EngineErrorCode::InvalidModel, m);
    let raw = {
        let _guard = ffi_guard();
        ffi::gcode_process_raw(&path.to_string_lossy()).map_err(|e| invalid(e.to_string()))?
    };
    // Couleurs des filaments de la config du G-code (palette de l'éditeur,
    // vue « Couleur »), absentes du résultat de GCodeProcessor.
    let mut summary = raw.summary_json.to_string();
    let colours = std::fs::File::open(path)
        .ok()
        .and_then(|f| filament_colours_from_gcode(std::io::BufReader::new(f)));
    if let Some(colours) = colours {
        if let Ok(serde_json::Value::Object(mut map)) = serde_json::from_str(&summary) {
            map.insert("filament_colours".into(), serde_json::json!(colours));
            summary = serde_json::Value::Object(map).to_string();
        }
    }
    ProcessedGcode::from_columns(raw.bytes, raw.floats, raw.ints, &summary).map_err(invalid)
}
