//! Réparation de maillage via admesh (T015, FR-012) et simplification
//! quadrique via `its_quadric_edge_collapse` (outil « Simplifier » d'Orca).
//!
//! Contrairement à l'import Orca standard, cette opération explicite active le
//! rebouchage de trous. Admesh soude aussi les sommets proches, supprime les
//! facettes dégénérées/non connectées et corrige les normales. Il ne garantit
//! pas la réparation des auto-intersections ou de toute topologie arbitraire.

use crate::api::mesh::{SimplifyParams, SimplifyReport};
use crate::api::{EngineError, EngineErrorCode, EngineResult, RepairedMeshErrors, TriangleMesh};

use super::bridge::ffi;
use super::model::ffi_guard;

fn to_raw(mesh: &TriangleMesh) -> ffi::RawMesh {
    ffi::RawMesh {
        vertices: mesh.vertices.iter().flatten().copied().collect(),
        indices: mesh.indices.iter().flatten().copied().collect(),
    }
}

fn from_raw(raw: &ffi::RawMesh) -> TriangleMesh {
    TriangleMesh {
        vertices: raw
            .vertices
            .chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect(),
        indices: raw
            .indices
            .chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect(),
    }
}

/// Simplifie `mesh` par fusion d'arêtes quadrique (libslic3r). Le paramètre
/// est validé avant tout appel C++ ; un paramètre refusé ou un maillage
/// invalide remonte en `InvalidModel` (erreur utilisateur).
pub fn simplify_mesh(
    mesh: &TriangleMesh,
    params: &SimplifyParams,
) -> EngineResult<(TriangleMesh, SimplifyReport)> {
    let target = params
        .target(mesh.triangle_count())
        .map_err(|e| EngineError::new(EngineErrorCode::InvalidModel, e.to_string()))?;
    let _guard = ffi_guard();
    let result = ffi::simplify_mesh_raw(
        &to_raw(mesh),
        target.triangle_count,
        target.max_error.unwrap_or(f32::MAX),
    )
    .map_err(|e| EngineError::new(EngineErrorCode::InvalidModel, e.to_string()))?;
    let simplified = from_raw(&result.mesh);
    let report = SimplifyReport {
        triangles_before: mesh.triangle_count() as u32,
        triangles_after: simplified.triangle_count() as u32,
        vertices_before: mesh.vertices.len() as u32,
        vertices_after: simplified.vertices.len() as u32,
        max_error: result.max_error,
    };
    Ok((simplified, report))
}

/// Bits de quantification Draco d'Orca par défaut (`drc_bits` = 0 : aucune).
pub const DRC_BITS_DEFAULT: i32 = 0;

/// Écrit `mesh` au format Draco (`Slic3r::store_drc`, export DRC d'Orca).
pub fn store_drc(mesh: &TriangleMesh, path: &std::path::Path) -> EngineResult<()> {
    if mesh.is_empty() {
        return Err(EngineError::new(
            EngineErrorCode::InvalidModel,
            "maillage vide : rien à exporter",
        ));
    }
    let path = path
        .to_str()
        .ok_or_else(|| EngineError::new(EngineErrorCode::Io, "chemin d'export Draco non UTF-8"))?;
    let _guard = ffi_guard();
    ffi::store_drc_raw(&to_raw(mesh), path, DRC_BITS_DEFAULT)
        .map_err(|e| EngineError::new(EngineErrorCode::InvalidModel, e.to_string()))
}

pub fn repair_mesh(mesh: &TriangleMesh) -> EngineResult<(TriangleMesh, RepairedMeshErrors)> {
    let _guard = ffi_guard();
    let result = ffi::repair_mesh_raw(&to_raw(mesh))
        .map_err(|e| EngineError::new(EngineErrorCode::InvalidModel, e.to_string()))?;
    let repaired = from_raw(&result.mesh);
    let report = RepairedMeshErrors {
        edges_fixed: result.edges_fixed.max(0) as u32,
        degenerate_facets: result.degenerate_facets.max(0) as u32,
        facets_removed: result.facets_removed.max(0) as u32,
        facets_reversed: result.facets_reversed.max(0) as u32,
        backwards_edges: result.backwards_edges.max(0) as u32,
    };
    Ok((repaired, report))
}
