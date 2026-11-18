//! Infra de spawn du worker (T122) : succès, crash contenu, timeout, binaire
//! introuvable. Exercée via le binaire `engine-worker self-test [--crash|--hang]`
//! (protocole T018), sans dépendre du pipeline de tranchage réel.
//!
//! Le binaire `engine-worker` a `required-features = ["ffi"]` : il n'est bâti
//! que dans la passe FFI (`cargo build -p engine --bin engine-worker
//! --features ffi`). Quand il est absent, les tests qui le lancent sont ignorés
//! proprement (même logique que les tests moteur `#[cfg(feature = "ffi")]`),
//! plutôt que d'échouer ou de rebâtir en cargo imbriqué (risque d'interblocage).

use std::path::PathBuf;
use std::time::Duration;

use backend::engine::scene::{
    arrange_at, repair_at, ArrangeWorkerRequest, MAX_SCENE_REQUEST_BYTES,
};
use backend::engine::{run_worker_at, run_worker_warm_at, WorkerError};
use engine::api::{ArrangeParams, BuildVolume, Model, ModelInstance, TriangleMesh};

/// Chemin du binaire `engine-worker` s'il est disponible : `ENGINE_WORKER_BIN`
/// sinon à côté du binaire de test (répertoire de profil `target/<profil>/`).
fn worker_bin() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("ENGINE_WORKER_BIN") {
        let p = PathBuf::from(p);
        return p.exists().then_some(p);
    }
    let profile_dir = std::env::current_exe()
        .ok()?
        .parent()? // deps/
        .parent()? // target/<profil>/
        .to_path_buf();
    let bin = profile_dir.join("engine-worker");
    bin.exists().then_some(bin)
}

/// Récupère le binaire ou ignore le test (message explicite) s'il n'est pas bâti.
macro_rules! worker_or_skip {
    () => {
        match worker_bin() {
            Some(bin) => bin,
            None => {
                eprintln!(
                    "engine-worker absent — test ignoré \
                     (bâtir : cargo build -p engine --bin engine-worker --features ffi)"
                );
                return;
            }
        }
    };
}

#[tokio::test]
async fn self_test_success_captures_stdout() {
    let bin = worker_or_skip!();
    let dir = tempfile::tempdir().unwrap();
    let out = run_worker_at(
        &bin,
        &["self-test", &dir.path().to_string_lossy()],
        Duration::from_secs(30),
    )
    .await
    .expect("self-test aboutit");

    let text = String::from_utf8_lossy(&out);
    // Le protocole émet la ligne résultat `R …` avec le chemin du G-code de test.
    assert!(text.contains("R "), "ligne résultat R présente : {text}");
    assert!(
        text.contains("self-test.gcode"),
        "chemin du G-code : {text}"
    );
}

#[tokio::test]
async fn crash_maps_to_crashed_error() {
    let bin = worker_or_skip!();
    let dir = tempfile::tempdir().unwrap();
    let err = run_worker_at(
        &bin,
        &["self-test", "--crash", &dir.path().to_string_lossy()],
        Duration::from_secs(30),
    )
    .await
    .expect_err("un crash worker doit remonter en erreur, pas tuer le parent");

    assert!(
        matches!(err, WorkerError::Crashed { .. }),
        "crash (signal) mappé en Crashed : {err:?}"
    );
}

#[tokio::test]
async fn hang_maps_to_timeout() {
    let bin = worker_or_skip!();
    let dir = tempfile::tempdir().unwrap();
    let err = run_worker_at(
        &bin,
        &["self-test", "--hang", &dir.path().to_string_lossy()],
        Duration::from_millis(800),
    )
    .await
    .expect_err("un worker bloqué doit expirer");

    assert!(
        matches!(err, WorkerError::Timeout(_)),
        "blocage mappé en Timeout : {err:?}"
    );
}

/// Worker préchauffé (lot 18) : sous-commande transmise sur stdin, un process
/// neuf par opération (le suivant est relancé à chaque prise), même mappage
/// succès / crash / timeout qu'au lancement à froid.
#[tokio::test]
async fn warm_worker_runs_each_operation_in_a_fresh_process() {
    let bin = worker_or_skip!();
    let dir = tempfile::tempdir().unwrap();
    let work = dir.path().to_string_lossy().to_string();
    for _ in 0..3 {
        let out = run_worker_warm_at(&bin, &["self-test", &work], Duration::from_secs(30))
            .await
            .expect("self-test via worker préchauffé");
        let text = String::from_utf8_lossy(&out);
        assert!(text.contains("self-test.gcode"), "ligne R : {text}");
    }
    let err = run_worker_warm_at(
        &bin,
        &["self-test", "--crash", &work],
        Duration::from_secs(30),
    )
    .await
    .expect_err("crash remonté");
    assert!(matches!(err, WorkerError::Crashed { .. }), "{err:?}");
    let err = run_worker_warm_at(
        &bin,
        &["self-test", "--hang", &work],
        Duration::from_millis(800),
    )
    .await
    .expect_err("blocage expiré");
    assert!(matches!(err, WorkerError::Timeout(_)), "{err:?}");
    // Après crash et timeout, l'emplacement est toujours servi.
    let out = run_worker_warm_at(&bin, &["config-count"], Duration::from_secs(30))
        .await
        .expect("config-count après incidents");
    assert!(
        String::from_utf8_lossy(&out)
            .lines()
            .last()
            .unwrap_or("")
            .trim()
            .parse::<usize>()
            .is_ok(),
        "nombre d'options : {}",
        String::from_utf8_lossy(&out)
    );
}

#[tokio::test]
async fn missing_binary_maps_to_spawn_error() {
    // Chemin explicite inexistant : n'altère aucune variable d'environnement
    // partagée (pas de course avec les tests parallèles).
    let missing = PathBuf::from("/nonexistent/engine-worker-does-not-exist");
    let err = run_worker_at(&missing, &["config-count"], Duration::from_secs(5))
        .await
        .expect_err("un binaire introuvable doit remonter une erreur de lancement");

    assert!(
        matches!(err, WorkerError::Spawn { .. }),
        "binaire introuvable mappé en Spawn : {err:?}"
    );
}

fn cube_mesh() -> TriangleMesh {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../engine/tests/fixtures/cube20.stl");
    let mesh = backend::mesh::parse_stl(&std::fs::read(path).unwrap()).unwrap();
    TriangleMesh {
        vertices: mesh
            .positions
            .chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect(),
        indices: mesh
            .indices
            .chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect(),
    }
}

fn bed_100(excluded: Vec<Vec<[f64; 2]>>) -> BuildVolume {
    BuildVolume {
        bed_shape: vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]],
        max_height: 200.0,
        excluded,
    }
}

/// Protocole arrange réel : ids et plateaux conservés, matrices complètes
/// retournées, zone exclue transmise au nester.
#[tokio::test]
async fn arrange_returns_real_matrices_and_preserves_plates() {
    let bin = worker_or_skip!();
    let mut model = Model::default();
    for index in 0..3 {
        let object = model.add_object(format!("cube-{index}"), cube_mesh());
        object.instances = vec![ModelInstance {
            matrix: glam::DMat4::from_rotation_z(0.3),
            plate_index: if index == 1 { 1 } else { 0 },
            printable: true,
        }];
    }
    let response = arrange_at(
        &bin,
        &ArrangeWorkerRequest {
            model,
            bed: bed_100(vec![vec![
                [40.0, 40.0],
                [60.0, 40.0],
                [60.0, 60.0],
                [40.0, 60.0],
            ]]),
            params: ArrangeParams {
                clearance: 2.0,
                use_machine_clearance: false,
                allow_rotations: false,
                orca_cli_mode: false,
                print_config: None,
                is_seq_print: false,
            },
            instance_ids: vec!["a".into(), "b".into(), "c".into()],
        },
        Duration::from_secs(30),
    )
    .await
    .expect("arrangement réel");

    assert_eq!(
        response
            .placements
            .iter()
            .map(|p| p.id.as_str())
            .collect::<Vec<_>>(),
        ["a", "b", "c"]
    );
    assert_eq!(response.placements[1].plate_index, 1);
    assert_eq!(response.placements[0].plate_index, 0);
    assert_eq!(response.placements[2].plate_index, 0);
    for placement in &response.placements {
        assert_eq!(placement.matrix.len(), 16);
        assert!(placement.matrix.iter().all(|v| v.is_finite()));
        // La rotation initiale n'est pas remplacée par une matrice identité.
        assert!(placement.matrix[0].abs() > 0.0 && placement.matrix[1].abs() > 0.0);
    }
}

/// Prisme régulier à 64 côtés (sommet à 0°), comme le golden `cylinder`.
fn cylinder_64_mesh() -> TriangleMesh {
    const N: u32 = 64;
    let mut vertices = Vec::new();
    for z in [0.0f32, 10.0] {
        for k in 0..N {
            let a = k as f32 * std::f32::consts::TAU / N as f32;
            vertices.push([10.0 * a.cos(), 10.0 * a.sin(), z]);
        }
    }
    vertices.push([0.0, 0.0, 0.0]);
    vertices.push([0.0, 0.0, 10.0]);
    let (bottom, top) = (2 * N, 2 * N + 1);
    let mut indices = Vec::new();
    for k in 0..N {
        let n = (k + 1) % N;
        indices.push([bottom, n, k]);
        indices.push([top, N + k, N + n]);
        indices.push([k, n, N + n]);
        indices.push([k, N + n, N + k]);
    }
    TriangleMesh { vertices, indices }
}

async fn arrange_cylinder(bin: &std::path::Path, allow_rotations: bool) -> [f64; 16] {
    let mut model = Model::default();
    model.add_object("cylinder", cylinder_64_mesh());
    let response = arrange_at(
        bin,
        &ArrangeWorkerRequest {
            model,
            bed: bed_100(vec![]),
            params: ArrangeParams {
                clearance: 0.0,
                use_machine_clearance: false,
                allow_rotations,
                orca_cli_mode: false,
                print_config: None,
                is_seq_print: false,
            },
            instance_ids: vec!["cyl".into()],
        },
        Duration::from_secs(30),
    )
    .await
    .expect("arrangement réel");
    response.placements[0].matrix
}

/// `allow_rotations` traverse JSON → Rust → cxx → `ArrangeParams` Orca : le
/// 64-gone est tourné d'un demi-pas (π/64, boîte englobante minimale) quand
/// les rotations sont permises, et reste non pivoté sinon.
#[tokio::test]
async fn arrange_allow_rotations_turns_the_64_gon_by_half_a_step() {
    let bin = worker_or_skip!();
    let step = std::f64::consts::TAU / 64.0;

    let fixed = arrange_cylinder(&bin, false).await;
    assert!(
        fixed[1].abs() < 1e-12 && (fixed[0] - 1.0).abs() < 1e-12,
        "{fixed:?}"
    );

    let rotated = arrange_cylinder(&bin, true).await;
    let angle = rotated[1].atan2(rotated[0]);
    let residue = angle.rem_euclid(step);
    assert!(
        (residue - step / 2.0).abs() < 1e-3,
        "angle {angle} rad, résidu {residue} (attendu {})",
        step / 2.0
    );
    // Rotation pure autour de Z : ni scale ni basculement.
    assert!((rotated[0].hypot(rotated[1]) - 1.0).abs() < 1e-9);
    assert!((rotated[10] - 1.0).abs() < 1e-12);
}

/// Protocole repair réel : le rapport admesh et le WSMh réparé sont renvoyés,
/// et le maillage rebouché redevient étanche.
#[tokio::test]
async fn repair_persists_bounded_wsmh_and_admesh_report() {
    let bin = worker_or_skip!();
    let mut broken = cube_mesh();
    broken.indices.pop();
    broken.indices.push([0, 0, 1]);
    let output = repair_at(&bin, &broken, Duration::from_secs(30))
        .await
        .expect("réparation réelle");

    assert!(output.repairs.repaired_anything(), "{:?}", output.repairs);
    assert_eq!(output.mesh.triangle_count(), 12);
    assert_eq!(output.display_bytes[0..4], *b"WSMh");
    let report = backend::scene::analyze_mesh(&backend::mesh::Mesh {
        positions: output.mesh.vertices.iter().flatten().copied().collect(),
        normals: Vec::new(),
        indices: output.mesh.indices.iter().flatten().copied().collect(),
    });
    assert_eq!(report.open_edges, 0);
    assert_eq!(report.degenerate, 0);
}

/// La taille est refusée sur métadonnées avant désérialisation/allocation.
#[tokio::test]
async fn oversized_scene_request_is_rejected_before_json_decode() {
    let bin = worker_or_skip!();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("oversized.json");
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(MAX_SCENE_REQUEST_BYTES + 1).unwrap();
    let err = run_worker_at(
        &bin,
        &["arrange", &path.to_string_lossy()],
        Duration::from_secs(5),
    )
    .await
    .expect_err("requête excessive refusée");
    let WorkerError::Failed { stdout, .. } = err else {
        panic!("erreur applicative attendue : {err:?}");
    };
    assert!(String::from_utf8_lossy(&stdout).contains("E invalid_model"));
}
