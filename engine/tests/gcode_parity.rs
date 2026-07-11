//! Parité de tranchage FFI ↔ OrcaSlicer desktop (T022, SC-003). Le tranchage
//! via le bridge FFI (`libslic3r-headless`) doit reproduire la sortie d'orca
//! desktop sur des **métriques indépendantes du build**.
//!
//! **Métrique de parité** : longueur de filament extrudé (`filament used [mm]`)
//! et nombre de couches. La longueur de filament est le volume physique
//! réellement déposé — identique dès lors que le tranchage l'est. Le **temps
//! estimé**, lui, dépend de la représentation *arc-fitting* propre à chaque
//! build de libslic3r (le statique headless et l'app desktop, tous deux 2.4.1,
//! découpent les arcs G2/G3 différemment) : à filament et couches identiques,
//! le temps estimé peut diverger jusqu'à ~5 % sur les prints à peu de couches.
//! Cet écart est **cosmétique** (cf. `specs/001-orcaslicer-web-parity/exclusions.md`,
//! EX-PARITY-TIME) ; on le rapporte comme garde-fou large, sans en faire le
//! critère de parité.
//!
//! **Corpus** : 45 cas (9 modèles imprimables × 5 presets, cf.
//! `fixtures/manifest.json`). Chaque cas est un 3MF au config résolu (héritage
//! OrcaSlicer aplati) produit par `fixtures/generate_parity_corpus.py`, tranché
//! hors-ligne par `orca-slicer` pour enregistrer ses métriques desktop dans
//! `fixtures/parity/references.json` (committé). Le 10ᵉ modèle
//! (`simplification.stl`, 0.59×0.35×0.16 mm) est exclu : orca lui-même le juge
//! non imprimable (cf. exclusions.md, EX-PARITY-SIMPLIFY).
//!
//! Exécution : `cargo test -p engine --features ffi --test gcode_parity`.
#![cfg(feature = "ffi")]

mod common;

use std::collections::BTreeMap;
use std::f64::consts::PI;

use engine::api::{CancelToken, ConfigValue, DynamicPrintConfig, ProgressSink, SliceRequest};

/// Tolérance de parité sur la longueur de filament (métrique SC-003, < 0,5 %).
const FILAMENT_TOLERANCE: f64 = 0.005;
/// Garde-fou large sur le temps estimé : borne la variance arc-fitting entre
/// builds (max observé ~4,6 %), pas un critère de parité. Piège les régressions
/// grossières sans se coupler au bruit de représentation.
const TIME_TRIPWIRE: f64 = 0.08;
/// Diamètre de filament par défaut (mm) si absent du config.
const DEFAULT_FILAMENT_DIAMETER: f64 = 1.75;

/// Une entrée de `references.json` : métriques desktop enregistrées.
#[derive(serde::Deserialize)]
struct CaseRef {
    model: String,
    preset_id: String,
    layers: u32,
    time_s: f64,
    filament_mm: f64,
}

#[derive(serde::Deserialize)]
struct References {
    cases: BTreeMap<String, CaseRef>,
}

/// Métriques desktop de référence, homogènes entre le cas nominal et le corpus.
struct Reference {
    layers: u32,
    time_s: f64,
    filament_mm: f64,
}

#[test]
fn ffi_gcode_matches_orca_desktop_reference() {
    let mut failures: Vec<String> = Vec::new();

    check_nominal(&mut failures);
    check_corpus(&mut failures);

    assert!(
        failures.is_empty(),
        "{} cas hors tolérance :\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Cas nominal : 3MF projet défaut vs G-code desktop enregistré (`cube_bbl.gcode`).
fn check_nominal(failures: &mut Vec<String>) {
    let gcode =
        std::fs::read_to_string(common::fixture("cube_bbl.gcode")).expect("G-code de référence");
    let reference = parse_reference_gcode(&gcode);
    let ffi = slice_via_ffi("orca_project.3mf");
    compare("orca_project.3mf (nominal)", &ffi, &reference, failures);
}

/// Corpus de parité : chaque 3MF résolu vs ses métriques desktop enregistrées.
fn check_corpus(failures: &mut Vec<String>) {
    let raw = std::fs::read_to_string(common::fixture("parity/references.json"))
        .expect("references.json du corpus lisible");
    let refs: References = serde_json::from_str(&raw).expect("references.json valide");
    assert_eq!(
        refs.cases.len(),
        45,
        "le corpus doit compter 45 cas imprimables"
    );

    for (case, r) in &refs.cases {
        let ffi = slice_via_ffi(&format!("parity/{case}.3mf"));
        let reference = Reference {
            layers: r.layers,
            time_s: r.time_s,
            filament_mm: r.filament_mm,
        };
        compare(
            &format!("{} [{}]", r.model, r.preset_id),
            &ffi,
            &reference,
            failures,
        );
    }
}

/// Métriques produites par le FFI pour un projet.
struct FfiMetrics {
    time_s: f64,
    layers: u32,
    filament_mm: f64,
}

/// Compare FFI vs référence ; consigne les écarts hors tolérance.
fn compare(label: &str, ffi: &FfiMetrics, r: &Reference, failures: &mut Vec<String>) {
    // Filament : métrique de parité de tranchage (volume physique déposé).
    let fil_drift = (ffi.filament_mm - r.filament_mm).abs() / r.filament_mm;
    // Couches : tolérance ±1 (le stat FFI compte les couches d'objet, orca les
    // couches d'impression — convention d'une unité, l'en-tête G-code concorde).
    let layer_gap = ffi.layers.abs_diff(r.layers);
    // Temps estimé : rapporté, borné large (variance arc-fitting inter-build).
    let time_drift = (ffi.time_s - r.time_s).abs() / r.time_s;
    eprintln!(
        "{label}: filament FFI {:.1} vs desktop {:.1} mm (écart {:.3} %) | \
         couches {}/{} | temps {:.0}/{:.0}s (écart {:.2} %)",
        ffi.filament_mm,
        r.filament_mm,
        fil_drift * 100.0,
        ffi.layers,
        r.layers,
        ffi.time_s,
        r.time_s,
        time_drift * 100.0
    );
    if fil_drift >= FILAMENT_TOLERANCE {
        failures.push(format!(
            "{label}: filament FFI {:.2} vs desktop {:.2} mm (écart {:.3} % ≥ {:.1} %)",
            ffi.filament_mm,
            r.filament_mm,
            fil_drift * 100.0,
            FILAMENT_TOLERANCE * 100.0
        ));
    }
    if layer_gap > 1 {
        failures.push(format!(
            "{label}: couches FFI {} vs desktop {} (écart {layer_gap} > 1)",
            ffi.layers, r.layers
        ));
    }
    if time_drift >= TIME_TRIPWIRE {
        failures.push(format!(
            "{label}: temps FFI {:.0} vs desktop {:.0}s (écart {:.2} % ≥ garde-fou {:.0} %)",
            ffi.time_s,
            r.time_s,
            time_drift * 100.0,
            TIME_TRIPWIRE * 100.0
        ));
    }
}

/// Tranche un projet via le worker FFI et renvoie ses métriques.
fn slice_via_ffi(project: &str) -> FfiMetrics {
    let (model, config) = engine::adapters::ffi::read_project_3mf(&common::fixture(project))
        .expect("lecture du projet 3MF");
    let diameter = filament_diameter(&config);
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
    // `stats.filament_mm` est le volume déposé (mm³) ; on le ramène en longueur
    // pour comparer à `filament used [mm]` d'orca : L = V / (π·(d/2)²).
    let volume_mm3: f64 = result.stats.filament_mm.iter().sum();
    let area = PI * (diameter / 2.0).powi(2);
    FfiMetrics {
        time_s: result.stats.estimated_time_s,
        layers: result.stats.layer_count as u32,
        filament_mm: volume_mm3 / area,
    }
}

/// Diamètre de filament (mm) du premier extrudeur, depuis le config résolu.
fn filament_diameter(config: &DynamicPrintConfig) -> f64 {
    match config.0.get("filament_diameter") {
        Some(ConfigValue::Floats(v)) => v.first().copied(),
        Some(ConfigValue::Float(x)) => Some(*x),
        _ => None,
    }
    .filter(|d| *d > 0.0)
    .unwrap_or(DEFAULT_FILAMENT_DIAMETER)
}

/// Extrait les métriques desktop d'un G-code de référence (cas nominal).
fn parse_reference_gcode(gcode: &str) -> Reference {
    let time_s = gcode
        .lines()
        .find_map(|l| l.split("total estimated time:").nth(1))
        .map(parse_duration_s)
        .expect("« total estimated time » présent");
    let layers = gcode
        .lines()
        .find_map(|l| {
            l.trim_start_matches("; ")
                .strip_prefix("total layer number: ")
        })
        .and_then(|s| s.trim().parse().ok())
        .expect("« total layer number » présent");
    let filament_mm = gcode
        .lines()
        .find_map(|l| l.split("filament used [mm]").nth(1))
        .and_then(|s| s.trim_start_matches([' ', '=']).trim().parse().ok())
        .expect("« filament used [mm] » présent");
    Reference {
        layers,
        time_s,
        filament_mm,
    }
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
