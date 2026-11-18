//! Protocole du process `engine-worker` (T018) : progression par pipe,
//! annulation par kill, crash contenu. Exécution :
//! `cargo test -p engine --features ffi` (le binaire engine-worker doit être
//! bâti — `cargo test` le construit).
//!
//! Ces tests exercent le protocole via `self-test`, indépendamment du
//! pipeline de tranchage réel (branché en T019).

#![cfg(feature = "ffi")]

mod common;

use std::io::Write;
use std::process::Command;
use std::sync::{Arc, Mutex};

use engine::adapters::ffi::worker;
use engine::api::{CancelToken, DisplayMesh, EngineErrorCode, ProgressSink};

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

#[test]
fn hung_worker_is_killed_after_the_timeout() {
    let work = tempfile::tempdir().unwrap();
    let (sink, _seen) = collecting_sink();
    let started = std::time::Instant::now();
    let err = worker::drive_with_timeout(
        &["self-test", "--hang", &work.path().to_string_lossy()],
        &sink,
        &CancelToken::new(),
        std::time::Duration::from_millis(800),
    )
    .expect_err("un worker bloqué doit être tué au délai");
    assert_eq!(err.code, EngineErrorCode::EngineCrashed, "{err}");
    assert!(err.message.contains("délai dépassé"), "{err}");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(10),
        "rendu peu après le délai : {:?}",
        started.elapsed()
    );
}

#[test]
fn verbose_stderr_does_not_block_the_worker() {
    // 1 Mio sur stderr (> pipe de 64 Kio) : sans lecture côté parent, le
    // worker resterait bloqué en écriture jusqu'au délai.
    let work = tempfile::tempdir().unwrap();
    let (sink, _seen) = collecting_sink();
    let json = worker::drive_with_timeout(
        &[
            "self-test",
            "--stderr-flood",
            &work.path().to_string_lossy(),
        ],
        &sink,
        &CancelToken::new(),
        std::time::Duration::from_secs(20),
    )
    .expect("le journal stderr est vidé en continu");
    assert!(json.contains("self-test.gcode"), "{json}");
}

#[cfg(target_os = "linux")]
#[test]
fn worker_exits_when_its_parent_dies() {
    // Un shell lance le worker (bloqué) en arrière-plan, affiche son PID puis
    // meurt : le worker, orphelin, doit se terminer de lui-même.
    let work = tempfile::tempdir().unwrap();
    let out = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "'{}' self-test --hang '{}' >/dev/null 2>&1 & echo $!; sleep 0.5",
            worker_binary(),
            work.path().display()
        ))
        .output()
        .expect("sh");
    let pid: u32 = String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse()
        .expect("PID du worker");
    let alive = |pid: u32| {
        std::fs::read_to_string(format!("/proc/{pid}/stat"))
            .ok()
            .and_then(|s| s.rsplit(')').next().map(|r| r.trim().to_string()))
            .is_some_and(|rest| !rest.starts_with('Z') && !rest.starts_with('X'))
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while alive(pid) && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let still_alive = alive(pid);
    if still_alive {
        let _ = Command::new("kill").arg("-9").arg(pid.to_string()).status();
    }
    assert!(!still_alive, "worker {pid} orphelin toujours vivant");
}

/// Chemin du binaire `engine-worker` bâti par cargo pour ce test d'intégration.
fn worker_binary() -> &'static str {
    env!("CARGO_BIN_EXE_engine-worker")
}

/// Extrait le chemin de la première ligne `R <chemin>` du protocole worker,
/// en ignorant le bruit de log libslic3r qui peut précéder sur stdout.
fn result_path(stdout: &[u8]) -> Option<std::path::PathBuf> {
    String::from_utf8_lossy(stdout)
        .lines()
        .find_map(|l| l.strip_prefix("R ").map(std::path::PathBuf::from))
}

/// Lance `load-model` sur une fixture, lit le WSMh pointé par la ligne `R`, le
/// supprime, et renvoie le maillage décodé.
fn run_load_model(path: &std::path::Path, format: &str) -> DisplayMesh {
    let out = Command::new(worker_binary())
        .arg("load-model")
        .arg(path)
        .arg(format)
        .output()
        .expect("lancement de engine-worker load-model");
    assert!(
        out.status.success(),
        "sortie nulle attendue ; stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let wsmh_path = result_path(&out.stdout).expect("ligne R <chemin> émise");
    let bytes = std::fs::read(&wsmh_path).expect("lecture du WSMh");
    let _ = std::fs::remove_file(&wsmh_path);
    DisplayMesh::decode(&bytes).expect("WSMh valide")
}

/// `load-model <fichier> <format>` produit un WSMh valide (ligne `R <chemin>`),
/// décodable, avec des triangles et une normale par sommet.
#[test]
fn load_model_emits_valid_wsmh() {
    let mesh = run_load_model(&common::fixture("cube20.stl"), "stl");
    // Le cube = 12 triangles → 36 indices ; 3 f32 de position par sommet.
    assert_eq!(mesh.indices.len(), 36, "12 triangles");
    assert_eq!(
        mesh.positions.len(),
        mesh.normals.len(),
        "une normale/sommet"
    );
    assert!(!mesh.positions.is_empty(), "positions non vides");
}

/// Le format sert de repli quand le chemin de stockage ne porte pas
/// l'extension attendue (upload nommé par UUID) : la copie temporaire suffixée
/// permet quand même au moteur de détecter le format.
#[test]
fn load_model_uses_format_when_path_has_no_extension() {
    let dir = tempfile::tempdir().unwrap();
    let extless = dir.path().join("model-uuid-without-ext");
    std::fs::copy(common::fixture("cube20.stl"), &extless).unwrap();

    let mesh = run_load_model(&extless, "stl");
    assert_eq!(
        mesh.indices.len(),
        36,
        "12 triangles malgré l'absence d'extension"
    );
}

/// Un fichier corrompu fait sortir le worker en code ≠ 0 avec une ligne `E`,
/// sans abattre le parent ni émettre de ligne `R`.
#[test]
fn load_model_corrupt_file_exits_nonzero_without_crashing_parent() {
    let dir = tempfile::tempdir().unwrap();
    let corrupt = dir.path().join("corrupt.stl");
    let mut f = std::fs::File::create(&corrupt).unwrap();
    f.write_all(b"pas du tout un STL binaire ou ASCII valide\n")
        .unwrap();

    let out = Command::new(worker_binary())
        .arg("load-model")
        .arg(&corrupt)
        .arg("stl")
        .output()
        .expect("lancement de engine-worker load-model");

    assert!(
        !out.status.success(),
        "code de sortie ≠ 0 sur fichier corrompu"
    );
    assert!(
        result_path(&out.stdout).is_none(),
        "aucune ligne R en erreur"
    );
    assert!(
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .any(|l| l.starts_with("E ")),
        "ligne d'erreur E émise"
    );
}

/// Un format inconnu est refusé proprement (code ≠ 0, pas de ligne `R`).
#[test]
fn load_model_rejects_unsupported_format() {
    let out = Command::new(worker_binary())
        .arg("load-model")
        .arg(common::fixture("cube20.stl"))
        .arg("gltf")
        .output()
        .expect("lancement de engine-worker load-model");

    assert!(!out.status.success(), "format non supporté → code ≠ 0");
    assert!(result_path(&out.stdout).is_none(), "aucune ligne R");
}

/// Lot 25 : plafond mémoire du worker (RLIMIT_DATA, `ENGINE_WORKER_MAX_MEMORY_MB`).
/// Une archive 3MF « zip bomb » (fixture zip_bomb.3mf, 4 Mo → 1,46 Go de XML,
/// 43 M sommets ; générée par fixtures/gen_zip_bomb.mjs) décompressée par
/// libslic3r fait échouer le worker (ligne `E`, code ≠ 0) au lieu d'épuiser la
/// RAM de l'hôte ; un modèle normal passe sous le même plafond.
#[cfg(target_os = "linux")]
#[test]
fn memory_cap_contains_a_3mf_zip_bomb() {
    const CAP_MB: u64 = 1024;
    let ok = Command::new(worker_binary())
        .env("ENGINE_WORKER_MAX_MEMORY_MB", CAP_MB.to_string())
        .arg("load-model")
        .arg(common::fixture("cube20.stl"))
        .arg("stl")
        .output()
        .expect("lancement de engine-worker");
    assert!(
        ok.status.success(),
        "un modèle normal passe sous le plafond"
    );
    if let Some(path) = result_path(&ok.stdout) {
        let _ = std::fs::remove_file(path);
    }

    let out_dir = tempfile::tempdir().unwrap();
    let started = std::time::Instant::now();
    let out = Command::new(worker_binary())
        .env("ENGINE_WORKER_MAX_MEMORY_MB", CAP_MB.to_string())
        .arg("read-project-3mf")
        .arg(common::fixture("zip_bomb.3mf"))
        .arg(out_dir.path().join("view.json"))
        .output()
        .expect("lancement de engine-worker");
    assert!(!out.status.success(), "la zip bomb doit être refusée");
    assert!(result_path(&out.stdout).is_none(), "aucune ligne R");
    assert!(started.elapsed() < std::time::Duration::from_secs(240));
    // Pic résident des enfants terminés (ru_maxrss, Kio) borné par le plafond.
    let mut usage = std::mem::MaybeUninit::<nix::libc::rusage>::zeroed();
    // SAFETY : getrusage remplit la structure fournie.
    unsafe { nix::libc::getrusage(nix::libc::RUSAGE_CHILDREN, usage.as_mut_ptr()) };
    let max_rss_mb = unsafe { usage.assume_init() }.ru_maxrss as u64 / 1024;
    eprintln!("pic résident des workers : {max_rss_mb} Mio");
    assert!(
        max_rss_mb < CAP_MB,
        "pic résident {max_rss_mb} Mio ≥ plafond {CAP_MB} Mio"
    );
}
