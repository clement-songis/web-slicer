//! Primitives d'Orca via libslic3r (`ObjectList::create_mesh`, GUI_ObjectList.cpp:2413 →
//! its_make_cube/cylinder/sphere/cone/torus, TriangleMesh.cpp).
//! Exécution : cargo test -p engine --features ffi --test ffi_primitives (devShell requis).
//!
//! Attentes dérivées indépendamment des paramètres de la GUI d'Orca
//! (côté = 0,1 × plateau, pas angulaires 2°/2°/2°/3°) : nombre de secteurs,
//! boîte englobante, volume analytique du polyèdre inscrit, maillage fermé.

#![cfg(feature = "ffi")]

use std::collections::HashMap;
use std::f64::consts::PI;

use engine::adapters::ffi::run_mesh_op;
use engine::api::split::{primitive_side_for_bed, signed_volume, MeshOp, PrimitiveShape};
use engine::api::TriangleMesh;

fn make(shape: PrimitiveShape, side: f64) -> TriangleMesh {
    let parts = run_mesh_op(
        &MeshOp::Primitive {
            shape,
            side_mm: side,
        },
        &[],
    )
    .expect("primitive");
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].0, shape.label());
    parts.into_iter().next().unwrap().1
}

fn open_edges(mesh: &TriangleMesh) -> usize {
    let mut count = HashMap::<(u32, u32), u32>::new();
    for t in &mesh.indices {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            *count.entry((a.min(b), a.max(b))).or_default() += 1;
        }
    }
    count.values().filter(|&&n| n != 2).count()
}

fn size(mesh: &TriangleMesh) -> [f64; 3] {
    let (lo, hi) = mesh.bounding_box().expect("bbox");
    std::array::from_fn(|i| f64::from(hi[i]) - f64::from(lo[i]))
}

fn close(a: f64, b: f64, rel: f64) -> bool {
    (a - b).abs() <= rel * b.abs().max(1e-9)
}

/// Plateau 256 mm (profils Bambu X1C / P1S) → côté 25,6 mm.
const SIDE: f64 = 25.6;

#[test]
fn side_follows_orca_bed_rule() {
    assert!(close(primitive_side_for_bed(256.0, 256.0), SIDE, 1e-12));
}

#[test]
fn cube_is_its_make_cube() {
    let m = make(PrimitiveShape::Cube, SIDE);
    assert_eq!((m.vertices.len(), m.indices.len()), (8, 12));
    assert_eq!(open_edges(&m), 0);
    assert!(size(&m).iter().all(|&s| close(s, SIDE, 1e-6)));
    assert!(close(signed_volume(&m), SIDE.powi(3), 1e-5));
}

#[test]
fn cylinder_has_180_sectors_like_orca() {
    // its_make_cylinder(r, h, fa = 2π/180) : 180 secteurs, 4 triangles chacun.
    let r = 0.5 * SIDE;
    let m = make(PrimitiveShape::Cylinder, SIDE);
    eprintln!(
        "cylinder: {} tri, {} vtx",
        m.indices.len(),
        m.vertices.len()
    );
    assert_eq!(m.indices.len(), 4 * 180, "triangles");
    assert_eq!(open_edges(&m), 0);
    let s = size(&m);
    assert!(close(s[2], SIDE, 1e-6));
    assert!(close(s[0], SIDE, 1e-3));
    let polygon = 0.5 * 180.0 * r * r * (2.0 * PI / 180.0).sin();
    assert!(close(signed_volume(&m), polygon * SIDE, 1e-4));
}

#[test]
fn disc_is_one_layer_thick_like_orca() {
    // Orca : its_make_cylinder(0.5 * side, 0.2f) — 0,2 mm, pas 2 mm.
    let m = make(PrimitiveShape::Disc, SIDE);
    assert!(close(size(&m)[2], 0.2, 1e-5));
    assert_eq!(m.indices.len(), 4 * 180);
    assert_eq!(open_edges(&m), 0);
}

#[test]
fn cone_has_180_sectors() {
    let r = 0.5 * SIDE;
    let m = make(PrimitiveShape::Cone, SIDE);
    eprintln!("cone: {} tri", m.indices.len());
    assert_eq!(open_edges(&m), 0);
    assert!(close(size(&m)[2], SIDE, 1e-6));
    let polygon = 0.5 * 180.0 * r * r * (2.0 * PI / 180.0).sin();
    assert!(close(signed_volume(&m), polygon * SIDE / 3.0, 1e-3));
}

#[test]
fn sphere_is_fine_like_orca() {
    // its_make_sphere(0.5 * side, π/90) : pas de 2°, volume à 0,1 % de la sphère.
    let r = 0.5 * SIDE;
    let m = make(PrimitiveShape::Sphere, SIDE);
    eprintln!("sphere: {} tri, {} vtx", m.indices.len(), m.vertices.len());
    assert_eq!(open_edges(&m), 0);
    assert!(size(&m).iter().all(|&s| close(s, SIDE, 2e-3)));
    let v = signed_volume(&m);
    assert!(close(v, 4.0 / 3.0 * PI * r.powi(3), 1e-3), "{v}");
    assert!(m.indices.len() > 10_000, "{}", m.indices.len());
}

#[test]
fn torus_matches_orca_proportions() {
    // its_make_torus(0.5 * side, 0.125 * side, π/60).
    let (big, small) = (0.5 * SIDE, 0.125 * SIDE);
    let m = make(PrimitiveShape::Torus, SIDE);
    eprintln!("torus: {} tri", m.indices.len());
    assert_eq!(open_edges(&m), 0);
    let s = size(&m);
    assert!(close(s[2], 2.0 * small, 2e-3));
    assert!(close(s[0], 2.0 * (big + small), 2e-3));
    let v = signed_volume(&m);
    assert!(close(v, 2.0 * PI * PI * big * small * small, 5e-3), "{v}");
}
