//! Vérification bout-en-bout de la peinture de **couture** (seam, T141) : une
//! peinture du canal `seam` doit déplacer la couture — le canal remonte jusqu'à
//! `libslic3r` par le bridge FFI (`ModelVolume::seam_facets`, cf. `project.cpp`,
//! même chemin que le lecteur 3MF).
//!
//! Sans réglage à activer (contrairement aux supports/fuzzy) : `SeamPlacer` lit
//! les enforcers dès que `is_seam_painted()`. Preuve : enforcer la couture d'un
//! **côté** puis de l'**autre** produit des parcours différents (la couture suit
//! la zone peinte). Un no-op donnerait des parcours identiques.
//!
//! Exécution : `cargo test -p engine --features ffi` (devShell requis).

#![cfg(feature = "ffi")]

mod common;

use engine::api::{CancelToken, Model, ProgressSink, SliceRequest};

/// Séquence des mouvements planaires `G1 X…` (parcours), hors commentaires (donc
/// hors en-tête horodaté) — deux tranchages identiques donnent la même séquence.
fn toolpath(gcode: &str) -> Vec<&str> {
    gcode
        .lines()
        .filter(|l| l.starts_with("G1 ") && l.contains('X'))
        .collect()
}

fn slice_toolpath(model: Model, config: &engine::api::DynamicPrintConfig) -> Vec<String> {
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
    toolpath(&gcode).into_iter().map(str::to_owned).collect()
}

/// Abscisse médiane des sommets d'un volume (plan de séparation gauche/droite).
fn mid_x(volume: &engine::api::ModelVolume) -> f32 {
    let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
    for v in &volume.mesh.vertices {
        lo = lo.min(v[0]);
        hi = hi.max(v[0]);
    }
    (lo + hi) / 2.0
}

/// Peint la couture (enforcer « 4 ») sur les triangles dont le barycentre est du
/// côté demandé du plan médian (`right` = x > milieu).
fn paint_seam_side(model: &Model, right: bool) -> Model {
    let mut out = model.clone();
    for object in &mut out.objects {
        for volume in &mut object.volumes {
            let mid = mid_x(volume);
            let mut facets = Vec::new();
            for (t, tri) in volume.mesh.indices.iter().enumerate() {
                let cx = tri
                    .iter()
                    .map(|&i| volume.mesh.vertices[i as usize][0])
                    .sum::<f32>()
                    / 3.0;
                if (cx > mid) == right {
                    facets.push((t as u32, "4".to_string()));
                }
            }
            volume.painted.seam = facets;
        }
    }
    out
}

#[test]
fn painted_seam_moves_the_seam() {
    let (model, config) = engine::adapters::ffi::read_project_3mf(&common::fixture(
        "parity/bridge__bbl-a1-standard.3mf",
    ))
    .expect("lecture du projet 3MF");

    let plain = slice_toolpath(model.clone(), &config);
    let seam_right = slice_toolpath(paint_seam_side(&model, true), &config);
    let seam_left = slice_toolpath(paint_seam_side(&model, false), &config);

    // La couture peinte modifie le parcours, et enforcer à gauche ≠ à droite :
    // preuve que la couture **suit** la zone peinte (sinon tout serait identique).
    assert_ne!(
        seam_right, plain,
        "enforcer la couture à droite doit changer le parcours"
    );
    assert_ne!(
        seam_right, seam_left,
        "enforcer la couture à droite vs à gauche doit produire des parcours différents"
    );
}
