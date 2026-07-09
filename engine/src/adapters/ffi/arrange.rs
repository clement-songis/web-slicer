//! Arrangement et orientation automatiques via libslic3r (T016, FR-013).

use crate::api::{
    ArrangeParams, BuildVolume, EngineError, EngineErrorCode, EngineResult, Model, ModelObject,
};

use super::bridge::ffi;
use super::model::{ffi_guard, to_object, to_raw_objects};

pub fn arrange(model: &mut Model, bed: &BuildVolume, params: &ArrangeParams) -> EngineResult<()> {
    let _guard = ffi_guard();
    let raw = to_raw_objects(model);
    let bed_xy: Vec<f64> = bed.bed_shape.iter().flatten().copied().collect();
    let arranged = ffi::arrange_raw(&raw, &bed_xy, params.clearance)
        .map_err(|e| EngineError::new(EngineErrorCode::OutOfBuildVolume, e.to_string()))?;
    model.objects = arranged.into_iter().map(to_object).collect();
    Ok(())
}

pub fn orient(object: &mut ModelObject) -> EngineResult<()> {
    let _guard = ffi_guard();
    let mut model = Model {
        objects: vec![object.clone()],
    };
    let raw = to_raw_objects(&model);
    let oriented = ffi::orient_raw(&raw[0])
        .map_err(|e| EngineError::new(EngineErrorCode::Internal, e.to_string()))?;
    model.objects = vec![to_object(oriented)];
    *object = model.objects.remove(0);
    Ok(())
}
