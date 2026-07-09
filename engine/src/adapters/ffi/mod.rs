//! Adaptateur principal : bridge cxx vers libslic3r-headless (R1).
//!
//! T012 : chaîne de build + smoke. Le trait complet arrive en T013–T019
//! (les opérations lourdes passeront par le process `engine-worker`).

mod bridge;

use crate::api::{EngineError, EngineErrorCode, EngineResult};

/// Nombre de triangles d'un fichier modèle, via libslic3r (smoke T012).
pub fn model_triangle_count(path: &std::path::Path) -> EngineResult<usize> {
    bridge::ffi::model_triangle_count(&path.to_string_lossy())
        .map_err(|e| EngineError::new(EngineErrorCode::InvalidModel, e.to_string()))
}

/// Comptage runtime du registre C++ (croisement avec `params::REGISTRY`).
pub fn print_config_option_count() -> usize {
    bridge::ffi::print_config_option_count()
}
