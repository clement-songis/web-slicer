//! Parité G-code FFI ↔ OrcaSlicer desktop (T022, SC-003). Le tranchage via le
//! bridge FFI (`libslic3r-headless`) doit reproduire la sortie d'orca desktop :
//! mêmes métadonnées normalisées, temps estimé à moins de 1 %.
//!
//! **Périmètre de cette passe** : le cas nominal du corpus (cube 20 mm ×
//! `bbl-a1-standard`), pour lequel la sortie orca desktop est enregistrée dans
//! `fixtures/cube_bbl.gcode` (généré par OrcaSlicer 2.4.1). Le harnais est
//! générique : les 49 combinaisons restantes (10 modèles × 5 presets, cf.
//! `fixtures/manifest.json`) s'ajoutent en enregistrant leur G-code de référence
//! desktop et en étendant `CASES`. La génération du corpus complet reste un
//! prérequis d'infrastructure (références desktop enregistrées, committées).
//!
//! Exécution : `cargo test -p engine --features ffi --test gcode_parity`.
#![cfg(feature = "ffi")]

mod common;

use engine::api::{CancelToken, ProgressSink, SliceRequest};

/// Un cas de parité : projet 3MF (modèle + config embarquée) et sa référence
/// desktop enregistrée. À terme, un cas par (modèle, preset) du manifeste.
struct Case {
    /// Projet OrcaSlicer (scène + réglages) tranché par le FFI.
    project: &'static str,
    /// G-code de référence produit par orca desktop (mêmes modèle + réglages).
    reference: &'static str,
}

const CASES: &[Case] = &[Case {
    // Cube 20 mm, preset `bbl-a1-standard` (0.20 mm Standard @BBL A1).
    project: "orca_project.3mf",
    reference: "cube_bbl.gcode",
}];

/// Tolérance de parité sur le temps estimé (SC-003 : < 1 %).
const TIME_TOLERANCE: f64 = 0.01;

#[test]
fn ffi_gcode_matches_orca_desktop_reference() {
    for case in CASES {
        let (ffi_time_s, ffi_layers) = slice_via_ffi(case.project);
        let reference = std::fs::read_to_string(common::fixture(case.reference))
            .expect("G-code de référence lisible");
        let (ref_time_s, ref_layers) = parse_reference_metrics(&reference);

        // Nombre de couches : tolérance ±1. OrcaSlicer desktop compte
        // « total layer number » (couches d'impression, ici 100) ; les stats FFI
        // rapportent le nombre de couches d'objet (99) — une différence de
        // convention de comptage d'une unité, pas un écart de tranchage.
        let layer_gap = ffi_layers.abs_diff(ref_layers);
        assert!(
            layer_gap <= 1,
            "{}: couches FFI {ffi_layers} vs référence desktop {ref_layers} \
             (écart {layer_gap} > 1)",
            case.project
        );

        // Temps estimé : métrique de parité SC-003, écart relatif sous le seuil.
        let drift = (ffi_time_s - ref_time_s).abs() / ref_time_s;
        eprintln!(
            "{}: FFI {ffi_time_s:.0}s / {ffi_layers} couches vs desktop \
             {ref_time_s:.0}s / {ref_layers} couches (écart temps {:.3} %)",
            case.project,
            drift * 100.0
        );
        assert!(
            drift < TIME_TOLERANCE,
            "{}: temps estimé FFI {ffi_time_s:.0}s vs desktop {ref_time_s:.0}s \
             (écart {:.2} % ≥ {:.0} %)",
            case.project,
            drift * 100.0,
            TIME_TOLERANCE * 100.0
        );
    }
}

/// Tranche un projet via le worker FFI et renvoie (temps estimé s, couches).
fn slice_via_ffi(project: &str) -> (f64, u32) {
    let (model, config) = engine::adapters::ffi::read_project_3mf(&common::fixture(project))
        .expect("lecture du projet 3MF");
    let work = tempfile::tempdir().unwrap();
    let req = SliceRequest {
        model,
        config,
        plate_index: 0,
        work_dir: work.path().to_path_buf(),
    };
    let progress: ProgressSink = Box::new(|_phase, _ratio| {});
    let result = engine::adapters::ffi::slice(req, progress, CancelToken::new())
        .expect("le tranchage FFI aboutit");
    (
        result.stats.estimated_time_s,
        result.stats.layer_count as u32,
    )
}

/// Extrait (temps estimé total en s, nombre de couches) de l'en-tête OrcaSlicer.
fn parse_reference_metrics(gcode: &str) -> (f64, u32) {
    // `; model printing time: 29m 54s; total estimated time: 29m 56s`
    let time_s = gcode
        .lines()
        .find_map(|l| l.split("total estimated time:").nth(1))
        .map(parse_duration_s)
        .expect("« total estimated time » présent dans la référence");
    // `; total layer number: 100`
    let layers = gcode
        .lines()
        .find_map(|l| {
            l.trim_start_matches("; ")
                .strip_prefix("total layer number: ")
        })
        .and_then(|s| s.trim().parse().ok())
        .expect("« total layer number » présent dans la référence");
    (time_s, layers)
}

/// Convertit une durée OrcaSlicer (`1h 2m 3s`, `29m 56s`, `45s`) en secondes.
fn parse_duration_s(fragment: &str) -> f64 {
    let mut total = 0.0;
    for token in fragment.split_whitespace() {
        let (value, unit) = token.split_at(token.len().saturating_sub(1));
        let scale = match unit {
            "h" => 3600.0,
            "m" => 60.0,
            "s" => 1.0,
            _ => break, // fin du champ durée (autre métadonnée sur la même ligne)
        };
        match value.parse::<f64>() {
            Ok(n) => total += n * scale,
            Err(_) => break,
        }
    }
    total
}
