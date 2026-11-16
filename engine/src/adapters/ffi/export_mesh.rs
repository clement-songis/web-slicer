//! Maillages d'export (STL) comme `Plater::export_stl` d'Orca (export_mesh.cpp).

use crate::api::{EngineError, EngineErrorCode, EngineResult, Model, TriangleMesh};

use super::bridge::ffi;
use super::model::{ffi_guard, to_raw_objects};

/// Un maillage monde par objet : pièces positives, volumes négatifs soustraits
/// (booléens CGAL d'Orca), toutes les instances fusionnées.
pub fn export_meshes(model: &Model) -> EngineResult<Vec<TriangleMesh>> {
    let objects = to_raw_objects(model);
    let _guard = ffi_guard();
    let raw = ffi::export_meshes_raw(&objects)
        .map_err(|e| EngineError::new(EngineErrorCode::InvalidModel, e.to_string()))?;
    Ok(raw
        .into_iter()
        .map(|m| TriangleMesh {
            vertices: m
                .vertices
                .chunks_exact(3)
                .map(|c| [c[0], c[1], c[2]])
                .collect(),
            indices: m
                .indices
                .chunks_exact(3)
                .map(|c| [c[0], c[1], c[2]])
                .collect(),
        })
        .collect())
}
