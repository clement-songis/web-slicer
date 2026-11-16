//! `inherits_group` écrit dans le bloc de config du G-code : entrée process =
//! `inherits` du preset d'impression (Orca CLI OrcaSlicer.cpp:2985, GUI
//! PresetBundle.cpp:236), pas `print_settings_id`.
//! Exécution : cargo test -p engine --features ffi --test slice_inherits_group

#![cfg(feature = "ffi")]

mod common;

use engine::api::{CancelToken, ConfigValue, ProgressSink, SliceRequest};

fn inherits_group(adjust: impl FnOnce(&mut engine::api::DynamicPrintConfig)) -> String {
    let (model, mut config) =
        engine::adapters::ffi::read_project_3mf(&common::fixture("orca_project.3mf")).unwrap();
    config
        .set(
            "before_layer_change_gcode",
            ConfigValue::String("G92 E0".into()),
        )
        .unwrap();
    config.0.remove("inherits_group");
    adjust(&mut config);
    let work = tempfile::tempdir().unwrap();
    let req = SliceRequest {
        model,
        config,
        plate_index: 0,
        work_dir: work.path().to_path_buf(),
        calib: None,
    };
    let progress: ProgressSink = Box::new(|_, _| {});
    let result = engine::adapters::ffi::slice(req, progress, CancelToken::new()).unwrap();
    let gcode = std::fs::read_to_string(&result.gcode_path).unwrap();
    gcode
        .lines()
        .find_map(|l| l.strip_prefix("; inherits_group = "))
        .expect("inherits_group dans le bloc de config")
        .to_string()
}

#[test]
fn process_entry_is_inherits_not_settings_id() {
    let group = inherits_group(|c| {
        c.set("inherits", ConfigValue::String("harness_parent".into()))
            .unwrap();
        c.set(
            "print_settings_id",
            ConfigValue::String("harness_print".into()),
        )
        .unwrap();
    });
    assert_eq!(group.split(';').next(), Some("harness_parent"), "{group}");
}

#[test]
fn settings_id_is_the_fallback_without_inherits() {
    let group = inherits_group(|c| {
        c.0.remove("inherits");
        c.set(
            "print_settings_id",
            ConfigValue::String("harness_print".into()),
        )
        .unwrap();
    });
    assert_eq!(group.split(';').next(), Some("harness_print"), "{group}");
}

#[test]
fn given_inherits_group_is_kept() {
    // Orca CLI (make_refs brut) écrit la valeur fournie telle quelle.
    let group = inherits_group(|c| {
        c.set("inherits", ConfigValue::String("harness_parent".into()))
            .unwrap();
        c.set(
            "inherits_group",
            ConfigValue::Strings(vec!["harness_test".into()]),
        )
        .unwrap();
    });
    assert_eq!(group, "harness_test");
}
