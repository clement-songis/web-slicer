//! Suite générique du trait `SlicerEngine` (T011, TDD — écrite AVANT les
//! implémentations).
//!
//! Garantie de substituabilité (constitution II, contrat garantie n°4) :
//! la même suite doit passer telle quelle sur `adapters::ffi`,
//! `adapters::cli`, puis la future implémentation Rust native.
//! Chaque adaptateur l'invoque depuis son test d'intégration :
//! `common::trait_suite::run_all(&engine)`.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use engine::api::*;
use engine::SlicerEngine;

use super::fixture;

/// Exécute toute la suite. Panique au premier écart de contrat.
pub fn run_all(e: &dyn SlicerEngine) {
    load_stl_cube(e);
    load_obj(e);
    read_project_3mf(e);
    roundtrip_project_3mf(e);
    convert_step(e);
    repair_reports(e);
    arrange_keeps_objects_in_bed(e);
    resolve_chain_applies_overrides(e);
    validate_flags_out_of_bounds(e);
    slice_produces_gcode_with_progress(e);
    slice_cancellation(e);
    parse_gcode_has_layers(e);
}

fn bed_200x200() -> BuildVolume {
    BuildVolume {
        bed_shape: vec![[0.0, 0.0], [200.0, 0.0], [200.0, 200.0], [0.0, 200.0]],
        max_height: 200.0,
        excluded: vec![],
    }
}

fn total_triangles(m: &Model) -> usize {
    m.objects
        .iter()
        .flat_map(|o| &o.volumes)
        .map(|v| v.mesh.triangle_count())
        .sum()
}

pub fn load_stl_cube(e: &dyn SlicerEngine) {
    let model = e
        .load_model(&fixture("cube20.stl"), ModelFormat::Stl)
        .expect("charge cube20.stl");
    assert_eq!(model.objects.len(), 1);
    assert_eq!(total_triangles(&model), 12, "cube = 12 triangles");
}

pub fn load_obj(e: &dyn SlicerEngine) {
    let model = e
        .load_model(&fixture("cube20.obj"), ModelFormat::Obj)
        .expect("charge cube20.obj");
    assert_eq!(total_triangles(&model), 12);
}

pub fn read_project_3mf(e: &dyn SlicerEngine) {
    let (model, _config) = e
        .read_project_3mf(&fixture("orca_project.3mf"))
        .expect("lit un projet 3MF Orca (chemin unicode interne)");
    assert!(!model.is_empty(), "le projet contient au moins un objet");
}

pub fn roundtrip_project_3mf(e: &dyn SlicerEngine) {
    let model = e
        .load_model(&fixture("cube20.stl"), ModelFormat::Stl)
        .unwrap();
    let mut config = DynamicPrintConfig::new();
    config
        .set("layer_height", ConfigValue::Float(0.28))
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("roundtrip.3mf");
    e.write_project_3mf(&model, &config, &out).expect("écrit");
    let (back_model, back_config) = e.read_project_3mf(&out).expect("relit");
    assert_eq!(total_triangles(&back_model), 12);
    assert_eq!(
        back_config.get("layer_height"),
        Some(&ConfigValue::Float(0.28)),
        "la config embarquée survit à l'aller-retour"
    );
}

pub fn convert_step(e: &dyn SlicerEngine) {
    match e.convert_to_mesh(&fixture("cube20.step")) {
        Ok(mesh) => {
            assert!(!mesh.is_empty(), "le STEP produit un maillage");
            let (lo, hi) = mesh.bounding_box().unwrap();
            for i in 0..3 {
                assert!((hi[i] - lo[i] - 20.0).abs() < 0.5, "cube ~20 mm");
            }
        }
        // Contrat : une implémentation sans STEP doit répondre Unsupported
        // explicitement (fallback CLI, note T004) — jamais un succès vide.
        Err(err) => assert_eq!(err.code, EngineErrorCode::Unsupported, "{err}"),
    }
}

pub fn repair_reports(e: &dyn SlicerEngine) {
    // maillage troué : un tétraèdre auquel il manque une face
    let broken = TriangleMesh {
        vertices: vec![
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [0.0, 10.0, 0.0],
            [0.0, 0.0, 10.0],
        ],
        indices: vec![[0, 2, 1], [0, 1, 3], [1, 2, 3]],
    };
    let (repaired, report) = e.repair_mesh(&broken).expect("répare");
    assert!(!repaired.is_empty());
    assert!(
        report.repaired_anything(),
        "le trou doit être détecté/corrigé : {report:?}"
    );
}

pub fn arrange_keeps_objects_in_bed(e: &dyn SlicerEngine) {
    let mut model = Model::default();
    for _ in 0..4 {
        let m = e
            .load_model(&fixture("cube20.stl"), ModelFormat::Stl)
            .unwrap();
        model.objects.extend(m.objects);
    }
    let bed = bed_200x200();
    e.arrange(&mut model, &bed, &ArrangeParams::default())
        .expect("arrange");
    // chaque instance reste dans le plateau (approximation par l'origine)
    for obj in &model.objects {
        for inst in &obj.instances {
            let t = inst.matrix.w_axis;
            assert!(
                (0.0..=200.0).contains(&t.x) && (0.0..=200.0).contains(&t.y),
                "instance hors plateau : {t:?}"
            );
        }
    }
}

pub fn resolve_chain_applies_overrides(e: &dyn SlicerEngine) {
    let mut base = serde_json::Map::new();
    base.insert("layer_height".into(), serde_json::json!("0.2"));
    base.insert("sparse_infill_density".into(), serde_json::json!("15%"));
    let mut leaf = serde_json::Map::new();
    leaf.insert("layer_height".into(), serde_json::json!("0.12"));
    let chain = [
        RawPreset {
            name: "base".into(),
            inherits: None,
            values: base,
        },
        RawPreset {
            name: "fin".into(),
            inherits: Some("base".into()),
            values: leaf,
        },
    ];
    let cfg = e.resolve_preset_chain(&chain).expect("résout");
    assert_eq!(
        cfg.get("layer_height"),
        Some(&ConfigValue::Float(0.12)),
        "la feuille surcharge la racine"
    );
    assert!(
        cfg.get("sparse_infill_density").is_some(),
        "les valeurs non surchargées suivent le parent"
    );
}

pub fn validate_flags_out_of_bounds(e: &dyn SlicerEngine) {
    let mut cfg = DynamicPrintConfig::new();
    cfg.set("layer_height", ConfigValue::Float(-3.0)).unwrap();
    let warnings = e.validate_config(&cfg).expect("valide");
    assert!(
        warnings.iter().any(|w| w.key == "layer_height"),
        "layer_height négative doit être signalée"
    );
}

fn slice_cube(e: &dyn SlicerEngine, cancel: CancelToken) -> (EngineResult<SliceResult>, Vec<f32>) {
    let model = e
        .load_model(&fixture("cube20.stl"), ModelFormat::Stl)
        .unwrap();
    let config = default_slice_config();
    let dir = tempfile::tempdir().unwrap();
    let progress: Arc<Mutex<Vec<f32>>> = Arc::default();
    let sink = {
        let p = progress.clone();
        Box::new(move |_phase: &str, ratio: f32| p.lock().unwrap().push(ratio))
    };
    let result = e.slice(
        SliceRequest {
            model,
            config,
            plate_index: 0,
            work_dir: dir.keep(),
        },
        sink,
        cancel,
    );
    let seen = progress.lock().unwrap().clone();
    (result, seen)
}

/// Config minimale viable pour trancher le cube (complétée par les défauts
/// du registre côté implémentation).
fn default_slice_config() -> DynamicPrintConfig {
    let mut cfg = DynamicPrintConfig::new();
    cfg.set("layer_height", ConfigValue::Float(0.2)).unwrap();
    cfg
}

pub fn slice_produces_gcode_with_progress(e: &dyn SlicerEngine) {
    let (result, progress) = slice_cube(e, CancelToken::new());
    let result = result.expect("slice réussit");
    let gcode = std::fs::read_to_string(&result.gcode_path).expect("gcode lisible");
    assert!(gcode.contains("G1"), "du G-code de mouvement est produit");
    assert!(
        result.stats.layer_count > 10,
        "cube 20 mm / 0.2 → >10 couches"
    );
    assert!(!progress.is_empty(), "la progression est remontée");
    assert!(
        progress.windows(2).all(|w| w[0] <= w[1]),
        "progression monotone : {progress:?}"
    );
}

pub fn slice_cancellation(e: &dyn SlicerEngine) {
    let cancel = CancelToken::new();
    cancel.cancel(); // annulé avant même de démarrer
    let (result, _) = slice_cube(e, cancel);
    match result {
        Err(err) => assert_eq!(err.code, EngineErrorCode::Cancelled, "{err}"),
        Ok(_) => panic!("un slice annulé ne doit pas aboutir"),
    }
}

pub fn parse_gcode_has_layers(e: &dyn SlicerEngine) {
    let (result, _) = slice_cube(e, CancelToken::new());
    let result = result.expect("slice pour préviz");
    let preview = e.parse_gcode(&result.gcode_path).expect("parse");
    assert!(preview.layers.len() > 10);
    assert!(
        preview
            .kinds_present
            .iter()
            .any(|k| !matches!(k, LineKind::Travel)),
        "au moins un type extrudé présent"
    );
    // z strictement croissant
    assert!(preview.layers.windows(2).all(|w| w[0].z < w[1].z));
}

/// Compte les invocations — utilitaire pour les tests de progression fine.
#[allow(dead_code)]
pub struct CallCounter(pub AtomicU32);

#[allow(dead_code)]
impl CallCounter {
    pub fn bump(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}
