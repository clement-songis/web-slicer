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
