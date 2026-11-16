//! Profil de hauteur de couche via libslic3r (Slicing.cpp) — appelé uniquement
//! depuis `engine-worker layer-profile` : le backend ne lie jamais la FFI.

use crate::api::layer_profile::{LayerProfileOp, LayerProfileRequest};
use crate::api::{EngineError, EngineErrorCode, EngineResult};

use super::bridge::ffi;
use super::model::{config_to_orca_json, ffi_guard, to_raw_objects};

/// Calcule le profil plat `[z, h, …]` de l'unique objet de la requête.
pub fn layer_profile(req: &LayerProfileRequest) -> EngineResult<Vec<f64>> {
    let invalid = |m: String| EngineError::new(EngineErrorCode::InvalidModel, m);
    req.op.validate().map_err(invalid)?;
    if req.model.objects.len() != 1 {
        return Err(invalid("un seul objet attendu".into()));
    }
    let objects = to_raw_objects(&req.model);
    let config_json = config_to_orca_json(&req.config);
    let _guard = ffi_guard();
    if let LayerProfileOp::Adjust {
        profile,
        z,
        delta,
        band_width,
        action,
    } = &req.op
    {
        return ffi::adjust_layer_profile_raw(
            &objects,
            &config_json,
            profile,
            *z,
            *delta,
            *band_width,
            action.code(),
        )
        .map_err(|e| invalid(e.to_string()));
    }
    let (op, quality, profile, radius, keep_min) = match &req.op {
        LayerProfileOp::Adaptive { quality } => (0u8, *quality, Vec::new(), 0, false),
        LayerProfileOp::Smooth {
            profile,
            radius,
            keep_min,
        } => (1u8, 0.0, profile.clone(), *radius, *keep_min),
        LayerProfileOp::Adjust { .. } => unreachable!("traité ci-dessus"),
    };
    ffi::layer_profile_raw(
        &objects,
        &config_json,
        op,
        quality,
        &profile,
        radius,
        keep_min,
    )
    .map_err(|e| invalid(e.to_string()))
}
