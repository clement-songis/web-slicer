//! Profil de hauteur variable via libslic3r (Slicing.cpp), outil « Variable
//! layer height » d'Orca : adaptatif et lissage.
//! Exécution : cargo test -p engine --features ffi --test ffi_layer_profile (devShell requis).

#![cfg(feature = "ffi")]

use engine::adapters::ffi::{layer_profile, run_mesh_op};
use engine::api::layer_profile::{LayerEditAction, LayerProfileOp, LayerProfileRequest};
use engine::api::split::{MeshOp, PrimitiveShape};
use engine::api::{DynamicPrintConfig, Model, TriangleMesh};

fn sphere() -> TriangleMesh {
    run_mesh_op(
        &MeshOp::Primitive {
            shape: PrimitiveShape::Sphere,
            side_mm: 25.6,
        },
        &[],
    )
    .expect("sphere")
    .remove(0)
    .1
}

fn request(op: LayerProfileOp) -> LayerProfileRequest {
    let mut mesh = sphere();
    // Posée sur le plateau (z ∈ [0, 25.6]).
    let zmin = mesh.vertices.iter().map(|v| v[2]).fold(f32::MAX, f32::min);
    mesh.vertices.iter_mut().for_each(|v| v[2] -= zmin);
    let mut model = Model::default();
    model.add_object("sphere", mesh);
    LayerProfileRequest {
        model,
        config: DynamicPrintConfig::new(),
        op,
    }
}

fn pairs(p: &[f64]) -> Vec<(f64, f64)> {
    p.chunks_exact(2).map(|c| (c[0], c[1])).collect()
}

#[test]
fn adaptive_profile_is_thin_at_poles_and_thick_at_equator() {
    let profile =
        layer_profile(&request(LayerProfileOp::Adaptive { quality: 0.5 })).expect("adaptive");
    let p = pairs(&profile);
    eprintln!("adaptive sphere: {} points", p.len());
    assert!(p.len() > 10, "{p:?}");
    assert!(p.windows(2).all(|w| w[1].0 >= w[0].0), "z croissant");
    // Bornes min/max_layer_height de la config par défaut (≤ 0,75 × buse 0,4).
    assert!(p.iter().all(|&(_, h)| h > 0.0 && h <= 0.3 + 1e-6), "{p:?}");
    let at = |z: f64| {
        p.iter()
            .min_by(|a, b| (a.0 - z).abs().total_cmp(&(b.0 - z).abs()))
            .unwrap()
            .1
    };
    assert!(
        at(12.8) > at(24.0),
        "équateur {} vs pôle {}",
        at(12.8),
        at(24.0)
    );
}

#[test]
fn smooth_flattens_a_step() {
    let adaptive = layer_profile(&request(LayerProfileOp::Adaptive { quality: 0.5 })).unwrap();
    let smoothed = layer_profile(&request(LayerProfileOp::Smooth {
        profile: adaptive.clone(),
        radius: 5,
        keep_min: false,
    }))
    .expect("smooth");
    assert!(!smoothed.is_empty() && smoothed.len().is_multiple_of(2));
    // Le lissage gaussien réduit la variation totale des hauteurs.
    let tv = |p: &[f64]| {
        pairs(p)
            .windows(2)
            .map(|w| (w[1].1 - w[0].1).abs())
            .sum::<f64>()
    };
    eprintln!(
        "TV adaptive {:.4} → smoothed {:.4}",
        tv(&adaptive),
        tv(&smoothed)
    );
    assert!(tv(&smoothed) <= tv(&adaptive) + 1e-9);
}

#[test]
fn adjust_brush_raises_layers_around_z_only() {
    // Pinceau Orca (défauts GUI : strength 0,005, bande 2 mm), profil vide =
    // initialisé par update_layer_height_profile comme à l'ouverture de l'outil.
    let mut profile = Vec::new();
    for _ in 0..20 {
        profile = layer_profile(&request(LayerProfileOp::Adjust {
            profile: profile.clone(),
            z: 12.0,
            delta: 0.005,
            band_width: 2.0,
            action: LayerEditAction::Increase,
        }))
        .expect("adjust");
    }
    let p = pairs(&profile);
    let at = |z: f64| {
        p.iter()
            .min_by(|a, b| (a.0 - z).abs().total_cmp(&(b.0 - z).abs()))
            .unwrap()
            .1
    };
    eprintln!(
        "adjust: {} points, h(12)={:.3} h(4)={:.3} h(20)={:.3}",
        p.len(),
        at(12.0),
        at(4.0),
        at(20.0)
    );
    assert!(at(12.0) > at(4.0) + 0.02, "{p:?}");
    assert!((at(4.0) - at(20.0)).abs() < 1e-6, "hors bande inchangé");
}
