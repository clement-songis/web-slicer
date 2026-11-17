//! Volumes de purge calculés depuis les couleurs (FlushVolCalculator d'Orca,
//! flush.cpp) : matrice N×N aplatie (mm³), sans le multiplicateur.

use crate::api::{EngineError, EngineErrorCode, EngineResult};

use super::bridge::ffi;

/// Minimum de purge par filament source pour la buse `nozzle` d'une config
/// résolue (portage de get_min_flush_volumes de la GUI d'Orca).
pub fn min_flush_volumes(
    config: &crate::api::DynamicPrintConfig,
    nozzle: u32,
) -> EngineResult<Vec<i32>> {
    let json = super::model::config_to_orca_json(config);
    ffi::min_flush_volumes_raw(&json, nozzle)
        .map_err(|e| EngineError::new(EngineErrorCode::InvalidConfig, e.to_string()))
}

/// `min_flush` : minimum par filament source (GUI : get_min_flush_volumes ;
/// CLI : nozzle_volume[0]) ; `flush_dataset` : nozzle_flush_dataset.
pub fn flush_volumes(
    colours: &[String],
    is_support: &[bool],
    min_flush: &[i32],
    flush_dataset: i32,
) -> EngineResult<Vec<f64>> {
    if colours.len() > 64 {
        return Err(EngineError::new(
            EngineErrorCode::InvalidConfig,
            "64 filaments au plus",
        ));
    }
    ffi::flush_volumes_raw(
        &colours.to_vec(),
        &is_support.to_vec(),
        &min_flush.to_vec(),
        flush_dataset,
    )
    .map_err(|e| EngineError::new(EngineErrorCode::InvalidConfig, e.to_string()))
}
