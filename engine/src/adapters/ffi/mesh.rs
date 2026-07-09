//! Réparation de maillage via admesh (T015, FR-012).

use crate::api::{EngineError, EngineErrorCode, EngineResult, RepairedMeshErrors, TriangleMesh};

use super::bridge::ffi;
use super::model::ffi_guard;

pub fn repair_mesh(mesh: &TriangleMesh) -> EngineResult<(TriangleMesh, RepairedMeshErrors)> {
    let _guard = ffi_guard();
    let raw = ffi::RawMesh {
        vertices: mesh.vertices.iter().flatten().copied().collect(),
        indices: mesh.indices.iter().flatten().copied().collect(),
    };
    let result = ffi::repair_mesh_raw(&raw)
        .map_err(|e| EngineError::new(EngineErrorCode::InvalidModel, e.to_string()))?;
    let repaired = TriangleMesh {
        vertices: result
            .mesh
            .vertices
            .chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect(),
        indices: result
            .mesh
            .indices
            .chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect(),
    };
    let report = RepairedMeshErrors {
        edges_fixed: result.edges_fixed.max(0) as u32,
        degenerate_facets: result.degenerate_facets.max(0) as u32,
        facets_removed: result.facets_removed.max(0) as u32,
        facets_reversed: result.facets_reversed.max(0) as u32,
        backwards_edges: result.backwards_edges.max(0) as u32,
    };
    Ok((repaired, report))
}
