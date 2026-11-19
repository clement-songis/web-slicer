//! Arrangement de la GUI d'Orca (ArrangeJob + ArrangeSettings du menu
//! « Options d'arrangement ») via libslic3r : cube20 sur le Voron 350 (config
//! de tests/fixtures/arrange_seq). Pas de référence CLI (fonction GUI) :
//! propriétés vérifiées = espacement demandé respecté, création de plateaux
//! au-delà de la capacité (postprocess_bed_index_for_selected), plateaux
//! verrouillés intacts, portée « ce plateau » (postprocess_bed_index_for_
//! current_plate), instances non imprimables sur le dernier plateau
//! (finalize), alignement Y, déterminisme.
//! Exécution : cargo test -p engine --features ffi --test ffi_arrange_gui.

#![cfg(feature = "ffi")]

use std::path::{Path, PathBuf};

use engine::api::{BuildVolume, ConfigValue, DynamicPrintConfig, GuiArrangeSettings, Model};
use glam::DVec3;

fn root(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn setup(count: usize, seq: bool) -> (Model, BuildVolume, DynamicPrintConfig) {
    let fixture = if seq { "by-object.3mf" } else { "by-layer.3mf" };
    let (_, config) = engine::adapters::ffi::read_project_3mf(&root(&format!(
        "tests/fixtures/arrange_seq/{fixture}"
    )))
    .unwrap();
    let cube =
        engine::adapters::ffi::load_model(&root("../tests/oracle/fixtures/cube20.stl")).unwrap();
    let mut model = Model::default();
    for i in 0..count {
        let mut object = cube.objects[0].clone();
        object.name = format!("cube{i}");
        model.objects.push(object);
    }
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

fn settings(distance: f64) -> GuiArrangeSettings {
    GuiArrangeSettings {
        distance,
        allow_multi_materials_on_same_plate: true,
        avoid_extrusion_cali_region: true,
        plate_count: 1,
        ..Default::default()
    }
}

/// Boîte XY (repère local du plateau) de chaque instance.
fn boxes(model: &Model) -> Vec<[f64; 4]> {
    model
        .objects
        .iter()
        .flat_map(|o| {
            o.instances.iter().map(move |inst| {
                let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
                for v in &o.volumes {
                    let m = inst.matrix * v.matrix;
                    for p in &v.mesh.vertices {
                        let w =
                            m.transform_point3(DVec3::new(p[0].into(), p[1].into(), p[2].into()));
                        b = [b[0].min(w.x), b[1].min(w.y), b[2].max(w.x), b[3].max(w.y)];
                    }
                }
                b
            })
        })
        .collect()
}

fn plates(model: &Model) -> Vec<u32> {
    model
        .objects
        .iter()
        .flat_map(|o| o.instances.iter().map(|i| i.plate_index))
        .collect()
}

fn min_gap_same_plate(b: &[[f64; 4]], plates: &[u32]) -> f64 {
    let mut gap = f64::MAX;
    for i in 0..b.len() {
        for j in i + 1..b.len() {
            if plates[i] != plates[j] {
                continue;
            }
            let gx = (b[j][0] - b[i][2]).max(b[i][0] - b[j][2]);
            let gy = (b[j][1] - b[i][3]).max(b[i][1] - b[j][3]);
            gap = gap.min(gx.max(gy));
        }
    }
    gap
}

#[test]
fn spacing_is_respected_inside_the_bed_and_deterministic() {
    let (mut a, bed, config) = setup(6, false);
    let mut b = a.clone();
    let out = engine::adapters::ffi::arrange_gui(&mut a, &bed, &config, &settings(15.0)).unwrap();
    engine::adapters::ffi::arrange_gui(&mut b, &bed, &config, &settings(15.0)).unwrap();
    assert_eq!(boxes(&a), boxes(&b), "déterministe");
    assert_eq!(out.plate_count, 1);
    assert!(out.unarranged.iter().all(|u| !u));
    let bx = boxes(&a);
    for r in &bx {
        assert!(r[0] >= -1e-6 && r[1] >= -1e-6 && r[2] <= 350.0 + 1e-6 && r[3] <= 350.0 + 1e-6);
    }
    // min_obj_distance = espacement (ArrangeParams, nester libnest2d).
    let gap = min_gap_same_plate(&bx, &plates(&a));
    assert!(gap >= 15.0 - 1e-3, "écart {gap}");
}

#[test]
fn overflow_creates_plates_and_spacing_changes_the_layout() {
    // 350² / (20 + 60)² ≈ 19 cubes par plateau avec 60 mm d'espacement.
    let (mut model, bed, config) = setup(30, false);
    let out =
        engine::adapters::ffi::arrange_gui(&mut model, &bed, &config, &settings(60.0)).unwrap();
    assert!(out.plate_count >= 2, "plateaux {}", out.plate_count);
    let p = plates(&model);
    assert!(p.iter().all(|&i| i < out.plate_count));
    assert!(p.iter().any(|&i| i > 0));
    let gap = min_gap_same_plate(&boxes(&model), &p);
    assert!(gap >= 60.0 - 1e-3, "écart {gap}");
}

#[test]
fn locked_plates_keep_their_instances() {
    let (mut model, bed, config) = setup(4, false);
    for (k, object) in model.objects.iter_mut().enumerate() {
        object.instances[0].plate_index = if k == 0 { 1 } else { 0 };
        object.instances[0].matrix =
            glam::DMat4::from_translation(DVec3::new(40.0 + 10.0 * k as f64, 50.0, 10.0));
    }
    let before = model.objects[0].instances[0].matrix;
    let mut s = settings(0.0);
    s.plate_count = 2;
    s.locked_plates = vec![1];
    engine::adapters::ffi::arrange_gui(&mut model, &bed, &config, &s).unwrap();
    assert_eq!(model.objects[0].instances[0].plate_index, 1);
    assert_eq!(model.objects[0].instances[0].matrix, before);
    // Les autres sont placés sur le plateau 0 (plateau verrouillé sauté).
    assert!(model.objects[1..]
        .iter()
        .all(|o| o.instances[0].plate_index == 0));
}

#[test]
fn plate_scope_only_moves_the_current_plate() {
    let (mut model, bed, config) = setup(4, false);
    for (k, object) in model.objects.iter_mut().enumerate() {
        object.instances[0].plate_index = u32::from(k >= 2);
        object.instances[0].matrix = glam::DMat4::from_translation(DVec3::new(100.0, 100.0, 10.0));
    }
    let untouched: Vec<_> = model.objects[..2]
        .iter()
        .map(|o| o.instances[0].matrix)
        .collect();
    let mut s = settings(0.0);
    s.plate_count = 2;
    s.plate_scope = Some(1);
    s.current_plate = 1;
    let out = engine::adapters::ffi::arrange_gui(&mut model, &bed, &config, &s).unwrap();
    assert_eq!(out.plate_count, 2);
    for (k, m) in untouched.iter().enumerate() {
        assert_eq!(&model.objects[k].instances[0].matrix, m);
        assert_eq!(model.objects[k].instances[0].plate_index, 0);
    }
    let p = plates(&model);
    assert_eq!(&p[2..], &[1, 1]);
    let bx = boxes(&model);
    assert!(
        min_gap_same_plate(&bx[2..], &p[2..]) > 0.0,
        "plus de chevauchement"
    );
}

#[test]
fn unprintable_instances_go_after_the_last_plate() {
    let (mut model, bed, config) = setup(3, false);
    model.objects[2].instances[0].printable = false;
    let out =
        engine::adapters::ffi::arrange_gui(&mut model, &bed, &config, &settings(0.0)).unwrap();
    assert_eq!(model.objects[2].instances[0].plate_index, 1, "beds + 1");
    assert_eq!(out.plate_count, 2);
}

#[test]
fn sequential_settings_return_print_orders() {
    let (mut model, bed, config) = setup(3, true);
    let mut s = settings(0.0);
    s.is_seq_print = true;
    let out = engine::adapters::ffi::arrange_gui(&mut model, &bed, &config, &s).unwrap();
    let mut orders: Vec<u32> = out.print_orders.iter().map(|o| o.expect("rang")).collect();
    orders.sort_unstable();
    assert_eq!(orders, vec![1, 2, 3]);
}

#[test]
fn invalid_spacing_is_rejected() {
    let (mut model, bed, config) = setup(1, false);
    assert!(
        engine::adapters::ffi::arrange_gui(&mut model, &bed, &config, &settings(-1.0)).is_err()
    );
    assert!(
        engine::adapters::ffi::arrange_gui(&mut model, &bed, &config, &settings(f64::NAN)).is_err()
    );
}
