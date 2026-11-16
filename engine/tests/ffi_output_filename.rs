//! Nom d'export d'Orca calculé par libslic3r au tranchage
//! (`Print::output_filename` : gabarit `filename_format` évalué par le
//! PlaceholderParser avec les statistiques du tranchage), base = jeton
//! `OUTPUT_FILENAME_BASE_TOKEN` remplacé par le nom du projet côté serveur.
//! Exécution : cargo test -p engine --features ffi --test ffi_output_filename.

#![cfg(feature = "ffi")]

mod common;

use engine::api::{
    CancelToken, ConfigValue, ProgressSink, SliceRequest, OUTPUT_FILENAME_BASE_TOKEN,
};

fn slice_with_format(format: Option<&str>) -> (String, String) {
    let (model, mut config) =
        engine::adapters::ffi::read_project_3mf(&common::fixture("orca_project.3mf")).unwrap();
    config
        .set(
            "before_layer_change_gcode",
            ConfigValue::String("G92 E0".into()),
        )
        .unwrap();
    if let Some(f) = format {
        config
            .set("filename_format", ConfigValue::String(f.into()))
            .unwrap();
    }
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
    (result.output_filename.expect("nom calculé"), gcode)
}

/// `short_time` d'Orca : « 8m 52s » → « 8m52s ».
fn header_short_time(gcode: &str) -> String {
    gcode
        .lines()
        .find_map(|l| l.strip_prefix("; estimated printing time (normal mode) = "))
        .unwrap()
        .replace(' ', "")
}

fn config_value(gcode: &str, key: &str) -> String {
    let prefix = format!("; {key} = ");
    gcode
        .lines()
        .find_map(|l| l.strip_prefix(&prefix))
        .unwrap()
        .to_string()
}

#[test]
fn default_template_is_evaluated_with_slicing_statistics() {
    let (name, gcode) = slice_with_format(None);
    let filament = config_value(&gcode, "filament_type");
    let filament = filament.split(';').next().unwrap().trim_matches('"');
    assert_eq!(
        name,
        format!(
            "{OUTPUT_FILENAME_BASE_TOKEN}_{filament}_{}.gcode",
            header_short_time(&gcode)
        )
    );
}

#[test]
fn full_placeholder_language_is_supported() {
    // Condition + expression : hors du sous-ensemble de l'ancien analyseur Rust.
    let (name, gcode) = slice_with_format(Some(
        "{if layer_height < 0.25}fine{else}draft{endif}_{input_filename_base}_{int(layer_height*100)}",
    ));
    let lh: f64 = config_value(&gcode, "layer_height").parse().unwrap();
    let quality = if lh < 0.25 { "fine" } else { "draft" };
    assert_eq!(
        name,
        format!(
            "{quality}_{OUTPUT_FILENAME_BASE_TOKEN}_{}.gcode",
            (lh * 100.0) as i64
        )
    );
}
