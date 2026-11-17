//! Maillage STEP avec les réglages du dialogue StepMeshDialog d'Orca
//! (déflexion linéaire / angulaire, découpe des compounds) : Step::load +
//! Step::mesh comme Model::read_from_step. Le CLI d'Orca refuse les .step
//! (« Input file must have .stl, .obj, .amf(.xml) extension », make_ref.py) ;
//! la référence est le nombre de triangles que le dialogue d'Orca annonce
//! (Step::get_triangle_num) pour les mêmes réglages.
//! Exécution : cargo test -p engine --features ffi --test ffi_step_mesh

#![cfg(feature = "ffi")]

use std::path::{Path, PathBuf};

use engine::adapters::ffi::{load_step, StepMeshOptions};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/step_mesh")
        .join(name)
}

fn triangles(model: &engine::api::Model) -> usize {
    model
        .objects
        .iter()
        .flat_map(|o| &o.volumes)
        .map(|v| v.mesh.indices.len())
        .sum()
}

fn opts(linear: f64, angle: f64, split: bool) -> StepMeshOptions {
    StepMeshOptions {
        linear_deflection: linear,
        angle_deflection: angle,
        split_compound: split,
    }
}

#[test]
fn deflection_sets_the_mesh_like_orca_dialog() {
    let mut previous = 0;
    // Valeurs du dialogue : défaut (0,003 / 0,5), puis plus grossier et plus fin.
    for (linear, angle) in [(0.1, 1.0), (0.003, 0.5), (0.001, 0.1)] {
        let (model, dialog) =
            load_step(&fixture("cylinder.step"), opts(linear, angle, false)).unwrap();
        let n = triangles(&model);
        assert_eq!(
            n as u32, dialog,
            "{linear}/{angle} : triangles = nombre annoncé par le dialogue"
        );
        assert!(
            n > previous,
            "{linear}/{angle} : {n} triangles (précédent {previous})"
        );
        previous = n;
        // Cylindre Ø20 × 20 : dimensions conservées.
        let v = &model.objects[0].volumes[0];
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for p in &v.mesh.vertices {
            for a in 0..3 {
                lo[a] = lo[a].min(p[a]);
                hi[a] = hi[a].max(p[a]);
            }
        }
        assert!((hi[2] - lo[2] - 20.0).abs() < 1e-3 && (hi[0] - lo[0] - 20.0).abs() < 0.05);
    }
    // Défauts = import sans réglage (même maillage que load_model).
    let (model, _) = load_step(&fixture("cylinder.step"), StepMeshOptions::default()).unwrap();
    let plain = engine::adapters::ffi::load_model(&fixture("cylinder.step")).unwrap();
    assert_eq!(triangles(&model), triangles(&plain));
}

#[test]
fn split_compound_like_orca() {
    let (merged, _) = load_step(&fixture("compound.step"), StepMeshOptions::default()).unwrap();
    let (split, dialog) = load_step(&fixture("compound.step"), opts(0.003, 0.5, true)).unwrap();
    // Step::mesh crée toujours UN objet ; la découpe donne une pièce (volume)
    // par solide (« <nom>-SOLID-<n> »), sinon le compound reste une pièce.
    assert_eq!(
        (merged.objects.len(), merged.objects[0].volumes.len()),
        (1, 1),
        "compound non découpé"
    );
    assert_eq!(
        (split.objects.len(), split.objects[0].volumes.len()),
        (1, 2),
        "un volume par solide"
    );
    assert!(
        split.objects[0]
            .volumes
            .iter()
            .all(|v| v.name.contains("-SOLID-")),
        "noms d'Orca"
    );
    assert_eq!(triangles(&split), triangles(&merged));
    assert_eq!(triangles(&split) as u32, dialog);
}

#[test]
fn invalid_deflection_is_refused() {
    assert!(load_step(&fixture("cylinder.step"), opts(0.0, 0.5, false)).is_err());
}
