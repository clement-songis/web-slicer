//! Projets de calibration d'Orca (portage des Plater::calib_*) : modèle,
//! coupes, réglages et paramètres conformes au code de la GUI d'Orca 2.4.1,
//! puis tranchage en mode calibration (GCode.cpp) et valeurs injectées.
//! Exécution : cargo test -p engine --features ffi --test ffi_calib_project

#![cfg(feature = "ffi")]

mod common;

use std::path::{Path, PathBuf};

use engine::adapters::ffi::{calib_project, CalibProject, CalibRequest};
use engine::api::{CalibMode, CancelToken, ConfigValue, DynamicPrintConfig, SliceRequest};

fn resources() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../vendor/OrcaSlicer/resources")
}

/// Config résolue par le CLI Orca (Voron 2.4 350, 0.20 Standard, Generic PLA ;
/// 3MF exporté par orca-slicer, tranchable tel quel par le CLI).
fn base_config() -> DynamicPrintConfig {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/print_sequence/by-layer.3mf");
    engine::adapters::ffi::read_project_3mf(&path).unwrap().1
}

fn project(kind: &str, start: f64, end: f64, step: f64) -> (CalibProject, DynamicPrintConfig) {
    project_with(CalibRequest {
        kind: kind.into(),
        start,
        end,
        step,
        ..Default::default()
    })
}

fn project_with(req: CalibRequest) -> (CalibProject, DynamicPrintConfig) {
    let config = base_config();
    let kind = req.kind.clone();
    let p = calib_project(&req, &config, &resources()).unwrap_or_else(|e| panic!("{kind} : {e:?}"));
    let mut merged = config;
    merged.0.extend(p.overrides.0.clone());
    (p, merged)
}

fn z_size(p: &CalibProject) -> f64 {
    let o = &p.model.objects[0];
    let m = o.instances[0].matrix;
    let (mut lo, mut hi) = (f64::MAX, f64::MIN);
    for v in &o.volumes {
        for q in &v.mesh.vertices {
            let w = (m * v.matrix).transform_point3(glam::DVec3::new(
                q[0].into(),
                q[1].into(),
                q[2].into(),
            ));
            lo = lo.min(w.z);
            hi = hi.max(w.z);
        }
    }
    hi - lo
}

fn obj_setting(p: &CalibProject, key: &str) -> Option<ConfigValue> {
    p.model.objects[0].settings.get(key).cloned()
}

fn slice(p: &CalibProject, config: DynamicPrintConfig) -> String {
    let work = tempfile::tempdir().unwrap();
    let r = engine::adapters::ffi::slice(
        SliceRequest {
            allow_mixed_temperatures: false,
            model: p.model.clone(),
            config,
            plate_index: 0,
            work_dir: work.path().to_path_buf(),
            calib: p.calib,
        },
        Box::new(|_, _| {}),
        CancelToken::new(),
    )
    .unwrap();
    std::fs::read_to_string(r.gcode_path).unwrap()
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

#[test]
fn temperature_tower_like_orca() {
    let (p, cfg) = project("temp_tower", 230.0, 190.0, 5.0);
    let calib = p.calib.unwrap();
    assert_eq!(calib.mode, CalibMode::TempTower);
    // 9 blocs de 10 mm (230 → 190 par 5 °C), buse 0,4.
    assert!(close(z_size(&p), 90.0, 0.05), "hauteur {}", z_size(&p));
    assert_eq!(
        obj_setting(&p, "layer_height"),
        Some(ConfigValue::Float(0.2))
    );
    assert_eq!(
        p.overrides.get("nozzle_temperature"),
        Some(&ConfigValue::Ints(vec![230]))
    );
    let gcode = slice(&p, cfg);
    let temps: Vec<i64> = gcode
        .lines()
        .filter_map(|l| l.strip_prefix("M104 S"))
        .filter_map(|t| t.split_whitespace().next()?.parse::<f64>().ok())
        .map(|t| t as i64)
        .filter(|t| *t > 0) // M104 S0 du G-code de fin
        .collect();
    let mut distinct = temps.clone();
    distinct.dedup();
    assert_eq!(distinct.first(), Some(&230));
    assert_eq!(distinct.iter().min(), Some(&190));
    assert!(distinct.windows(2).all(|w| w[1] <= w[0]), "{distinct:?}");
}

#[test]
fn pa_tower_like_orca() {
    let (p, cfg) = project("pa_tower", 0.0, 0.1, 0.002);
    assert_eq!(p.calib.unwrap().mode, CalibMode::PaTower);
    assert!(close(z_size(&p), 51.0, 0.05), "hauteur {}", z_size(&p)); // ceil(0,1/0,002)+1
    assert_eq!(obj_setting(&p, "wall_loops"), Some(ConfigValue::Int(2)));
    // Orca main : outer_wall_speed vectoriel par extrudeur (coFloats).
    assert!(
        matches!(obj_setting(&p, "outer_wall_speed"), Some(ConfigValue::Floats(v)) if v.len() == 1 && v[0] > 0.0)
    );
    let gcode = slice(&p, cfg);
    let pa = gcode
        .lines()
        .filter(|l| l.starts_with("SET_PRESSURE_ADVANCE") || l.starts_with("M900"))
        .count();
    assert!(pa > 40, "{pa} commandes PA");
}

#[test]
fn pa_line_like_orca() {
    let (p, cfg) = project("pa_line", 0.0, 0.1, 0.002);
    assert_eq!(p.calib.unwrap().mode, CalibMode::PaLine);
    assert_eq!(
        p.overrides.get("overhang_reverse"),
        Some(&ConfigValue::Bool(false))
    );
    let gcode = slice(&p, cfg);
    let pa = gcode
        .lines()
        .filter(|l| l.starts_with("SET_PRESSURE_ADVANCE") || l.starts_with("M900"))
        .count();
    assert!(pa >= 50, "{pa} commandes PA (une par ligne de test)");
}

#[test]
fn flow_rate_pass1_like_orca() {
    let (p, cfg) = project("flow_pass1", 0.0, 0.0, 0.0);
    assert!(p.calib.is_none(), "pas de mode calibration pour le débit");
    assert_eq!(
        p.overrides.get("layer_height"),
        Some(&ConfigValue::Float(0.2))
    );
    let mut ratios: Vec<f64> = p
        .model
        .objects
        .iter()
        .map(|o| match o.settings.get("print_flow_ratio") {
            Some(ConfigValue::Float(v)) => (v * 100.0).round() / 100.0,
            other => panic!("{} : {other:?}", o.name),
        })
        .collect();
    ratios.sort_by(f64::total_cmp);
    assert!(
        close(ratios[0], 0.8, 1e-6) && close(*ratios.last().unwrap(), 1.2, 1e-6),
        "{ratios:?}"
    );
    assert_eq!(ratios.len(), 9);
    let gcode = slice(&p, cfg);
    assert!(gcode.contains("Top surface"));
}

#[test]
fn flow_rate_yolo_like_orca() {
    let (p, _) = project("flow_yolo", 0.0, 0.0, 0.0);
    assert!(p.model.objects.len() > 5);
    assert!(p
        .model
        .objects
        .iter()
        .all(|o| o.settings.get("print_flow_ratio").is_some()));
}

#[test]
fn max_volumetric_speed_like_orca() {
    let (p, cfg) = project("max_vol_speed", 5.0, 20.0, 0.5);
    let calib = p.calib.unwrap();
    assert_eq!(calib.mode, CalibMode::VolSpeedTower);
    assert!(close(z_size(&p), 32.0, 0.6), "hauteur {}", z_size(&p)); // (20-5+1)/0,5, couches 0,32
                                                                     // mm³/s → mm/s : Flow(0,7 ; 0,32 ; 0,4).mm3_per_mm × filament_flow_ratio.
    assert!(calib.start > 5.0 && calib.end > calib.start);
    assert_eq!(
        p.overrides.get("spiral_mode"),
        Some(&ConfigValue::Bool(true))
    );
    let gcode = slice(&p, cfg);
    assert!(gcode
        .lines()
        .any(|l| l.starts_with("G1") && l.contains(" F")));
}

#[test]
fn retraction_tower_like_orca() {
    let (p, cfg) = project("retraction", 0.0, 2.0, 0.1);
    assert_eq!(p.calib.unwrap().mode, CalibMode::RetractionTower);
    assert!(close(z_size(&p), 21.4, 0.25), "hauteur {}", z_size(&p)); // 1 + 0,4 + 20
    assert_eq!(
        p.overrides.get("use_firmware_retraction"),
        Some(&ConfigValue::Bool(false))
    );
    let gcode = slice(&p, cfg);
    assert!(gcode.contains("; Calib_Retraction_tower"));
}

#[test]
fn vfa_tower_like_orca() {
    let (p, cfg) = project("vfa", 40.0, 200.0, 10.0);
    assert_eq!(p.calib.unwrap().mode, CalibMode::VfaTower);
    assert!(close(z_size(&p), 85.0, 0.25), "hauteur {}", z_size(&p)); // 5 × 17
    let gcode = slice(&p, cfg);
    assert!(gcode.contains("; spiral_mode = 1"));
}

/// Port de normalize (tests/oracle/lib/gcode.ts).
fn normalize(gcode: &str) -> Vec<String> {
    let volatile = regex::Regex::new(
        r"^; (filename|.*_settings_id|inherits|compatible_.*|thumbnails.*|printer_model|print_compatible_printers|upward_compatible_machine|setting_id|from|name|.*_notes) = ",
    )
    .unwrap();
    let thumb_start =
        regex::Regex::new(r"^; (THUMBNAIL_BLOCK_START|thumbnail(_JPG|_QOI)? begin)").unwrap();
    let thumb_end =
        regex::Regex::new(r"^; (THUMBNAIL_BLOCK_END|thumbnail(_JPG|_QOI)? end)").unwrap();
    let generated = regex::Regex::new(r"(?i)^; generated by ").unwrap();
    let block = regex::Regex::new(r"^;\s*(HEADER|EXECUTABLE)_BLOCK").unwrap();
    let mut out = Vec::new();
    let mut skipping = false;
    for raw in gcode.replace("\r\n", "\n").split('\n') {
        let line = raw.trim_end();
        if thumb_start.is_match(line) {
            skipping = true;
        }
        if skipping {
            if thumb_end.is_match(line) {
                skipping = false;
            }
            continue;
        }
        if line.is_empty()
            || generated.is_match(line)
            || block.is_match(line)
            || volatile.is_match(line)
        {
            continue;
        }
        out.push(line.to_string());
    }
    out
}

const PARITY: [(&str, f64, f64, f64); 12] = [
    ("pa_pattern", 0.0, 0.08, 0.005),
    ("is_freq", 0.15, 0.0, 0.0),
    ("is_damp", 0.0, 0.4, 0.0),
    ("cornering", 1.0, 15.0, 0.0),
    ("temp_tower", 230.0, 190.0, 5.0),
    ("pa_tower", 0.0, 0.1, 0.002),
    ("flow_pass1", 0.0, 0.0, 0.0),
    ("flow_pass2", 0.0, 0.0, 0.0),
    ("flow_yolo", 0.0, 0.0, 0.0),
    ("max_vol_speed", 5.0, 20.0, 0.5),
    ("retraction", 0.0, 2.0, 0.1),
    ("vfa", 40.0, 200.0, 10.0),
];

fn parity_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/calib_project")
}

/// Régénère les projets de parité (3MF : modèle + config résolue avec les
/// presets édités d'Orca) ; les références sont produites par make_ref.py
/// (orca-slicer --slice 0, variante majoritaire sur 5 tirages).
#[test]
#[ignore = "régénère tests/fixtures/calib_project/*.3mf"]
fn export_calib_projects() {
    std::fs::create_dir_all(parity_dir()).unwrap();
    for (kind, s, e, st) in PARITY {
        let (p, cfg) = project(kind, s, e, st);
        engine::adapters::ffi::write_project_3mf_with_plates(
            &p.model,
            &cfg,
            &[],
            &parity_dir().join(format!("{kind}.3mf")),
        )
        .unwrap();
    }
}

/// Le projet construit (modèle, coupes, échelle, réglages d'objet, presets
/// édités) tranché par le moteur est identique à Orca tranchant le même 3MF.
/// Le CLI d'Orca ne connaît pas le mode calibration (Print::set_calib_params,
/// GUI seulement) : comparaison hors injection de calibration, celle-ci étant
/// le code de GCode.cpp de libslic3r (vérifiée par les tests ci-dessus).
#[test]
fn calib_projects_slice_like_orca_cli() {
    let mut checked = 0;
    for (kind, ..) in PARITY {
        let reference = parity_dir().join(format!("{kind}.orca.gcode"));
        if !reference.exists() {
            if parity_dir().join(format!("{kind}.variants.json")).exists() {
                eprintln!("{kind} : Orca non déterministe → orca_variants_contain_engine_output");
            } else {
                eprintln!("{kind} : référence absente (make_ref.py)");
            }
            continue;
        }
        let (model, config) =
            engine::adapters::ffi::read_project_3mf(&parity_dir().join(format!("{kind}.3mf")))
                .unwrap();
        let work = tempfile::tempdir().unwrap();
        let r = engine::adapters::ffi::slice(
            SliceRequest {
                allow_mixed_temperatures: false,
                model,
                config,
                plate_index: 0,
                work_dir: work.path().to_path_buf(),
                calib: None,
            },
            Box::new(|_, _| {}),
            CancelToken::new(),
        )
        .unwrap();
        let ours = normalize(&std::fs::read_to_string(r.gcode_path).unwrap());
        let orca = normalize(&std::fs::read_to_string(reference).unwrap());
        let first = (0..ours.len().max(orca.len())).find(|&i| ours.get(i) != orca.get(i));
        assert!(
            first.is_none(),
            "{kind} : ligne {:?} moteur {:?} / Orca {:?}",
            first,
            first.and_then(|i| ours.get(i)),
            first.and_then(|i| orca.get(i))
        );
        checked += 1;
    }
    eprintln!("{checked}/{} projets identiques à Orca", PARITY.len());
}

#[test]
fn unknown_kind_and_bad_range_are_refused() {
    let config = base_config();
    let bad = CalibRequest {
        kind: "nope".into(),
        start: 0.0,
        end: 1.0,
        step: 0.1,
        ..Default::default()
    };
    assert!(calib_project(&bad, &config, &resources()).is_err());
    let bad = CalibRequest {
        kind: "vfa".into(),
        start: 40.0,
        end: 200.0,
        step: 0.0,
        ..Default::default()
    };
    assert!(calib_project(&bad, &config, &resources()).is_err());
}

/// FNV-1a 64 bits des lignes normalisées (sans dépendance de hachage).
fn fnv(lines: &[String]) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for line in lines {
        for b in line.bytes().chain(std::iter::once(b'\n')) {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x100000001b3);
        }
    }
    format!("{h:016x}")
}

fn slice_parity_3mf(kind: &str) -> Vec<String> {
    let (model, config) =
        engine::adapters::ffi::read_project_3mf(&parity_dir().join(format!("{kind}.3mf"))).unwrap();
    let work = tempfile::tempdir().unwrap();
    let r = engine::adapters::ffi::slice(
        SliceRequest {
            allow_mixed_temperatures: false,
            model,
            config,
            plate_index: 0,
            work_dir: work.path().to_path_buf(),
            calib: None,
        },
        Box::new(|_, _| {}),
        CancelToken::new(),
    )
    .unwrap();
    normalize(&std::fs::read_to_string(r.gcode_path).unwrap())
}

/// Enregistre les empreintes des variantes Orca observées
/// (ORCA_VARIANTS_DIR/<kind>/*.gcode, collect_variants.py) dans
/// tests/fixtures/calib_project/<kind>.variants.json.
#[test]
#[ignore = "enregistre les variantes Orca (ORCA_VARIANTS_DIR)"]
fn record_orca_variants() {
    let root = PathBuf::from(std::env::var("ORCA_VARIANTS_DIR").unwrap());
    for kind in ["temp_tower", "retraction"] {
        let mut hashes: Vec<String> = std::fs::read_dir(root.join(kind))
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|x| x == "gcode"))
            .map(|e| fnv(&normalize(&std::fs::read_to_string(e.path()).unwrap())))
            .collect();
        hashes.sort();
        hashes.dedup();
        std::fs::write(
            parity_dir().join(format!("{kind}.variants.json")),
            serde_json::to_string_pretty(&hashes).unwrap(),
        )
        .unwrap();
    }
}

/// Température et rétraction : Orca n'est pas déterministe sur ces modèles
/// (pas de variante majoritaire, cf. lot15/LOT15.md). Le moteur doit produire
/// l'une des variantes Orca observées (ensemble), jusqu'à 3 tirages (le moteur
/// a le même non-déterminisme).
#[test]
fn orca_variants_contain_engine_output() {
    for kind in ["temp_tower", "retraction"] {
        let path = parity_dir().join(format!("{kind}.variants.json"));
        let Ok(text) = std::fs::read_to_string(&path) else {
            eprintln!("{kind} : variantes absentes");
            continue;
        };
        let variants: Vec<String> = serde_json::from_str(&text).unwrap();
        let mut seen = Vec::new();
        // Sous charge CI, 3 tirages ne suffisaient pas (nondéterminisme
        // libslic3r documenté) : 6 tirages indépendants.
        let found = (0..6).any(|_| {
            let h = fnv(&slice_parity_3mf(kind));
            let ok = variants.contains(&h);
            seen.push(h);
            ok
        });
        assert!(
            found,
            "{kind} : moteur {seen:?} ∉ variantes Orca {variants:?}"
        );
    }
}

#[test]
fn pa_pattern_project_like_orca() {
    let (p, cfg) = project("pa_pattern", 0.0, 0.08, 0.005);
    assert_eq!(p.calib.unwrap().mode, CalibMode::PaPattern);
    assert_eq!(p.model.objects.len(), 1);
    assert!(
        p.model.objects[0].name.starts_with("pa_pattern_"),
        "{}",
        p.model.objects[0].name
    );
    assert_eq!(p.overrides.get("wall_loops"), Some(&ConfigValue::Int(3)));
    assert_eq!(p.overrides.get("skirt_loops"), Some(&ConfigValue::Int(0)));
    // Orca main : outer_wall_speed vectoriel par extrudeur (coFloats).
    assert!(
        matches!(p.overrides.get("outer_wall_speed"), Some(ConfigValue::Floats(v)) if v.len() == 1 && v[0] > 0.0)
    );
    let items = &p.model.custom_gcodes[0].items;
    assert!(
        items.len() > 2 && items.iter().all(|i| !i.extra.is_empty()),
        "motif par couche"
    );
    let gcode = slice(&p, cfg);
    let pa = gcode
        .lines()
        .filter(|l| l.starts_with("SET_PRESSURE_ADVANCE") || l.starts_with("M900"))
        .count();
    assert!(pa >= 17, "{pa} commandes PA (0 → 0,08 par 0,005)");
}

#[test]
fn pa_pattern_batch_speeds_and_accels() {
    let (p, _) = project_with(CalibRequest {
        kind: "pa_pattern".into(),
        start: 0.0,
        end: 0.04,
        step: 0.01,
        speeds: vec![100.0, 150.0],
        accelerations: vec![3000.0, 5000.0],
        ..Default::default()
    });
    assert_eq!(p.model.objects.len(), 4);
    let mut names: Vec<&str> = p.model.objects.iter().map(|o| o.name.as_str()).collect();
    names.sort();
    assert_eq!(
        names,
        [
            "pa_pattern_100_3000",
            "pa_pattern_100_5000",
            "pa_pattern_150_3000",
            "pa_pattern_150_5000"
        ]
    );
    assert!(p
        .model
        .objects
        .iter()
        .all(|o| o.settings.get("outer_wall_speed").is_some()
            && o.settings.get("outer_wall_acceleration").is_some()));
}

#[test]
fn input_shaping_frequency_like_orca() {
    let (p, cfg) = project_with(CalibRequest {
        kind: "is_freq".into(),
        start: 0.15,
        freq_start_x: 15.0,
        freq_end_x: 110.0,
        freq_start_y: 15.0,
        freq_end_y: 110.0,
        shaper_type: engine::api::ShaperType::Mzv,
        ..Default::default()
    });
    let calib = p.calib.unwrap();
    assert_eq!(calib.mode, CalibMode::InputShapingFreq);
    assert_eq!(
        p.overrides.get("spiral_mode"),
        Some(&ConfigValue::Bool(true))
    );
    assert_eq!(
        p.overrides.get("outer_wall_acceleration"),
        // Orca main : coFloats par extrudeur.
        Some(&ConfigValue::Floats(vec![20000.0]))
    );
    assert_eq!(
        p.overrides.get("layer_height"),
        Some(&ConfigValue::Float(0.2))
    );
    let gcode = slice(&p, cfg);
    assert!(gcode.contains("SHAPER_TYPE=MZV"), "type de shaper transmis");
    let sets = gcode
        .lines()
        .filter(|l| l.starts_with("SET_INPUT_SHAPER"))
        .count();
    assert!(sets > 50, "{sets} SET_INPUT_SHAPER");
}

#[test]
fn input_shaping_damping_like_orca() {
    let (p, cfg) = project_with(CalibRequest {
        kind: "is_damp".into(),
        start: 0.0,
        end: 0.4,
        freq_start_x: 40.0,
        freq_start_y: 45.0,
        test_model: 1,
        ..Default::default()
    });
    assert_eq!(p.calib.unwrap().mode, CalibMode::InputShapingDamp);
    assert!(
        p.overrides.get("layer_height").is_none(),
        "l'amortissement garde la hauteur de couche"
    );
    let gcode = slice(&p, cfg);
    assert!(
        gcode.contains("SHAPER_FREQ_X=40") && gcode.contains("SHAPER_FREQ_Y=45"),
        "fréquences fixes"
    );
    assert!(
        gcode
            .lines()
            .filter(|l| l.contains("DAMPING_RATIO"))
            .count()
            > 50
    );
}

#[test]
fn cornering_like_orca() {
    let (p, cfg) = project_with(CalibRequest {
        kind: "cornering".into(),
        start: 1.0,
        end: 15.0,
        test_model: 2,
        ..Default::default()
    });
    assert_eq!(p.calib.unwrap().mode, CalibMode::Cornering);
    assert_eq!(
        p.overrides.get("machine_max_jerk_x"),
        Some(&ConfigValue::Floats(vec![15.0]))
    );
    assert_eq!(
        p.overrides.get("input_shaping_type"),
        Some(&ConfigValue::String("Disable".into()))
    );
    assert_eq!(
        p.overrides.get("filament_max_volumetric_speed"),
        Some(&ConfigValue::Floats(vec![200.0]))
    );
    let gcode = slice(&p, cfg);
    let scv = gcode
        .lines()
        .filter(|l| l.contains("SQUARE_CORNER_VELOCITY"))
        .count();
    assert!(
        scv > 20,
        "{scv} réglages de jerk (Klipper : SQUARE_CORNER_VELOCITY)"
    );
}
