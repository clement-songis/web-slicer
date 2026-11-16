//! Simplification quadrique réelle via libslic3r (`its_quadric_edge_collapse`).
//! Exécution : `cargo test -p engine --features ffi --test ffi_simplify`
//! (devShell requis).

#![cfg(feature = "ffi")]

use engine::api::mesh::SimplifyParams;
use engine::api::{EngineErrorCode, TriangleMesh};

/// Cube indexé (sommets partagés) de côté `size`, chaque face subdivisée en
/// `n`×`n` carrés : `12·n²` triangles, normales sortantes.
fn subdivided_cube(size: f32, n: u32) -> TriangleMesh {
    let mut mesh = TriangleMesh::default();
    let mut index = std::collections::HashMap::<[u32; 3], u32>::new();
    let step = size / n as f32;
    for (axis, positive) in [
        (0, false),
        (0, true),
        (1, false),
        (1, true),
        (2, false),
        (2, true),
    ] {
        let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
        let mut faces = Vec::new();
        for i in 0..n {
            for j in 0..n {
                let mut vertex = |i: u32, j: u32| -> u32 {
                    let mut key = [0u32; 3];
                    key[axis] = if positive { n } else { 0 };
                    key[u] = i;
                    key[v] = j;
                    *index.entry(key).or_insert_with(|| {
                        mesh.vertices.push(key.map(|c| c as f32 * step));
                        (mesh.vertices.len() - 1) as u32
                    })
                };
                let (a, b, c, d) = (
                    vertex(i, j),
                    vertex(i + 1, j),
                    vertex(i + 1, j + 1),
                    vertex(i, j + 1),
                );
                if positive {
                    faces.push([a, b, c]);
                    faces.push([a, c, d]);
                } else {
                    faces.push([a, c, b]);
                    faces.push([a, d, c]);
                }
            }
        }
        mesh.indices.extend(faces);
    }
    mesh
}

/// Arêtes de bord (utilisées par un seul triangle) : 0 = maillage fermé.
fn open_edges(mesh: &TriangleMesh) -> usize {
    let mut count = std::collections::HashMap::<(u32, u32), u32>::new();
    for t in &mesh.indices {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            *count.entry((a.min(b), a.max(b))).or_default() += 1;
        }
    }
    count.values().filter(|&&c| c == 1).count()
}

#[test]
fn ratio_halves_a_subdivided_cube_without_changing_its_shape() {
    let cube = subdivided_cube(20.0, 8);
    assert_eq!(cube.triangle_count(), 768);
    assert_eq!(open_edges(&cube), 0);

    let (simplified, report) = engine::adapters::ffi::simplify_mesh(
        &cube,
        &SimplifyParams::Ratio {
            decimate_ratio: 50.0,
        },
    )
    .expect("simplification libslic3r");
    assert_eq!(report.triangles_before, 768);
    assert_eq!(report.vertices_before as usize, cube.vertices.len());
    assert_eq!(report.triangles_after as usize, simplified.triangle_count());
    assert_eq!(report.vertices_after as usize, simplified.vertices.len());
    assert!(
        simplified.triangle_count() <= 384 && simplified.triangle_count() >= 12,
        "{report:?}"
    );
    assert!(simplified.vertices.len() < cube.vertices.len());
    assert_eq!(open_edges(&simplified), 0, "cube toujours fermé");
    // Aucun sommet orphelin, indices bornés.
    let mut used = vec![false; simplified.vertices.len()];
    for &i in simplified.indices.iter().flatten() {
        used[i as usize] = true;
    }
    assert!(used.iter().all(|&u| u), "sommets compactés");
    let (lo, hi) = simplified.bounding_box().unwrap();
    for k in 0..3 {
        assert!(
            lo[k].abs() < 1e-3 && (hi[k] - 20.0).abs() < 1e-3,
            "{lo:?} {hi:?}"
        );
    }
}

#[test]
fn detail_level_collapses_coplanar_faces() {
    let cube = subdivided_cube(20.0, 6);
    let (simplified, report) =
        engine::adapters::ffi::simplify_mesh(&cube, &SimplifyParams::Detail { level: 2 })
            .expect("simplification libslic3r");
    assert_eq!(report.triangles_before, 432);
    assert!(simplified.triangle_count() < 432, "{report:?}");
    assert!(report.max_error <= 0.1 + 1e-6, "{report:?}");
    assert_eq!(open_edges(&simplified), 0);
}

#[test]
fn invalid_parameters_and_meshes_are_input_errors() {
    let cube = subdivided_cube(20.0, 2);
    let err = engine::adapters::ffi::simplify_mesh(
        &cube,
        &SimplifyParams::Ratio {
            decimate_ratio: 100.0,
        },
    )
    .expect_err("ratio hors domaine");
    assert_eq!(err.code, EngineErrorCode::InvalidModel);

    let mut broken = cube.clone();
    broken.indices[0][0] = 10_000;
    let err = engine::adapters::ffi::simplify_mesh(
        &broken,
        &SimplifyParams::Ratio {
            decimate_ratio: 50.0,
        },
    )
    .expect_err("indice hors bornes");
    assert_eq!(err.code, EngineErrorCode::InvalidModel);
}
