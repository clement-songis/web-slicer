//! Volumes de purge calculés depuis les couleurs (FlushVolCalculator, flush.cpp)
//! comparés aux matrices qu'Orca 2.4.1 a calculées et écrites dans les 3MF de
//! tests/fixtures/multi_filament (CLI : minimum = nozzle_volume[0],
//! nozzle_flush_dataset de la machine, couleurs des filaments).
//! Exécution : cargo test -p engine --features ffi --test ffi_flush_volumes

#![cfg(feature = "ffi")]

use std::io::Read;
use std::path::Path;

use serde_json::Value;

fn project(name: &str) -> serde_json::Map<String, Value> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/multi_filament")
        .join(name);
    let mut zip = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    let mut text = String::new();
    zip.by_name("Metadata/project_settings.config")
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    serde_json::from_str(&text).unwrap()
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect()
}

fn first_int(cfg: &serde_json::Map<String, Value>, key: &str) -> i32 {
    cfg.get(key)
        .and_then(|v| {
            v.as_array()
                .and_then(|a| a.first().cloned())
                .or(Some(v.clone()))
        })
        .and_then(|v| v.as_str().and_then(|s| s.parse::<f64>().ok()))
        .map_or(0, |f| f as i32)
}

fn check(name: &str) {
    let cfg = project(name);
    let colours = strings(&cfg["filament_colour"]);
    let orca: Vec<f64> = strings(&cfg["flush_volumes_matrix"])
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();
    let n = colours.len();
    // Minimum : get_min_flush_volumes (GUI) sur la config du projet relue.
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/multi_filament")
        .join(name);
    let (_, config) = engine::adapters::ffi::read_project_3mf(&path).unwrap();
    let min = engine::adapters::ffi::min_flush_volumes(&config, 0).unwrap();
    eprintln!(
        "{name} : minimum {min:?} (nozzle_volume {})",
        first_int(&cfg, "nozzle_volume")
    );
    let ours = engine::adapters::ffi::flush_volumes(
        &colours,
        &[],
        &min,
        first_int(&cfg, "nozzle_flush_dataset"),
    )
    .unwrap();
    // Matrice Orca par buse (n×n×extrudeurs) : première buse.
    assert_eq!(
        ours,
        orca[..n * n].to_vec(),
        "{name} : couleurs {colours:?}"
    );
}

#[test]
fn voron_matrix_matches_orca() {
    check("two_objects.3mf");
}

#[test]
fn bambu_x1c_matrix_matches_orca() {
    check("bambu_x1c.3mf");
}

#[test]
fn support_rules_and_invalid_colour() {
    let colours = vec!["#000000".to_string(), "#FFFFFF".to_string()];
    let m = engine::adapters::ffi::flush_volumes(&colours, &[false, true], &[0], 0).unwrap();
    assert_eq!(m[0], 0.0);
    assert_eq!(m[3], 0.0);
    assert_eq!(m[1], 230.0, "vers un support : g_flush_volume_to_support");
    assert!(
        m[2] >= 700.0,
        "depuis un support : au moins g_min_flush_volume_from_support"
    );
    assert!(engine::adapters::ffi::flush_volumes(&["rouge".into()], &[], &[], 0).is_err());
}
