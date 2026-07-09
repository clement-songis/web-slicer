//! Tranchage FFI de bout en bout (T019) : projet 3MF réel → G-code + stats,
//! progression reçue par le pipe du worker. Exécution :
//! `cargo test -p engine --features ffi --test slice`.

#![cfg(feature = "ffi")]

mod common;

use std::sync::{Arc, Mutex};

use engine::api::{CancelToken, ProgressSink, SliceRequest};

#[test]
fn tranche_un_projet_reel_avec_progression() {
    // Projet OrcaSlicer complet (cube 20 mm + config projet embarquée).
    let (model, config) =
        engine::adapters::ffi::read_project_3mf(&common::fixture("orca_project.3mf"))
            .expect("lecture du projet 3MF");

    let work = tempfile::tempdir().unwrap();
    let req = SliceRequest {
        model,
        config,
        plate_index: 0,
        work_dir: work.path().to_path_buf(),
    };

    let seen = Arc::new(Mutex::new(Vec::<f32>::new()));
    let sink_seen = Arc::clone(&seen);
    let progress: ProgressSink = Box::new(move |_phase, ratio| {
        sink_seen.lock().unwrap().push(ratio);
    });

    let result = engine::adapters::ffi::slice(req, progress, CancelToken::new())
        .expect("le tranchage aboutit");

    assert!(
        result.gcode_path.exists(),
        "le G-code est écrit : {}",
        result.gcode_path.display()
    );
    let gcode = std::fs::read_to_string(&result.gcode_path).unwrap();
    assert!(gcode.contains('G'), "le fichier contient bien du G-code");

    assert!(
        result.stats.estimated_time_s > 0.0,
        "temps estimé renseigné : {:?}",
        result.stats
    );
    assert!(
        result.stats.layer_count > 0,
        "au moins une couche : {:?}",
        result.stats
    );

    // Le statusbar de libslic3r n'est pas strictement monotone (les sous-
    // étapes rapportent leur propre pourcentage) ; on vérifie seulement que la
    // progression est remontée par le pipe et qu'elle avance nettement. La
    // garantie de monotonie du protocole lui-même est testée en T018.
    let ratios = seen.lock().unwrap().clone();
    assert!(
        !ratios.is_empty(),
        "la progression du statusbar est remontée par le pipe"
    );
    let max = ratios.iter().cloned().fold(0.0_f32, f32::max);
    assert!(max >= 0.5, "la progression avance nettement : max={max}");
}
