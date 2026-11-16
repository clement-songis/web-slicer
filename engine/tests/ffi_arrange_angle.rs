//! Parité de l'angle d'arrangement (allow_rotations) avec le CLI Orca :
//! le cylindre golden doit pivoter du même angle quel que soit le chemin de
//! chargement (read_from_file direct vs maillage cuit WSMh).
//! Exécution : `cargo test -p engine --features ffi --test ffi_arrange_angle`.

#![cfg(feature = "ffi")]

use std::path::PathBuf;

use engine::api::{ArrangeParams, BuildVolume, Model};

fn cylinder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/oracle/fixtures/cylinder.stl")
}

fn arrange_angle_deg(mut model: Model) -> f64 {
    let bed = BuildVolume {
        bed_shape: vec![[0.0, 0.0], [350.0, 0.0], [350.0, 350.0], [0.0, 350.0]],
        max_height: 340.0,
        excluded: Vec::new(),
    };
    let params = ArrangeParams {
        clearance: 0.0,
        use_machine_clearance: false,
        allow_rotations: true,
        orca_cli_mode: false,
        print_config: None,
        is_seq_print: false,
    };
    engine::adapters::ffi::arrange(&mut model, &bed, &params).expect("arrange");
    let m = model.objects[0].instances[0].matrix;
    m.x_axis.y.atan2(m.x_axis.x).to_degrees()
}

#[test]
fn cylinder_arrange_angle_matches_between_load_paths() {
    let direct = engine::adapters::ffi::load_model(&cylinder()).expect("load_model");
    let mesh = engine::adapters::ffi::convert_to_mesh(&cylinder()).expect("convert_to_mesh");
    let mut baked = Model::default();
    baked.add_object("cylinder.stl", mesh);
    let a = arrange_angle_deg(direct);
    let b = arrange_angle_deg(baked);
    eprintln!("angle read_from_file = {a}, angle maillage cuit = {b}");
    assert!((a - b).abs() < 1e-9, "read_from_file {a}° ≠ cuit {b}°");
}

/// Angle réel du CLI Orca, lu dans la matrice d'instance du 3MF exporté par
/// `orca-slicer --arrange 1 --orient 0 --export-3mf` sur ce cylindre :
/// −137,812452° (l'ancien « +2,8125° » était une lecture du G-code). La
/// réplique engine/tests/replica/arrange_replica.cpp du bloc d'arrangement du
/// CLI donne la même valeur ; seule l'inflation d'item la change.
#[test]
fn cylinder_arrange_angle_matches_orca_cli() {
    let direct = engine::adapters::ffi::load_model(&cylinder()).expect("load_model");
    let a = arrange_angle_deg(direct);
    assert!(
        (a - -137.812452459).abs() < 1e-7,
        "angle {a}° ≠ −137.812452459° (Orca CLI)"
    );
}
