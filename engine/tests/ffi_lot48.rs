//! Lot 48 (audit 46) : contrôles moteur.
//! - GCHK-a : trajets hors du plateau (bits 2 / 3 de `gcode_check_result`,
//!   valables aussi en mono-buse) remontés comme la GUI d'Orca
//!   (`PartPlate::is_slice_result_ready_for_print`) : tranchage réussi,
//!   `gcode_check` renseigné ; impression / export refusés par le serveur.
//! - VLH-a : profil de hauteur de couche variable qu'Orca ignore
//!   (`PrintObject::update_layer_height_profile`) signalé par la validation.
//!
//! `cargo test -p engine --features ffi --test ffi_lot48`.

#![cfg(feature = "ffi")]

mod common;

use std::path::Path;

use engine::api::{CancelToken, ConfigValue, ProgressSink, SliceRequest, SliceResult};

fn multi_fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/multi_filament")
        .join(name)
}

fn slice(
    model: engine::api::Model,
    config: engine::api::DynamicPrintConfig,
    work: &Path,
) -> SliceResult {
    let req = SliceRequest {
        allow_mixed_temperatures: false,
        model,
        config,
        plate_index: 0,
        work_dir: work.to_path_buf(),
        calib: None,
    };
    let progress: ProgressSink = Box::new(|_, _| {});
    engine::adapters::ffi::slice(req, progress, CancelToken::new())
        .unwrap_or_else(|e| panic!("tranchage réussi attendu (GUI d'Orca) : {e:?}"))
}

/// GCHK-a : X1C (mono-buse, plateau 256 × 256) avec la tour d'essuyage en
/// y = 240 : la tour déborde au-delà de Y = 256 + 2 mm. Le CLI d'Orca 2.4.1
/// refuse le même 3MF (−102, vérifié : wipe_tower_y = 240 dans
/// bambu_x1c.3mf) ; la GUI garde l'aperçu mais bloque impression / export
/// (gcode_check_result.error_code bit 2). Sans déplacement : aucun contrôle
/// (CLI : 0).
#[test]
fn x1c_prime_tower_beyond_the_plate_is_reported_like_orca_gui() {
    let (model, mut config) =
        engine::adapters::ffi::read_project_3mf(&multi_fixture("bambu_x1c.3mf")).expect("3MF");

    let work = tempfile::tempdir().unwrap();
    let inside = slice(model.clone(), config.clone(), work.path());
    assert!(
        inside.gcode_check.is_none(),
        "tour dans le plateau : aucun contrôle ({:?})",
        inside.gcode_check
    );

    config
        .set("wipe_tower_y", ConfigValue::Floats(vec![240.0]))
        .unwrap();
    let work = tempfile::tempdir().unwrap();
    let outside = slice(model, config, work.path());
    let check = outside
        .gcode_check
        .as_ref()
        .expect("gcode_check attendu : tour hors du plateau (CLI −102)");
    assert_eq!(check.error_code & 0b100, 0b100, "bit 2 : {check:?}");
    assert_eq!(
        check.error_code & 0b11,
        0,
        "pas de bit multi-buses : {check:?}"
    );
    assert_eq!(
        check.message, "A G-code path goes beyond the plate boundaries.",
        "texte de la GUI (ToolpathOutside) : {check:?}"
    );
    let issue = check
        .issues
        .iter()
        .find(|i| i.kind == "plate_area")
        .unwrap_or_else(|| panic!("détail plate_area attendu : {check:?}"));
    assert!(issue.nozzle.is_empty(), "{issue:?}");
    assert!(
        !issue.filaments.is_empty(),
        "filaments de la tour : {issue:?}"
    );
    // Le G-code reste écrit (aperçu) et extrude bien au-delà de Y = 258.
    let text = std::fs::read_to_string(&outside.gcode_path).unwrap();
    let max_y = text
        .lines()
        .filter(|l| l.starts_with("G1 ") && l.contains(" E"))
        .filter_map(|l| {
            l.split_whitespace()
                .find_map(|w| w.strip_prefix('Y'))
                .and_then(|v| v.parse::<f64>().ok())
        })
        .fold(f64::MIN, f64::max);
    assert!(
        max_y > 258.0,
        "extrusion au-delà du plateau : max Y = {max_y}"
    );
}

/// GCHK-a, plateau rond : SeeMeCNC Artemis (delta, rayon 145 mm) avec la tour
/// en (15, 220) — hors du plateau (Y 214–252). Le CLI refuse (−102) ; la GUI
/// bloque aussi l'impression. Le lot 47 ignorait le bit 2 pour ne pas faire
/// échouer ce tranchage : depuis le lot 47c le tranchage réussit (G-code
/// identique, seemecnc_artemis_change_filament_gcode_and_type2_tower_match_orca)
/// et le contrôle est remonté.
#[test]
fn delta_round_bed_tower_outside_is_reported() {
    let (model, config) =
        engine::adapters::ffi::read_project_3mf(&multi_fixture("seemecnc_artemis.3mf"))
            .expect("3MF");
    let work = tempfile::tempdir().unwrap();
    let result = slice(model, config, work.path());
    let check = result
        .gcode_check
        .as_ref()
        .expect("gcode_check attendu (CLI −102)");
    assert_eq!(check.error_code, 0b100, "{check:?}");
    assert_eq!(
        check.message,
        "A G-code path goes beyond the plate boundaries."
    );
}

fn cube_project() -> (engine::api::Model, engine::api::DynamicPrintConfig) {
    let (model, mut config) =
        engine::adapters::ffi::read_project_3mf(&common::fixture("orca_project.3mf"))
            .expect("lecture du projet 3MF");
    config
        .set(
            "before_layer_change_gcode",
            ConfigValue::String("G92 E0".into()),
        )
        .unwrap();
    (model, config)
}

fn validate(
    model: engine::api::Model,
    config: engine::api::DynamicPrintConfig,
) -> engine::api::ValidateResult {
    let work = tempfile::tempdir().unwrap();
    let req = SliceRequest {
        allow_mixed_temperatures: false,
        model,
        config,
        plate_index: 0,
        work_dir: work.path().to_path_buf(),
        calib: None,
    };
    let path = work.path().join("validate.json");
    std::fs::write(&path, serde_json::to_vec(&req).unwrap()).unwrap();
    engine::adapters::ffi::validate_in_worker(&path).expect("validation")
}

fn ignored_profile(result: &engine::api::ValidateResult) -> Option<&engine::api::ValidateIssue> {
    result
        .issues
        .iter()
        .find(|i| i.params.first().map(String::as_str) == Some("layer_height_profile_ignored"))
}

/// VLH-a : profil arrêté à Z = 10 sur un cube plus haut → Orca l'efface sans
/// message (update_layer_height_profile) ; la validation l'annonce
/// (avertissement, objet désigné, cause « top »). Un profil qui atteint le
/// sommet n'est pas signalé ; une première hauteur ≠ hauteur de première
/// couche l'est (cause « first_layer »).
#[test]
fn truncated_layer_height_profile_is_reported_by_validation() {
    let (mut model, config) = cube_project();
    let first = match config.get("initial_layer_print_height") {
        Some(ConfigValue::Float(v)) => *v,
        other => panic!("initial_layer_print_height : {other:?}"),
    };

    // Hauteur de l'objet : sommet du profil adaptatif (couvre l'objet comme Orca).
    let height = {
        let req = engine::api::layer_profile::LayerProfileRequest {
            model: model.clone(),
            config: config.clone(),
            op: engine::api::layer_profile::LayerProfileOp::Adaptive { quality: 0.5 },
        };
        let profile = engine::adapters::ffi::layer_profile(&req).expect("profil adaptatif");
        profile[profile.len() - 2]
    };
    assert!(height > 10.5, "objet assez haut : {height}");

    model.objects[0].layer_height_profile = vec![0.0, first, height, 0.2];
    let ok = validate(model.clone(), config.clone());
    assert!(
        ignored_profile(&ok).is_none(),
        "profil complet : {:?}",
        ok.issues
    );

    model.objects[0].layer_height_profile = vec![0.0, first, 10.0, 0.1];
    let bad = validate(model.clone(), config.clone());
    let issue =
        ignored_profile(&bad).unwrap_or_else(|| panic!("avertissement attendu : {:?}", bad.issues));
    assert!(issue.is_warning, "{issue:?}");
    assert_eq!(issue.object_index, Some(0), "{issue:?}");
    assert_eq!(issue.params[1], "top");
    assert!((issue.params[2].parse::<f64>().unwrap() - 10.0).abs() < 1e-6);
    assert!((issue.params[3].parse::<f64>().unwrap() - height).abs() < 1e-3);
    assert!(
        issue.message.contains("s'arrête à Z = 10.00 mm"),
        "{}",
        issue.message
    );

    model.objects[0].layer_height_profile = vec![0.0, first + 0.05, height, 0.2];
    let first_bad = validate(model, config);
    let issue = ignored_profile(&first_bad)
        .unwrap_or_else(|| panic!("avertissement attendu : {:?}", first_bad.issues));
    assert_eq!(issue.params[1], "first_layer");
}

/// VLH-a, outil (lissage / pinceau) : comme LayersEditing d'Orca, qui ne
/// travaille que sur un profil validé par update_layer_height_profile, un
/// profil tronqué reçu est d'abord remplacé par celui des réglages : le
/// résultat couvre toujours tout l'objet (plus de profil arrêté à Z = 10).
#[test]
fn layer_tools_never_return_a_truncated_profile() {
    use engine::api::layer_profile::{LayerEditAction, LayerProfileOp, LayerProfileRequest};
    let (model, config) = cube_project();
    let request = |op| LayerProfileRequest {
        model: model.clone(),
        config: config.clone(),
        op,
    };
    let adaptive =
        engine::adapters::ffi::layer_profile(&request(LayerProfileOp::Adaptive { quality: 0.5 }))
            .unwrap();
    let height = adaptive[adaptive.len() - 2];
    let truncated = vec![0.0, 0.2, 10.0, 0.2];
    let smoothed = engine::adapters::ffi::layer_profile(&request(LayerProfileOp::Smooth {
        profile: truncated.clone(),
        radius: 5,
        keep_min: false,
    }))
    .unwrap();
    assert!(
        (smoothed[smoothed.len() - 2] - height).abs() < 1e-3,
        "lissage : sommet {} ≠ {height}",
        smoothed[smoothed.len() - 2]
    );
    let adjusted = engine::adapters::ffi::layer_profile(&request(LayerProfileOp::Adjust {
        profile: truncated,
        z: 5.0,
        delta: -0.05,
        band_width: 2.0,
        action: LayerEditAction::Decrease,
    }))
    .unwrap();
    assert!(
        (adjusted[adjusted.len() - 2] - height).abs() < 1e-3,
        "pinceau : sommet {} ≠ {height}",
        adjusted[adjusted.len() - 2]
    );
}
