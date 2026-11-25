//! Mode calibration transmis au moteur (`Print::set_calib_params`) : tour de
//! rétraction d'Orca (GCode.cpp, `Calib_Retraction_tower`) — la longueur de
//! rétraction croît d'un pas par millimètre au-dessus de 0,4 mm.
//! `cargo test -p engine --features ffi --test ffi_calib`.

#![cfg(feature = "ffi")]

mod common;

use engine::api::{CalibMode, CalibParams, CancelToken, ConfigValue, SliceRequest};

fn slice_cube(calib: Option<CalibParams>) -> String {
    slice_cube_with(calib, &[])
}

/// Tranche le cube du projet de référence avec des surcharges de config.
fn slice_cube_with(calib: Option<CalibParams>, overrides: &[(&str, ConfigValue)]) -> String {
    let (model, mut config) =
        engine::adapters::ffi::read_project_3mf(&common::fixture("orca_project.3mf"))
            .expect("lecture du projet 3MF");
    config
        .set(
            "before_layer_change_gcode",
            ConfigValue::String("G92 E0".into()),
        )
        .unwrap();
    for (key, value) in overrides {
        config.set(key, value.clone()).unwrap();
    }
    let work = tempfile::tempdir().unwrap();
    let result = engine::adapters::ffi::slice(
        SliceRequest {
            allow_mixed_temperatures: false,
            model,
            config,
            plate_index: 0,
            work_dir: work.path().to_path_buf(),
            calib,
        },
        Box::new(|_, _| {}),
        CancelToken::new(),
    )
    .expect("tranchage");
    std::fs::read_to_string(&result.gcode_path).unwrap()
}

#[test]
fn retraction_tower_steps_retraction_length() {
    let gcode = slice_cube(Some(CalibParams {
        mode: CalibMode::RetractionTower,
        start: 0.5,
        end: 2.0,
        step: 0.1,
        print_numbers: false,
        ..calib(CalibMode::RetractionTower)
    }));
    let lengths: Vec<f64> = gcode
        .lines()
        .filter_map(|l| l.strip_prefix("; Calib_Retraction_tower: "))
        .filter_map(|l| l.split("length:").nth(1))
        .map(|v| v.trim().parse().unwrap())
        .collect();
    assert!(
        lengths.len() > 50,
        "une ligne par couche : {}",
        lengths.len()
    );
    assert!((lengths[0] - 0.5).abs() < 1e-9);
    assert!(
        lengths.windows(2).all(|w| w[1] >= w[0]),
        "croissance monotone"
    );
    let distinct = {
        let mut v: Vec<i64> = lengths
            .iter()
            .map(|l| (l * 1000.0).round() as i64)
            .collect();
        v.dedup();
        v
    };
    // Cube 20 mm : 1 pas par mm au-dessus de 0,4 mm → ~20 paliers.
    assert!(distinct.len() >= 15, "paliers : {distinct:?}");
    assert!((lengths.last().unwrap() - (0.5 + 19.0 * 0.1)).abs() < 0.11);

    // Sans calibration : aucune ligne de calibration.
    assert!(!slice_cube(None).contains("Calib_Retraction_tower"));
}

/// Paramètres de calibration neutres pour un mode (champs input shaping à 0).
fn calib(mode: CalibMode) -> CalibParams {
    CalibParams {
        mode,
        start: 0.0,
        end: 0.0,
        step: 1.0,
        print_numbers: false,
        freq_start_x: 0.0,
        freq_end_x: 0.0,
        freq_start_y: 0.0,
        freq_end_y: 0.0,
        test_model: 0,
        shaper_type: engine::api::ShaperType::Default,
    }
}

fn klipper() -> Vec<(&'static str, ConfigValue)> {
    vec![
        ("gcode_flavor", ConfigValue::String("klipper".into())),
        // Orca main : default_jerk vectoriel par extrudeur (coFloats).
        ("default_jerk", ConfigValue::Floats(vec![0.0])),
    ]
}

/// Valeurs numériques de `key=` sur les lignes commençant par `prefix`.
fn values(gcode: &str, prefix: &str, key: &str) -> Vec<f64> {
    gcode
        .lines()
        .filter(|l| l.starts_with(prefix))
        .filter_map(|l| l.split(key).nth(1))
        .filter_map(|v| v.split([' ', ';']).next())
        .filter_map(|v| v.trim().parse().ok())
        .collect()
}

fn distinct(v: &[f64]) -> Vec<i64> {
    let mut d: Vec<i64> = v.iter().map(|x| (x * 1000.0).round() as i64).collect();
    d.sort();
    d.dedup();
    d
}

#[test]
fn input_shaping_frequency_sweeps_shaper_freq_per_layer() {
    let gcode = slice_cube_with(
        Some(CalibParams {
            start: 0.15,
            freq_start_x: 15.0,
            freq_end_x: 110.0,
            freq_start_y: 15.0,
            freq_end_y: 110.0,
            test_model: 0,
            shaper_type: engine::api::ShaperType::Default,
            ..calib(CalibMode::InputShapingFreq)
        }),
        &klipper(),
    );
    // Couche 1 : amortissement fixé ; ensuite fréquence interpolée par couche.
    let damp = values(&gcode, "SET_INPUT_SHAPER", "DAMPING_RATIO_X=");
    assert!(
        damp.iter().any(|d| (d - 0.15).abs() < 1e-6),
        "amortissement {damp:?}"
    );
    let freqs = values(&gcode, "SET_INPUT_SHAPER", "SHAPER_FREQ_X=");
    assert!(
        freqs.len() > 50,
        "une commande par couche : {}",
        freqs.len()
    );
    assert!(
        freqs.windows(2).all(|w| w[1] >= w[0]),
        "croissance monotone"
    );
    assert!(freqs[0] >= 15.0 - 1e-6 && *freqs.last().unwrap() <= 110.0 + 1e-6);
    assert!(
        distinct(&freqs).len() > 50,
        "paliers : {}",
        distinct(&freqs).len()
    );
    assert!(gcode.contains("SET_VELOCITY_LIMIT MINIMUM_CRUISE_RATIO=0"));
    assert!(!slice_cube_with(None, &klipper()).contains("SET_INPUT_SHAPER"));
}

#[test]
fn input_shaping_damping_sweeps_damping_ratio() {
    let gcode = slice_cube_with(
        Some(CalibParams {
            start: 0.0,
            end: 0.4,
            freq_start_x: 30.0,
            freq_start_y: 42.0,
            ..calib(CalibMode::InputShapingDamp)
        }),
        &klipper(),
    );
    assert!(values(&gcode, "SET_INPUT_SHAPER", "SHAPER_FREQ_X=").contains(&30.0));
    assert!(values(&gcode, "SET_INPUT_SHAPER", "SHAPER_FREQ_Y=").contains(&42.0));
    let damp = values(&gcode, "SET_INPUT_SHAPER", "DAMPING_RATIO_X=");
    assert!(damp.len() > 50, "une commande par couche : {}", damp.len());
    assert!(
        damp.windows(2).all(|w| w[1] >= w[0] - 1e-9),
        "croissance monotone"
    );
    assert!(*damp.last().unwrap() <= 0.4 + 1e-6 && distinct(&damp).len() > 40);
}

#[test]
fn input_shaping_damping_outside_unit_range_is_rejected() {
    let p = CalibParams {
        start: 0.0,
        end: 1.5,
        ..calib(CalibMode::InputShapingDamp)
    };
    assert!(p.validate().is_err());
}

#[test]
fn cornering_sweeps_square_corner_velocity() {
    let mut overrides = klipper();
    overrides.push(("machine_max_jerk_x", ConfigValue::Floats(vec![15.0, 15.0])));
    overrides.push(("machine_max_jerk_y", ConfigValue::Floats(vec![15.0, 15.0])));
    let gcode = slice_cube_with(
        Some(CalibParams {
            start: 1.0,
            end: 15.0,
            ..calib(CalibMode::Cornering)
        }),
        &overrides,
    );
    let scv = values(
        &gcode,
        "SET_VELOCITY_LIMIT SQUARE_CORNER_VELOCITY",
        "VELOCITY=",
    );
    assert!(scv.len() > 50, "une commande par couche : {}", scv.len());
    assert!(
        scv.windows(2).all(|w| w[1] >= w[0]),
        "croissance monotone : {scv:?}"
    );
    assert!(scv[0] >= 1.0 - 1e-6 && *scv.last().unwrap() <= 15.0 + 1e-6);
    assert!(distinct(&scv).len() > 20);
}

#[test]
fn pa_pattern_injects_one_pattern_per_pressure_advance_value() {
    let gcode = slice_cube_with(
        Some(CalibParams {
            start: 0.0,
            end: 0.08,
            step: 0.02,
            print_numbers: true,
            ..calib(CalibMode::PaPattern)
        }),
        &klipper(),
    );
    assert!(gcode.contains("; start pressure advance pattern for layer"));
    let pa = values(&gcode, "SET_PRESSURE_ADVANCE", "ADVANCE=");
    let got = distinct(&pa);
    // 0 ; 0,02 ; 0,04 ; 0,06 ; 0,08 → 5 motifs (CalibPressureAdvancePattern).
    assert_eq!(got, vec![0, 20, 40, 60, 80], "valeurs PA : {pa:?}");
    // Réglages imposés par Plater::_calib_pa_pattern (3 parois).
    assert!(gcode.contains("; wall_loops = 3"), "wall_loops imposé");
}
