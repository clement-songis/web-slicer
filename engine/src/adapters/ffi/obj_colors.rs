//! Import OBJ coloré (obj_colors.cpp) : peinture MMU par facette + palette.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::api::{EngineError, EngineErrorCode, EngineResult, Model};

use super::bridge::ffi;
use super::model::{ffi_guard, to_object};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjColors {
    pub model: Model,
    /// Couleurs regroupées, filament n = palette[n-1] (« #RRGGBBAA » comme le CLI).
    pub palette: Vec<String>,
    /// Faux si l'OBJ ne porte aucune couleur.
    pub colored: bool,
}

pub fn load_obj_colors(path: &Path) -> EngineResult<ObjColors> {
    let _guard = ffi_guard();
    let raw = ffi::load_obj_colors_raw(&path.to_string_lossy())
        .map_err(|e| EngineError::new(EngineErrorCode::InvalidModel, e.to_string()))?;
    Ok(ObjColors {
        model: Model {
            objects: raw.objects.into_iter().map(to_object).collect(),
            custom_gcodes: Vec::new(),
        },
        palette: raw.palette.into_iter().collect(),
        colored: raw.colored,
    })
}
