//! Arrangement séquentiel (objet par objet) piloté par la config, comme le
//! CLI Orca : `orca-slicer --arrange 1 --orient 0 --export-3mf` sur cube20 +
//! cylinder + bridge (tests/fixtures/arrange_seq, make_ref.py, 5/5 tirages
//! identiques). Positions monde de chaque objet comparées à 1e-3 mm.
//! Exécution : cargo test -p engine --features ffi --test ffi_arrange_seq.

#![cfg(feature = "ffi")]

use std::path::{Path, PathBuf};

use engine::api::{ArrangeParams, BuildVolume, ConfigValue, DynamicPrintConfig, Model};
use glam::DVec3;

fn oracle(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../tests/oracle/fixtures/{name}.stl"))
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/arrange_seq/{name}"))
}

/// Boîte XY monde (instance × volume) de chaque objet, triée par nom (Orca
/// réordonne les objets du 3MF par ordre d'arrangement, PartPlate.cpp:6102).
fn world_boxes(model: &Model) -> Vec<[f64; 4]> {
    let mut objects: Vec<_> = model.objects.iter().collect();
    objects.sort_by(|a, b| a.name.cmp(&b.name));
    objects
        .into_iter()
        .map(|o| {
            let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
            let inst = o.instances[0].matrix;
            for v in &o.volumes {
                let m = inst * v.matrix;
                for p in &v.mesh.vertices {
                    let w = m.transform_point3(DVec3::new(p[0].into(), p[1].into(), p[2].into()));
                    b = [b[0].min(w.x), b[1].min(w.y), b[2].max(w.x), b[3].max(w.y)];
                }
            }
            b
        })
        .collect()
}

fn bed_of(config: &DynamicPrintConfig) -> BuildVolume {
    let Some(ConfigValue::Points(points)) = config.get("printable_area") else {
        panic!("printable_area absent");
    };
    BuildVolume {
        bed_shape: points.iter().map(|p| [p[0], p[1]]).collect(),
        max_height: 340.0,
        excluded: Vec::new(),
    }
}

type Arranged = (
    Vec<[f64; 4]>,
    Vec<[f64; 4]>,
    Vec<Option<u32>>,
    DynamicPrintConfig,
    Vec<String>,
    Vec<String>,
);

fn arranged(reference: &str) -> Arranged {
    let (orca, config) = engine::adapters::ffi::read_project_3mf(&fixture(reference)).unwrap();
    let mut model = Model::default();
    for name in ["cube20", "cylinder", "bridge"] {
        let loaded = engine::adapters::ffi::load_model(&oracle(name)).unwrap();
        model.objects.extend(loaded.objects);
    }
    let params = ArrangeParams {
        clearance: 0.0,
        use_machine_clearance: true,
        allow_rotations: true, // défaut du CLI (OrcaSlicer.cpp:1439)
        orca_cli_mode: true,
        print_config: Some(config.clone()),
        is_seq_print: false,
    };
    let orders =
        engine::adapters::ffi::arrange_ordered(&mut model, &bed_of(&config), &params).unwrap();
    // Noms dans l'ordre d'impression moteur / dans l'ordre des objets du 3MF Orca.
    let mut by_order: Vec<(Option<u32>, String)> = orders
        .iter()
        .zip(&model.objects)
        .map(|(o, obj)| (*o, obj.name.clone()))
        .collect();
    by_order.sort();
    let ours_order = by_order.into_iter().map(|(_, n)| n).collect();
    let orca_order = orca.objects.iter().map(|o| o.name.clone()).collect();
    (
        world_boxes(&model),
        world_boxes(&orca),
        orders,
        config,
        ours_order,
        orca_order,
    )
}

fn assert_close(ours: &[[f64; 4]], orca: &[[f64; 4]]) {
    assert_eq!(ours.len(), orca.len());
    for (i, (a, b)) in ours.iter().zip(orca).enumerate() {
        for k in 0..4 {
            assert!(
                (a[k] - b[k]).abs() < 1e-3,
                "objet {i} : moteur {a:?} / Orca {b:?}"
            );
        }
    }
}

#[test]
fn sequential_arrangement_matches_orca_cli() {
    let (ours, orca, orders, config, ours_order, orca_order) = arranged("by-object.3mf");
    // Orca range les objets du plateau par arrange_order (PartPlate.cpp:6102).
    assert_eq!(ours_order, orca_order, "ordre d'impression");
    assert!(
        matches!(config.get("print_sequence"), Some(ConfigValue::String(s)) if s == "by object")
    );
    assert_close(&ours, &orca);
    // Ordre d'impression : permutation de 1..3.
    let mut o: Vec<u32> = orders
        .iter()
        .map(|x| x.expect("ordre séquentiel"))
        .collect();
    o.sort();
    assert_eq!(o, vec![1, 2, 3]);
    // Dégagement : boîtes gonflées de clearance_radius / 2 sans chevauchement.
    let Some(ConfigValue::Float(r)) = config.get("extruder_clearance_radius") else {
        panic!()
    };
    for i in 0..ours.len() {
        for j in i + 1..ours.len() {
            let (a, b) = (ours[i], ours[j]);
            let gap_x = (b[0] - a[2]).max(a[0] - b[2]);
            let gap_y = (b[1] - a[3]).max(a[1] - b[3]);
            assert!(
                gap_x.max(gap_y) >= *r - 1e-3,
                "objets {i}/{j} à {} mm < {r}",
                gap_x.max(gap_y)
            );
        }
    }
}

#[test]
fn layer_by_layer_arrangement_with_config_matches_orca_cli() {
    let (ours, orca, orders, _, _, _) = arranged("by-layer.3mf");
    assert_close(&ours, &orca);
    assert!(
        orders.iter().all(Option::is_none),
        "pas d'ordre hors séquentiel"
    );
}
