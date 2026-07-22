//! Vérification bout-en-bout de la peinture au triangle (T056+) : une peinture de
//! **support enforcer** injectée via `PaintedFacets` doit réellement produire du
//! support dans le G-code — le canal remonte jusqu'à `libslic3r` par le bridge
//! FFI (`FacetsAnnotation::set_triangle_from_string`, cf. `project.cpp`).
//!
//! Isolation stricte : `support_type = normal(manual)` désactive la détection
//! automatique d'overhang, donc **seuls** les enforcers peints génèrent du
//! support. Contrôle sans peinture → aucun support ; avec peinture → support.
//!
//! Exécution : `cargo test -p engine --features ffi` (devShell requis).

#![cfg(feature = "ffi")]

mod common;

use engine::api::{CancelToken, ConfigValue, Model, ProgressSink, SliceRequest};

/// Compte les segments de support dans un G-code OrcaSlicer (commentaire de type
/// de trait `; FEATURE: Support` / `Support interface`).
fn support_feature_lines(gcode: &str) -> usize {
    gcode
        .lines()
        .filter(|l| l.contains("FEATURE: Support") || l.contains("FEATURE: Support interface"))
        .count()
}

/// Tranche un modèle avec le config donné et renvoie le texte du G-code.
fn slice_to_gcode(model: Model, config: &engine::api::DynamicPrintConfig) -> String {
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

/// Peint tout le volume-pièce de chaque objet en enforcer de support (état "4",
/// valeur Orca de l'enforcer, un triangle entier non subdivisé).
fn paint_all_as_support_enforcer(model: &mut Model) {
    for object in &mut model.objects {
        for volume in &mut object.volumes {
            let triangle_count = volume.mesh.indices.len();
            volume.painted.supports = (0..triangle_count as u32)
                .map(|t| (t, "4".into()))
                .collect();
        }
    }
}

#[test]
fn painted_support_enforcer_generates_support_in_gcode() {
    // Modèle à surplombs (deux piliers + travée) au config résolu réel.
    let (model, mut config) = engine::adapters::ffi::read_project_3mf(&common::fixture(
        "parity/bridge__bbl-a1-standard.3mf",
    ))
    .expect("lecture du projet 3MF");

    // Support activé, mode **manuel** : seuls les enforcers peints comptent.
    config
        .set("enable_support", ConfigValue::Bool(true))
        .unwrap();
    config
        .set("support_type", ConfigValue::String("normal(manual)".into()))
        .unwrap();

    // Contrôle : aucune peinture → aucun support (mode manuel).
    let control = slice_to_gcode(model.clone(), &config);
    let control_support = support_feature_lines(&control);

    // Peint : enforcer sur tout le modèle → support généré sous les surplombs.
    let mut painted_model = model;
    paint_all_as_support_enforcer(&mut painted_model);
    let painted = slice_to_gcode(painted_model, &config);
    let painted_support = support_feature_lines(&painted);

    assert_eq!(
        control_support, 0,
        "mode manuel sans peinture : aucun support attendu (obtenu {control_support})"
    );
    assert!(
        painted_support > 0,
        "l'enforcer peint doit générer du support (obtenu {painted_support})"
    );
}
