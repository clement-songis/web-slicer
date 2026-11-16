//! « Remplir le plateau avec des instances » d'Orca (FillBedJob) via
//! libslic3r : cube20 seul sur le Voron 350 (config de
//! tests/fixtures/arrange_seq). Pas de référence CLI (fonction GUI) :
//! propriétés vérifiées = nombre attendu par la formule d'Orca, instances
//! dans le plateau, sans chevauchement, écart séquentiel respecté, et
//! résultat déterministe.
//! Exécution : cargo test -p engine --features ffi --test ffi_fill_bed.

#![cfg(feature = "ffi")]

use std::path::{Path, PathBuf};

use engine::api::{BuildVolume, ConfigValue, DynamicPrintConfig, Model};
use glam::DVec3;

fn root(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn setup(seq: bool) -> (Model, BuildVolume, DynamicPrintConfig) {
    let fixture = if seq { "by-object.3mf" } else { "by-layer.3mf" };
    let (_, config) = engine::adapters::ffi::read_project_3mf(&root(&format!(
        "tests/fixtures/arrange_seq/{fixture}"
    )))
    .unwrap();
    let model =
        engine::adapters::ffi::load_model(&root("../tests/oracle/fixtures/cube20.stl")).unwrap();
    let Some(ConfigValue::Points(points)) = config.get("printable_area") else {
        panic!()
    };
    let bed = BuildVolume {
        bed_shape: points.iter().map(|p| [p[0], p[1]]).collect(),
        max_height: 340.0,
        excluded: Vec::new(),
    };
    (model, bed, config)
}

fn boxes(model: &Model, matrices: &[glam::DMat4]) -> Vec<[f64; 4]> {
    let v = &model.objects[0].volumes[0];
    matrices
        .iter()
        .map(|inst| {
            let m = *inst * v.matrix;
            let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
            for p in &v.mesh.vertices {
                let w = m.transform_point3(DVec3::new(p[0].into(), p[1].into(), p[2].into()));
                b = [b[0].min(w.x), b[1].min(w.y), b[2].max(w.x), b[3].max(w.y)];
            }
            b
        })
        .collect()
}

fn min_gap(b: &[[f64; 4]]) -> f64 {
    let mut gap = f64::MAX;
    for i in 0..b.len() {
        for j in i + 1..b.len() {
            let gx = (b[j][0] - b[i][2]).max(b[i][0] - b[j][2]);
            let gy = (b[j][1] - b[i][3]).max(b[i][1] - b[j][3]);
            gap = gap.min(gx.max(gy));
        }
    }
    gap
}

#[test]
fn fills_the_bed_without_overlap_and_deterministically() {
    let (model, bed, config) = setup(false);
    let a = engine::adapters::ffi::fill_bed(&model, &bed, &config, 0, 0.0).unwrap();
    let b = engine::adapters::ffi::fill_bed(&model, &bed, &config, 0, 0.0).unwrap();
    assert_eq!(a, b, "déterministe");
    // Formule d'Orca : (aire plateau − aire fixe) / aire item = 350²/20² − 1 = 305,
    // plafonné à 100 items (au-delà Orca bascule sur une grille GUI).
    assert_eq!(a.len(), 100, "{} instances", a.len());
    let bx = boxes(&model, &a);
    for (i, b) in bx.iter().enumerate() {
        assert!(
            b[0] >= -1e-6 && b[1] >= -1e-6 && b[2] <= 350.0 + 1e-6 && b[3] <= 350.0 + 1e-6,
            "{i} hors plateau {b:?}"
        );
    }
    assert!(min_gap(&bx) >= -1e-6, "chevauchement : {}", min_gap(&bx));
}

#[test]
fn sequential_fill_keeps_the_extruder_clearance() {
    let (model, bed, config) = setup(true);
    let filled = engine::adapters::ffi::fill_bed(&model, &bed, &config, 0, 0.0).unwrap();
    assert!(
        filled.len() > 1 && filled.len() < 100,
        "{} instances",
        filled.len()
    );
    let Some(ConfigValue::Float(r)) = config.get("extruder_clearance_radius") else {
        panic!()
    };
    let gap = min_gap(&boxes(&model, &filled));
    assert!(gap >= *r - 1e-3, "écart {gap} < {r}");
}
