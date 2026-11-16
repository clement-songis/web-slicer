//! Subdivision de Loop réelle via libslic3r (`TriangleMeshDeal::smooth_triangle_mesh`,
//! « Subdivision mesh (Lost color) » d'Orca).
//! Exécution : `cargo test -p engine --features ffi --test ffi_smooth` (devShell requis).

#![cfg(feature = "ffi")]

use std::collections::HashMap;

use engine::adapters::ffi::run_mesh_op;
use engine::api::split::{signed_volume, MeshOp};
use engine::api::{EngineErrorCode, TriangleMesh};

/// Cube fermé indexé (8 sommets, 12 triangles, normales sortantes).
fn cube(size: f32) -> TriangleMesh {
    let s = size;
    TriangleMesh {
        vertices: vec![
            [0.0, 0.0, 0.0],
            [s, 0.0, 0.0],
            [s, s, 0.0],
            [0.0, s, 0.0],
            [0.0, 0.0, s],
            [s, 0.0, s],
            [s, s, s],
            [0.0, s, s],
        ],
        indices: vec![
            [0, 2, 1],
            [0, 3, 2],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ],
    }
}

/// Arêtes de bord (utilisées par un nombre de triangles ≠ 2) : 0 = fermé.
fn open_edges(mesh: &TriangleMesh) -> usize {
    let mut count = HashMap::<(u32, u32), u32>::new();
    for t in &mesh.indices {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            *count.entry((a.min(b), a.max(b))).or_default() += 1;
        }
    }
    count.values().filter(|&&n| n != 2).count()
}

#[test]
fn loop_subdivision_quadruples_triangles_and_stays_closed() {
    let source = cube(20.0);
    let parts = run_mesh_op(&MeshOp::Smooth, std::slice::from_ref(&source)).expect("smooth");
    assert_eq!(parts.len(), 1);
    let (label, mesh) = &parts[0];
    assert_eq!(label, "result");
    assert_eq!(mesh.indices.len(), source.indices.len() * 4);
    // Loop : un sommet par sommet d'origine + un par arête (V + E = 8 + 18).
    assert_eq!(mesh.vertices.len(), 26);
    assert_eq!(open_edges(mesh), 0, "maillage subdivisé non fermé");
    // Lissage : volume positif, strictement réduit (sommets tirés vers l'intérieur).
    let (before, after) = (signed_volume(&source), signed_volume(mesh));
    assert!(after > 0.0 && after < before, "volume {before} → {after}");
    // Deux passes : 12 → 48 → 192 triangles, toujours fermé.
    let again = run_mesh_op(&MeshOp::Smooth, std::slice::from_ref(mesh)).expect("smooth ×2");
    assert_eq!(again[0].1.indices.len(), 192);
    assert_eq!(open_edges(&again[0].1), 0);
}

#[test]
fn open_mesh_is_refused_like_orca() {
    let mut open = cube(10.0);
    open.indices.pop();
    let err = run_mesh_op(&MeshOp::Smooth, &[open]).unwrap_err();
    assert_eq!(err.code, EngineErrorCode::InvalidModel);
    assert!(err.to_string().contains("réparez"), "{err}");
}
