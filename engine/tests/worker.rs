//! Protocole du process `engine-worker` (T018) : progression par pipe,
//! annulation par kill, crash contenu. Exécution :
//! `cargo test -p engine --features ffi` (le binaire engine-worker doit être
//! bâti — `cargo test` le construit).
//!
//! Ces tests exercent le protocole via `self-test`, indépendamment du
//! pipeline de tranchage réel (branché en T019).

#![cfg(feature = "ffi")]

use std::sync::{Arc, Mutex};

use engine::adapters::ffi::worker;
use engine::api::{CancelToken, EngineErrorCode, ProgressSink};

/// Collecte les ratios de progression observés.
fn collecting_sink() -> (ProgressSink, Arc<Mutex<Vec<f32>>>) {
    let seen = Arc::new(Mutex::new(Vec::<f32>::new()));
    let clone = Arc::clone(&seen);
    let sink: ProgressSink = Box::new(move |_phase, ratio| {
        clone.lock().unwrap().push(ratio);
    });
    (sink, seen)
}

#[test]
fn monotonic_progress_and_result() {
    let work = tempfile::tempdir().unwrap();
    let (sink, seen) = collecting_sink();
    let cancel = CancelToken::new();

    let json = worker::drive(
        &["self-test", &work.path().to_string_lossy()],
        &sink,
        &cancel,
    )
    .expect("self-test aboutit");

    let ratios = seen.lock().unwrap().clone();
    assert!(
        ratios.len() >= 2,
        "plusieurs progressions reçues : {ratios:?}"
    );
    assert!(
        ratios.windows(2).all(|w| w[1] >= w[0]),
        "progression monotone : {ratios:?}"
    );
    assert_eq!(ratios.last().copied(), Some(1.0), "atteint 100 %");
    assert!(
        json.contains("self-test.gcode"),
        "résultat contient le chemin du G-code : {json}"
    );
}

#[test]
fn cancellation_kills_worker() {
    let work = tempfile::tempdir().unwrap();
    let cancel = CancelToken::new();

    // Le sink annule dès la première progression : le worker (--hang) est
    // alors tué par le parent.
    let cancel_on_progress = cancel.clone();
    let sink: ProgressSink = Box::new(move |_phase, _ratio| {
        cancel_on_progress.cancel();
    });

    let err = worker::drive(
        &["self-test", "--hang", &work.path().to_string_lossy()],
        &sink,
        &cancel,
    )
    .expect_err("l'annulation doit interrompre le worker");
    assert_eq!(err.code, EngineErrorCode::Cancelled, "{err}");
}

#[test]
fn worker_crash_becomes_error() {
    let work = tempfile::tempdir().unwrap();
    let (sink, _seen) = collecting_sink();
    let cancel = CancelToken::new();

    let err = worker::drive(
        &["self-test", "--crash", &work.path().to_string_lossy()],
        &sink,
        &cancel,
    )
    .expect_err("un crash worker doit remonter en erreur, pas tuer le parent");
    assert_eq!(err.code, EngineErrorCode::EngineCrashed, "{err}");
}
