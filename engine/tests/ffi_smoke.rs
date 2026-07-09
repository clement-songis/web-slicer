//! Smoke test du bridge cxx (T012) : chaîne include + link + exceptions.
//! Exécution : `cargo test -p engine --features ffi` (devShell requis).

#![cfg(feature = "ffi")]

mod common;

#[test]
fn charge_un_stl_et_compte_les_triangles() {
    let n = engine::adapters::ffi::model_triangle_count(&common::fixture("cube20.stl"))
        .expect("libslic3r charge le STL");
    assert_eq!(n, 12, "cube = 12 triangles");
}

#[test]
fn erreur_propre_sur_fichier_inexistant() {
    let err = engine::adapters::ffi::model_triangle_count(std::path::Path::new(
        "/nulle/part/fantome.stl",
    ))
    .expect_err("doit échouer");
    assert_eq!(err.code, engine::api::EngineErrorCode::InvalidModel);
}

#[test]
fn registre_runtime_aligne_avec_le_registre_genere() {
    use engine::params::{ParamGroup, REGISTRY};
    let runtime = engine::adapters::ffi::print_config_option_count();
    let generated = REGISTRY
        .iter()
        .filter(|p| {
            matches!(
                p.group,
                ParamGroup::Fff | ParamGroup::Common | ParamGroup::Sla
            )
        })
        .count();
    assert_eq!(runtime, generated, "parité exacte C++ ↔ registre généré");
}

#[test]
fn charge_la_scene_complete_stl() {
    let model = engine::adapters::ffi::load_model(&common::fixture("cube20.stl")).unwrap();
    assert_eq!(model.objects.len(), 1);
    let obj = &model.objects[0];
    assert_eq!(obj.volumes.len(), 1);
    assert_eq!(obj.volumes[0].mesh.triangle_count(), 12);
    assert!(!obj.instances.is_empty(), "AddDefaultInstances");
}

#[test]
fn charge_un_projet_3mf_orca() {
    let model = engine::adapters::ffi::load_model(&common::fixture("orca_project.3mf")).unwrap();
    assert!(!model.is_empty());
    let n: usize = model
        .objects
        .iter()
        .flat_map(|o| &o.volumes)
        .map(|v| v.mesh.triangle_count())
        .sum();
    assert!(n > 0, "géométrie présente dans le 3MF");
}

#[test]
fn convertit_le_step_en_maillage() {
    let mesh = engine::adapters::ffi::convert_to_mesh(&common::fixture("cube20.step"))
        .expect("libslic3r (OCCT) lit le STEP de fixture");
    assert!(!mesh.is_empty());
    let (lo, hi) = mesh.bounding_box().unwrap();
    for i in 0..3 {
        assert!(
            (hi[i] - lo[i] - 20.0).abs() < 0.5,
            "cube ~20 mm sur l'axe {i}: {lo:?} {hi:?}"
        );
    }
}

#[test]
fn aller_retour_projet_3mf_avec_config() {
    use engine::api::{ConfigValue, DynamicPrintConfig};
    let model = engine::adapters::ffi::load_model(&common::fixture("cube20.stl")).unwrap();
    let mut config = DynamicPrintConfig::new();
    config
        .set("layer_height", ConfigValue::Float(0.28))
        .unwrap();
    config
        .set(
            "sparse_infill_pattern",
            ConfigValue::String("gyroid".into()),
        )
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("roundtrip.3mf");
    engine::adapters::ffi::write_project_3mf(&model, &config, &out).expect("écrit");
    let (back_model, back_config) = engine::adapters::ffi::read_project_3mf(&out).expect("relit");
    let n: usize = back_model
        .objects
        .iter()
        .flat_map(|o| &o.volumes)
        .map(|v| v.mesh.triangle_count())
        .sum();
    assert_eq!(n, 12, "géométrie préservée");
    assert_eq!(
        back_config.get("layer_height"),
        Some(&ConfigValue::Float(0.28)),
        "config embarquée préservée"
    );
    assert_eq!(
        back_config.get("sparse_infill_pattern"),
        Some(&ConfigValue::String("gyroid".into()))
    );
}
