//! Vérification bout-en-bout de la hauteur de couche variable (T058/T147) : un
//! `layer_height_profile` posé sur un `ModelObject` doit réellement changer le
//! tranchage — le vecteur plat `[z,h,…]` remonte jusqu'à `libslic3r` par le
//! bridge FFI (`ModelObject::layer_height_profile.set(...)`, cf. `project.cpp`).
//!
//! Contrat de `libslic3r` (PrintObject::update_layer_height_profile) : le profil
//! est **rejeté puis régénéré uniforme** s'il n'est pas de longueur paire, si sa
//! dernière cote ≠ sommet de l'objet, ou si `profil[1]` ≠ hauteur de 1ʳᵉ couche.
//! On construit donc un profil **valide** à partir du sommet réel (`max_z_height`
//! du G-code) et de la hauteur de 1ʳᵉ couche, forçant des couches fines (0.1 mm)
//! au-dessus de la première. Résultat : strictement **plus de couches** que sans
//! profil.
//!
//! Exécution : `cargo test -p engine --features ffi` (devShell requis).

#![cfg(feature = "ffi")]

mod common;

use engine::api::{CancelToken, Model, ProgressSink, SliceRequest};

/// Tranche et renvoie (nombre de couches, texte du G-code).
fn slice(model: Model, config: &engine::api::DynamicPrintConfig) -> (u32, String) {
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
    let gcode = std::fs::read_to_string(&result.gcode_path).expect("G-code lisible");
    (result.stats.layer_count, gcode)
}

/// Première valeur numérique après `marker` dans le G-code (en-tête / config).
fn gcode_number(gcode: &str, marker: &str) -> Option<f64> {
    gcode.lines().find_map(|l| {
        let rest = l.split(marker).nth(1)?;
        rest.trim()
            .trim_start_matches([':', '=', ' '])
            .split_whitespace()
            .next()?
            .trim_end_matches(';')
            .parse()
            .ok()
    })
}

#[test]
fn variable_layer_height_profile_changes_the_layer_count() {
    let (model, config) = engine::adapters::ffi::read_project_3mf(&common::fixture(
        "parity/bridge__bbl-a1-standard.3mf",
    ))
    .expect("lecture du projet 3MF");

    // Sans profil : couches uniformes du config.
    let (uniform_layers, gcode) = slice(model.clone(), &config);

    // Sommet réel de l'objet + hauteur de 1ʳᵉ couche (contrat de validation).
    let top = gcode_number(&gcode, "max_z_height").expect("max_z_height dans le G-code");
    let first = gcode_number(&gcode, "initial_layer_print_height")
        .expect("initial_layer_print_height dans le config du G-code");

    // Profil valide forçant 0.1 mm au-dessus de la première couche :
    // paires plates [z, h] — longueur paire, `profil[1] = first`, dernière cote = top.
    let mut varied = model;
    for object in &mut varied.objects {
        object.layer_height_profile = vec![0.0, first, top * 0.5, 0.1, top, 0.1];
    }
    let (varied_layers, _) = slice(varied, &config);

    assert!(
        varied_layers > uniform_layers,
        "le profil fin (0.1) doit produire plus de couches que l'uniforme : \
         obtenu {varied_layers} vs {uniform_layers} (top={top}, first={first})"
    );
}
