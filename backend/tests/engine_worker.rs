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

use backend::engine::{run_worker_at, WorkerError};

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
