//! GCodeProcessor d'Orca via libslic3r : le temps simulé doit être celui
//! qu'Orca a écrit dans le G-code pendant le tranchage — en-tête « estimated
//! printing time (normal mode) » / « total estimated time », temps de première
//! couche, et chaque `M73 P<pct> R<min>` (progression le long du fichier,
//! donc des couches). 3 G-code de référence produits par OrcaSlicer 2.4.1.
//! Exécution : cargo test -p engine --features ffi --test ffi_gcode_process

#![cfg(feature = "ffi")]

use std::path::{Path, PathBuf};

use engine::adapters::ffi::process_gcode;
use engine::api::gcode_process::{col, move_type, ProcessedGcode};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// "1h 2m 3s" / "8m 41s" / "2s" / "0.960830s" → secondes.
fn parse_dhms(s: &str) -> f64 {
    s.split_whitespace()
        .map(|tok| {
            let (num, unit) = tok.split_at(tok.len() - 1);
            let v: f64 = num.parse().unwrap_or(0.0);
            match unit {
                "d" => v * 86400.0,
                "h" => v * 3600.0,
                "m" => v * 60.0,
                _ => v,
            }
        })
        .sum()
}

struct Header {
    total: f64,
    first_layer: Option<f64>,
    /// (ligne 1-based, pourcentage, minutes restantes)
    m73: Vec<(u32, u32, u32)>,
}

fn header(text: &str) -> Header {
    let mut total = None;
    let mut first_layer = None;
    let mut m73 = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if let Some(rest) = line.strip_prefix("; estimated printing time (normal mode) = ") {
            total = Some(parse_dhms(rest));
        } else if let Some(idx) = line.find("total estimated time: ") {
            total = Some(parse_dhms(&line[idx + 22..]));
        } else if let Some(rest) =
            line.strip_prefix("; estimated first layer printing time (normal mode) = ")
        {
            first_layer = Some(parse_dhms(rest));
        } else if let Some(rest) = line.strip_prefix("M73 P") {
            let mut it = rest.split_whitespace();
            let p = it.next().and_then(|v| v.parse().ok());
            let r = it
                .next()
                .and_then(|v| v.strip_prefix('R'))
                .and_then(|v| v.parse().ok());
            if let (Some(p), Some(r)) = (p, r) {
                m73.push((i as u32 + 1, p, r));
            }
        }
    }
    Header {
        total: total.expect("en-tête de temps"),
        first_layer,
        m73,
    }
}

/// Orca (TimeProcessor::post_process, GCodeProcessor.cpp:1181) : M73 inséré
/// après la ligne G1 quand le couple change ; P = int(100·écoulé/total) et
/// R = int((total − écoulé + 0,5) / 60) (time_in_minutes), en f32.
fn check(name: &str) -> (f64, f64, usize, usize) {
    let path = fixture(name);
    let text = std::fs::read_to_string(&path).unwrap();
    let h = header(&text);
    let p: ProcessedGcode = process_gcode(&path).expect("process");
    let total = p.meta["summary"]["modes"][0]["time"].as_f64().unwrap();
    // Orca écrit le temps arrondi à la seconde (get_time_dhms).
    assert!(
        (total.round() - h.total).abs() <= 1.0,
        "{name}: total {total} vs en-tête {}",
        h.total
    );
    let layer_sum: f64 = p.layers.iter().map(|l| f64::from(l.time[0])).sum();
    assert!(
        (layer_sum - total).abs() < 1e-3 * total.max(1.0),
        "{name}: Σ couches {layer_sum} vs {total}"
    );
    // Progression : chaque M73 doit correspondre au temps simulé écoulé.
    let mut exact = 0;
    for &(line, pct, rem) in &h.m73 {
        let elapsed = p.elapsed_before_line(line, 0) as f32;
        let total32 = total as f32;
        let our_pct = (100.0f32 * elapsed / total32) as i64;
        let our_rem = ((total32 - elapsed + 0.5) / 60.0) as i64;
        let dp = (our_pct - i64::from(pct)).abs();
        let dr = (our_rem - i64::from(rem)).abs();
        assert!(
            dp <= 1 && dr <= 1,
            "{name}: ligne {line} M73 P{pct} R{rem} vs P{our_pct} R{our_rem}"
        );
        if dp == 0 && dr == 0 {
            exact += 1;
        }
    }
    let first = f64::from(
        p.layers
            .iter()
            .find(|l| {
                (l.first_move..l.end_move).any(|i| p.move_type(i as usize) == move_type::EXTRUDE)
            })
            .map(|l| l.time[0])
            .unwrap_or(0.0),
    );
    let first_hdr = h.first_layer.unwrap_or(f64::NAN);
    // Orca (GCodeProcessor.cpp:1059) écrit sous ce libellé `machine.prepare_time`
    // (blocs « prepare_stage » : G-code de démarrage avant la 1re extrusion),
    // pas le temps de la couche 1 (`first_layer_time`, blocs layer_id == 1).
    let prepare = p.meta["summary"]["modes"][0]["prepare_time"]
        .as_f64()
        .unwrap();
    let tol = if first_hdr < 60.0 {
        1e-3_f64.max(first_hdr * 1e-5)
    } else {
        1.0
    };
    assert!(
        (prepare - first_hdr).abs() <= tol.max(0.5 * 10f64.powi(-6))
            || (prepare.round() - first_hdr).abs() < 1e-9,
        "{name}: prepare_time {prepare} vs en-tête 1re couche {first_hdr}"
    );
    // Couche 1 d'Orca : block.layer_id = max(1, m_layer_id) (GCodeProcessor.cpp:3935)
    // ⇔ MoveVertex.layer_id = max(1, m_layer_id) − 1 = 0 (store_move, :5601) :
    // démarrage + première couche imprimée.
    let layer1: f64 = (0..p.move_count())
        .filter(|&i| p.layer_id(i) == 0)
        .map(|i| f64::from(p.f(i, col::TIME_NORMAL)))
        .sum();
    let initial = p.meta["summary"]["initial_layer_time"].as_f64().unwrap();
    assert!(
        (layer1 - initial).abs() < 1e-3,
        "{name}: Σ couche 1 {layer1} vs initial_layer_time {initial}"
    );
    let actual_ok = (0..p.move_count())
        .filter(|&i| p.move_type(i) == move_type::EXTRUDE)
        .all(|i| p.f(i, col::ACTUAL_FEEDRATE) <= p.f(i, col::FEEDRATE) + 1e-3);
    assert!(actual_ok, "{name}: vitesse réelle > commandée");
    eprintln!(
        "{name}: total {total:.2}s (en-tête {:.0}s), {} couches, {} moves, M73 {}/{} exacts, \
         1re couche {first:.3}s, prepare {prepare:.6}s (en-tête « 1re couche » {first_hdr:.6}s), couche 1 {layer1:.3}s, initial_layer_time {}",
        h.total,
        p.layers.len(),
        p.move_count(),
        exact,
        h.m73.len(),
        p.meta["summary"]["initial_layer_time"]
    );
    (total, h.total, exact, h.m73.len())
}

#[test]
fn bbl_cube_times_match_orca_header_and_m73() {
    check("cube_bbl.gcode");
}

/// Indépendance à l'ordre : un G-code Voron traité avant le cube BBL (même
/// process) ne doit pas changer le résultat du cube (statique s_IsBBLPrinter).
#[test]
fn result_does_not_depend_on_previous_gcode() {
    let voron = fixture("gcode-processor/cylinder.0-20mm-standard-voron.gcode");
    process_gcode(&voron).unwrap();
    check("cube_bbl.gcode");
}

#[test]
fn voron_cylinder_times_match_orca_header_and_m73() {
    check("gcode-processor/cylinder.0-20mm-standard-voron.gcode");
}

#[test]
fn voron_overhang_times_match_orca_header_and_m73() {
    check("gcode-processor/overhang.0-28mm-extra-draft-voron.gcode");
}
