//! Vérification bout-en-bout de la peinture *fuzzy skin* (T142) : une peinture du
//! canal `fuzzy` doit réellement perturber les parois — le canal remonte jusqu'à
//! `libslic3r` par le bridge FFI (`ModelVolume::fuzzy_skin_facets`, cf.
//! `project.cpp`, même chemin que le lecteur 3MF).
//!
//! Contrairement aux supports (qui exigent `enable_support`), la fuzzy peinte
//! s'applique **sans réglage** : `apply_fuzzy_skin_segmentation` tourne dès que
//! `is_fuzzy_skin_painted()` (cf. `PrintObjectSlice.cpp`). L'effet observable :
//! la paroi extérieure gagne **beaucoup** de segments courts (le bruit fuzzy),
//! donc bien plus de mouvements d'extrusion `G1 X…` dans le G-code.
//!
//! Exécution : `cargo test -p engine --features ffi` (devShell requis).

#![cfg(feature = "ffi")]

mod common;

use engine::api::{CancelToken, ConfigValue, Model, ProgressSink, SliceRequest};

/// Nombre de mouvements planaires `G1 X…` (segments de parcours) du G-code.
fn move_count(gcode: &str) -> usize {
    gcode
        .lines()
        .filter(|l| l.starts_with("G1 ") && l.contains('X'))
        .count()
}

fn slice_gcode(model: Model, config: &engine::api::DynamicPrintConfig) -> String {
    let work = tempfile::tempdir().unwrap();
    let req = SliceRequest {
        model,
        config: config.clone(),
        plate_index: 0,
        work_dir: work.path().to_path_buf(),
    };
    let progress: ProgressSink = Box::new(|_phase, _ratio| {});
    let result = engine::adapters::ffi::slice(req, progress, CancelToken::new())
        .expect("le tranchage FFI aboutit");
    std::fs::read_to_string(&result.gcode_path).expect("G-code lisible")
}

#[test]
fn painted_fuzzy_skin_perturbs_the_walls() {
    let (model, mut config) = engine::adapters::ffi::read_project_3mf(&common::fixture(
        "parity/bridge__bbl-a1-standard.3mf",
    ))
    .expect("lecture du projet 3MF");
    // Amplitude du bruit fuzzy (indépendante de l'activation globale).
    config
        .set("fuzzy_skin_thickness", ConfigValue::Float(0.3))
        .unwrap();
    config
        .set("fuzzy_skin_point_distance", ConfigValue::Float(0.8))
        .unwrap();
    // `fuzzy_skin = none` (et non `disabled_fuzzy`) : globalement pas de fuzzy,
    // mais la **peinture** est autorisée à forcer `All` sur ses régions
    // (`PrintApply.cpp` : le passage à `FuzzySkinType::All` est sauté quand le
    // réglage vaut `disabled_fuzzy`). C'est donc bien la peinture qui déclenche.
    config
        .set("fuzzy_skin", ConfigValue::String("none".into()))
        .unwrap();

    // Sans peinture : parois lisses (fuzzy_skin = none partout).
    let plain = slice_gcode(model.clone(), &config);

    // Peinture fuzzy (état enforcer « 4 », comme le pinceau frontend) sur tout.
    let mut painted_model = model;
    for object in &mut painted_model.objects {
        for volume in &mut object.volumes {
            let n = volume.mesh.indices.len() as u32;
            volume.painted.fuzzy = (0..n).map(|t| (t, "4".into())).collect();
        }
    }
    let painted = slice_gcode(painted_model, &config);

    assert!(
        move_count(&painted) > move_count(&plain) * 3 / 2,
        "la fuzzy peinte doit multiplier les segments de paroi : \
         obtenu {} vs {} (lisse)",
        move_count(&painted),
        move_count(&plain)
    );
}
