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

#[test]
fn repare_un_maillage_troue() {
    // cube valide + un triangle dégénéré (deux sommets identiques) :
    // admesh le supprime (degenerate_facets/facets_removed)
    let mut broken = engine::adapters::ffi::load_model(&common::fixture("cube20.stl"))
        .unwrap()
        .objects
        .remove(0)
        .volumes
        .remove(0)
        .mesh;
    broken.indices.push([0, 0, 1]);
    let (repaired, report) = engine::adapters::ffi::repair_mesh(&broken).expect("répare");
    assert!(!repaired.is_empty());
    assert!(
        report.repaired_anything(),
        "triangle dégénéré détecté/supprimé : {report:?}"
    );
}

#[test]
fn arrange_quatre_cubes_sans_collision() {
    use engine::api::{ArrangeParams, BuildVolume, Model};
    let cube = engine::adapters::ffi::load_model(&common::fixture("cube20.stl")).unwrap();
    let mut model = Model::default();
    for _ in 0..4 {
        model.objects.extend(cube.objects.clone());
    }
    let bed = BuildVolume {
        bed_shape: vec![[0.0, 0.0], [200.0, 0.0], [200.0, 200.0], [0.0, 200.0]],
        max_height: 200.0,
        excluded: vec![],
    };
    engine::adapters::ffi::arrange(&mut model, &bed, &ArrangeParams::default()).expect("arrange");
    // les 4 instances ont des positions distinctes dans le plateau
    let mut positions: Vec<(i64, i64)> = model
        .objects
        .iter()
        .flat_map(|o| &o.instances)
        .map(|i| {
            let t = i.matrix.w_axis;
            assert!((-100.0..=300.0).contains(&t.x), "x aberrant: {}", t.x);
            ((t.x * 10.0) as i64, (t.y * 10.0) as i64)
        })
        .collect();
    positions.sort_unstable();
    positions.dedup();
    assert_eq!(positions.len(), 4, "positions distinctes après arrangement");
}

#[test]
fn oriente_un_objet() {
    let mut object = engine::adapters::ffi::load_model(&common::fixture("overhang.stl"))
        .unwrap()
        .objects
        .remove(0);
    engine::adapters::ffi::orient(&mut object).expect("orient");
}
