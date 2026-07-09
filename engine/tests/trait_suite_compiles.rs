//! Vérifie que la suite générique du trait compile et reste invocable
//! (T011). Les adaptateurs l'exécutent réellement : FFI en T019, CLI en
//! T020 (`common::trait_suite::run_all(&engine)`).

mod common;

use engine::SlicerEngine;

/// La suite doit rester utilisable sur tout `&dyn SlicerEngine`.
#[allow(dead_code)]
fn suite_est_generique(e: &dyn SlicerEngine) {
    common::trait_suite::run_all(e);
}

#[test]
fn fixtures_du_corpus_presentes() {
    for f in [
        "cube20.stl",
        "cube20.obj",
        "cube20.step",
        "orca_project.3mf",
        "manifest.json",
    ] {
        assert!(
            common::fixture(f).exists(),
            "fixture manquante : {f} (relancer engine/tests/fixtures/generate.py)"
        );
    }
}
