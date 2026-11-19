//! Lot 31 — données d'aperçu calculées depuis GCodeProcessor d'Orca, comparées
//! aux G-code de référence d'OrcaSlicer 2.4.1 :
//! - temps estimé à chaque mouvement (`get_estimated_time_at`) : total = en-tête
//!   du G-code, couches chaînées ;
//! - vue « Couleur » : changement de couleur M600 (`;COLOR_CHANGE,T0,#…`, balise
//!   réservée d'Orca), pauses / G-code personnalisés par couche ;
//! - coutures `EMoveType::Seam` = départ de chaque paroi extérieure du G-code ;
//! - tableau « Filament » (modèle / support / purge / tour) = pied du G-code
//!   (`filament used [mm]` de GCode.cpp).
//!
//! Exécution : cargo test -p engine --features ffi --test ffi_gcode_preview_data

#![cfg(feature = "ffi")]

use std::path::{Path, PathBuf};

use engine::adapters::ffi::process_gcode;
use engine::api::gcode_process::{col, move_type, ProcessedGcode};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// "1h 2m 3s" → secondes.
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

fn header_total(text: &str) -> f64 {
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("; estimated printing time (normal mode) = ") {
            return parse_dhms(rest);
        }
        if let Some(idx) = line.find("total estimated time: ") {
            return parse_dhms(&line[idx + 22..]);
        }
    }
    panic!("en-tête de temps absent");
}

/// `; <clé> = a, b` du pied de G-code.
fn footer_list(text: &str, key: &str) -> Vec<f64> {
    let prefix = format!("; {key} = ");
    text.lines()
        .rev()
        .find_map(|l| l.strip_prefix(&prefix))
        .map(|rest| rest.split(',').map(|v| v.trim().parse().unwrap()).collect())
        .unwrap_or_default()
}

#[test]
fn estimated_time_at_each_move_matches_orca_header() {
    for name in [
        "cube_bbl.gcode",
        "gcode-processor/cylinder.0-20mm-standard-voron.gcode",
        "custom_gcode/cube.orca.gcode",
    ] {
        let path = fixture(name);
        let text = std::fs::read_to_string(&path).unwrap();
        let p = process_gcode(&path).unwrap();
        let last = p.move_count() - 1;
        let total = p.meta["summary"]["modes"][0]["time"].as_f64().unwrap();
        let at_end = p.estimated_time_at(last, 0);
        // Temps au dernier sommet = temps total simulé (au mouvement 0 près,
        // sans sommet libvgcode) = en-tête écrit par Orca (arrondi à la s).
        let move0 = f64::from(p.f(0, col::TIME_NORMAL));
        assert!(
            (at_end + move0 - total).abs() < 1e-3 * total,
            "{name}: {at_end} vs {total}"
        );
        assert!(
            (at_end.round() - header_total(&text)).abs() <= 1.0,
            "{name}: {at_end} vs en-tête"
        );
        // Couches chaînées : elapsed(n+1) = elapsed(n) + Σ temps de la couche n.
        for w in p.layers.windows(2) {
            let sum: f64 = (w[0].first_move.max(1)..w[0].end_move)
                .map(|i| f64::from(p.f(i as usize, col::TIME_NORMAL)))
                .sum();
            assert!((w[0].elapsed[0] + sum - w[1].elapsed[0]).abs() < 1e-3);
        }
        // Identique au calcul direct (somme 1..=m) en tout point.
        let mut direct = 0f64;
        for m in 1..p.move_count() {
            direct += f64::from(p.f(m, col::TIME_NORMAL));
            if m % 97 == 0 {
                assert!((p.estimated_time_at(m, 0) - direct).abs() < 1e-3 * direct.max(1.0));
            }
        }
        assert_eq!(p.meta["layers"][1]["elapsed"][0], p.layers[1].elapsed[0]);
        eprintln!(
            "{name}: {} moves, total {total:.3}s, fin {at_end:.3}s",
            p.move_count()
        );
    }
}

/// G-code Orca du cube + changement de couleur au début de la couche z = 5,
/// sous la forme qu'écrit GCode.cpp (`;COLOR_CHANGE,T0,#00FF00` puis M600), à la
/// place exacte où Orca a écrit `;CUSTOM_GCODE` dans cube_custom.orca.gcode.
fn cube_with_color_change() -> (tempfile::TempDir, PathBuf) {
    let text = std::fs::read_to_string(fixture("custom_gcode/cube.orca.gcode")).unwrap();
    let mut out = String::with_capacity(text.len() + 64);
    let mut prev = "";
    let mut prev2 = "";
    let mut done = false;
    for line in text.lines() {
        out.push_str(line);
        out.push('\n');
        if !done
            && line == ";_SET_FAN_SPEED_CHANGING_LAYER"
            && prev == ";5"
            && prev2 == ";AFTER_LAYER_CHANGE"
        {
            out.push_str(";COLOR_CHANGE,T0,#00FF00\nM600\n");
            done = true;
        }
        prev2 = prev;
        prev = line;
    }
    assert!(done, "couche 5 introuvable");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cube_m600.gcode");
    std::fs::write(&path, out).unwrap();
    (dir, path)
}

#[test]
fn color_change_m600_feeds_the_color_print_view_like_orca() {
    let (_dir, path) = cube_with_color_change();
    let p = process_gcode(&path).unwrap();
    let cp = &p.meta["color_print"];
    eprintln!("color_print = {}", serde_json::to_string(cp).unwrap());
    // Plater::get_colors_for_color_print : couleurs des filaments (éditeur :
    // filament_colour de la config ; visionneuse : extruder_colors du
    // processeur, ici extruder_colour = #FF8000) puis une par changement.
    assert_eq!(cp["palette"], serde_json::json!(["#F2754E", "#00FF00"]));
    assert_eq!(
        cp["viewer_palette"],
        serde_json::json!(["#FF8000", "#00FF00"])
    );
    let markers = cp["markers"].as_array().unwrap();
    assert_eq!(markers.len(), 1);
    assert_eq!(markers[0]["type"], "color_change");
    let layer = markers[0]["layer"].as_u64().unwrap() as usize;
    assert!((p.layers[layer].z - 5.0).abs() < 1e-4, "couche {layer}");
    // Le changement de couleur ne grise pas la couche (seulement pause / perso).
    assert_eq!(cp["option_layers"], serde_json::json!([]));
    let custom = &cp["custom_gcodes"][0];
    assert_eq!(custom["type"], "color_change");
    assert_eq!(custom["color"], "#00FF00");
    assert_eq!(custom["layer"], layer);
    // m_used_extruders[0] : couleur 0 dès la 1re couche, puis couleur
    // extruders_count + 0 = 1 au changement.
    let items = cp["extruders"][0]["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["color_id"], 0);
    assert_eq!(items[0]["layer"], 0);
    assert_eq!(items[1]["color_id"], 1);
    assert_eq!(items[1]["layer"], layer);
    // cp_color_id par mouvement (octet 3 de WSPm) : 0 avant, 1 après.
    let marker_move = markers[0]["move"].as_u64().unwrap() as usize;
    // Exception fidèle à Orca : GCodeProcessor ré-émet après le réamorçage un
    // sommet d'une ligne antérieure (gcode_id < ligne du changement) qui garde
    // l'ancienne couleur — libvgcode le dessine ainsi, et get_color_prints ne
    // crée pas d'entrée pour lui (même couleur que la précédente).
    let marker_line = p.gcode_id(marker_move);
    let mut replayed = 0;
    for i in 0..p.move_count() {
        if p.move_type(i) == move_type::EXTRUDE {
            let cp = p.bytes[i * 4 + 3];
            if i < marker_move {
                assert_eq!(cp, 0, "mouvement {i}");
            } else if p.gcode_id(i) > marker_line {
                assert_eq!(cp, 1, "mouvement {i}");
            } else {
                replayed += 1;
            }
        }
    }
    assert!(replayed <= 1);
    // Temps du changement = temps cumulé au sommet (get_estimated_time_at).
    let t = items[1]["time"][0].as_f64().unwrap();
    let first_after = (marker_move..p.move_count())
        .find(|&i| p.move_type(i) == move_type::EXTRUDE)
        .unwrap();
    assert!((t - p.estimated_time_at(first_after, 0)).abs() < 1e-3);
}

#[test]
fn pause_and_custom_gcode_layers_like_orca() {
    // cube_custom : couleur (z = 5, émise en ;CUSTOM_GCODE par Orca 2.4.1 sur
    // cette machine), pause (z = 10), M117 (z = 15).
    let p = process_gcode(&fixture("custom_gcode/cube_custom.orca.gcode")).unwrap();
    let cp = &p.meta["color_print"];
    eprintln!("color_print = {}", serde_json::to_string(cp).unwrap());
    let markers = cp["markers"].as_array().unwrap();
    let types: Vec<&str> = markers
        .iter()
        .map(|m| m["type"].as_str().unwrap())
        .collect();
    assert_eq!(types, ["custom", "pause_print", "custom"]);
    let layers: Vec<usize> = markers
        .iter()
        .map(|m| m["layer"].as_u64().unwrap() as usize)
        .collect();
    // Couches de libvgcode : le déplacement inséré après la pause (id de la
    // couche précédente) ne crée pas de couche → 100 couches comme Orca.
    assert_eq!(p.layers.len(), 100);
    let zs: Vec<f32> = layers.iter().map(|&l| p.layers[l].z).collect();
    for (z, want) in zs.iter().zip([5.0f32, 10.0, 15.0]) {
        assert!((z - want).abs() < 1e-4, "{zs:?}");
    }
    assert_eq!(cp["option_layers"], serde_json::json!(layers));
    let custom_layers: Vec<usize> = cp["custom_gcodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["layer"].as_u64().unwrap() as usize)
        .collect();
    assert_eq!(custom_layers, layers);
    // Pas de changement de couleur : une seule couleur pour l'outil 0.
    assert_eq!(cp["extruders"][0]["items"].as_array().unwrap().len(), 1);
}

/// Départ (x, y) de chaque bloc `;TYPE:Outer wall` du texte (position avant la
/// première extrusion du bloc) : ce que l'aperçu devinait avant le lot 31.
fn outer_wall_starts(text: &str) -> Vec<(f64, f64)> {
    let (mut x, mut y) = (0.0, 0.0);
    let mut pending = false;
    let mut starts = Vec::new();
    for line in text.lines() {
        if line.starts_with(";TYPE:") {
            pending = line == ";TYPE:Outer wall";
            continue;
        }
        if line.starts_with("G1 ") || line.starts_with("G0 ") {
            let code = line.split(';').next().unwrap();
            let (mut nx, mut ny) = (x, y);
            let mut extrudes = false;
            for w in code.split_whitespace().skip(1) {
                let (k, v) = w.split_at(1);
                match k {
                    "X" => nx = v.parse().unwrap(),
                    "Y" => ny = v.parse().unwrap(),
                    "E" => extrudes = v.parse::<f64>().unwrap() > 0.0,
                    _ => {}
                }
            }
            if pending && extrudes {
                starts.push((x, y));
                pending = false;
            }
            x = nx;
            y = ny;
        }
    }
    starts
}

#[test]
fn seams_are_the_outer_wall_starts_of_orca_gcode() {
    let path = fixture("custom_gcode/cube.orca.gcode");
    let text = std::fs::read_to_string(&path).unwrap();
    let p: ProcessedGcode = process_gcode(&path).unwrap();
    let seams = p.seams(0, usize::MAX);
    let starts = outer_wall_starts(&text);
    eprintln!(
        "{} coutures, {} parois extérieures, {} couches",
        seams.len(),
        starts.len(),
        p.layers.len()
    );
    assert_eq!(
        seams.len(),
        starts.len(),
        "une couture par paroi extérieure"
    );
    assert_eq!(p.meta["seam_count"], seams.len());
    for (s, (x, y)) in seams.iter().zip(&starts) {
        // Seam = milieu du premier et du dernier sommet de la boucle (< 0,25 mm).
        let d = ((f64::from(s.x) - x).powi(2) + (f64::from(s.y) - y).powi(2)).sqrt();
        assert!(d < 0.25, "couture {s:?} vs départ ({x}, {y})");
        assert!((s.z - p.layers[s.layer as usize].z).abs() < 1e-3);
    }
}

/// Tableau « Filament » (GCodeViewer::render_legend) vs pied du G-code d'Orca :
/// total par filament = modèle + support + purge + tour simulés ; GCode.cpp
/// écrit `filament used [mm]` = longueur extrudée (pièce + tour, purge
/// comprise).
#[test]
fn filament_table_matches_orca_footer_for_multi_filament() {
    for name in [
        "multi_filament/bambu_x1c.orca.gcode",
        "multi_filament/mmu_painted.orca.gcode",
    ] {
        let path = fixture(name);
        let text = std::fs::read_to_string(&path).unwrap();
        let p = process_gcode(&path).unwrap();
        let t = &p.meta["filament_table"];
        let rows = t["rows"].as_array().unwrap();
        let mm = footer_list(&text, "filament used [mm]");
        let g = footer_list(&text, "filament used [g]");
        eprintln!("{name}: colonnes {}", t["columns"]);
        assert_eq!(rows.len(), mm.len(), "{name}");
        for (i, row) in rows.iter().enumerate() {
            let m = row["total"]["m"].as_f64().unwrap() * 1000.0;
            let grams = row["total"]["g"].as_f64().unwrap();
            eprintln!(
                "{name} T{i}: modèle {} g support {} g purge {} g tour {} g | total {m:.2} mm {grams:.2} g vs pied {} mm {} g",
                row["model"]["g"], row["support"]["g"], row["flushed"]["g"], row["tower"]["g"], mm[i], g[i]
            );
        }
        // Total de tous les filaments = Σ pied (longueur et masse).
        let m_all = t["total"]["total"]["m"].as_f64().unwrap() * 1000.0;
        let mm_all: f64 = mm.iter().sum();
        assert!(
            (m_all - mm_all).abs() <= 0.02 * rows.len() as f64,
            "{name}: {m_all} vs {mm_all}"
        );
        let g_all = t["total"]["total"]["g"].as_f64().unwrap();
        let g_footer: f64 = g.iter().sum();
        assert!(
            (g_all - g_footer).abs() <= 0.01 * rows.len() as f64,
            "{name}: {g_all} vs {g_footer}"
        );
        // Par filament : identique au pied quand la tour n'a pas de couches
        // creuses (X1C). Sur mmu_painted, GCode.cpp attribue les couches
        // creuses de la tour (wipe_tower_data.used_filament) autrement que la
        // simulation (≈ 70 mm passent de T0 à T1, total identique) : le
        // tableau suit GCodeProcessor, comme le tableau de la GUI d'Orca.
        if name.contains("x1c") {
            for (i, row) in rows.iter().enumerate() {
                let m = row["total"]["m"].as_f64().unwrap() * 1000.0;
                assert!(
                    (m - mm[i]).abs() <= 0.01,
                    "{name} T{i}: {m} mm vs {}",
                    mm[i]
                );
                let grams = row["total"]["g"].as_f64().unwrap();
                assert!(
                    (grams - g[i]).abs() <= 0.005,
                    "{name} T{i}: {grams} g vs {}",
                    g[i]
                );
            }
            assert_eq!(
                t["columns"],
                serde_json::json!({"model": true, "support": false, "flushed": true, "tower": true})
            );
        }
        assert_eq!(t["columns"]["model"], true);
    }
}
