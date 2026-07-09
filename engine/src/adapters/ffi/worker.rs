//! Pilotage du process `engine-worker` (T018, R1/R9).
//!
//! Le tranchage libslic3r s'exécute dans un process séparé : un crash C++
//! reste **contenu** (le backend n'en meurt pas) et l'annulation se fait par
//! **kill** du worker. La progression remonte par un protocole de lignes sur
//! la sortie standard du worker (pipe) ; ce module est l'extrémité parent.
//!
//! Protocole (une ligne = un message, préfixe + espace) :
//! - `P <ratio> <phase>`  progression, `ratio` ∈ [0, 1] monotone
//! - `R <json>`           résultat de l'opération (une seule ligne)
//! - `E <code> <message>` erreur structurée (code = variante snake_case)
//!
//! T018 établit et teste ce protocole via le sous-commande `self-test` du
//! worker (progression déterministe, `--crash`, `--hang`). Le tranchage réel
//! (callbacks statusbar de `Slic3r::Print`) est branché en T019.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

use crate::api::{CancelToken, EngineError, EngineErrorCode, EngineResult, ProgressSink};

/// Préfixes de ligne du protocole (partagés avec le binaire worker).
pub mod proto {
    pub const PROGRESS: char = 'P';
    pub const RESULT: char = 'R';
    pub const ERROR: char = 'E';

    /// Nom de la variable d'environnement pointant le binaire worker
    /// (utile en test : `CARGO_BIN_EXE_engine-worker`).
    pub const BIN_ENV: &str = "ENGINE_WORKER_BIN";
}

/// Événement décodé d'une ligne du worker.
enum Event {
    Progress { ratio: f32, phase: String },
    Result(String),
    Error(EngineError),
}

/// Lance `engine-worker <args>` et pilote le protocole jusqu'au résultat.
///
/// Renvoie la charge JSON de la ligne `R` en cas de succès. La progression
/// est répercutée sur `progress` ; si `cancel` passe à vrai, le worker est
/// tué et l'erreur `Cancelled` est renvoyée (garantie d'annulation, R9).
pub fn drive(args: &[&str], progress: &ProgressSink, cancel: &CancelToken) -> EngineResult<String> {
    let bin = worker_binary()?;
    let mut child = Command::new(&bin)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            EngineError::new(
                EngineErrorCode::EngineCrashed,
                format!("lancement de engine-worker ({}) : {e}", bin.display()),
            )
        })?;

    // Fil lecteur : chaque ligne de stdout est transmise au parent.
    let stdout = child.stdout.take().expect("stdout demandé en pipe");
    let (tx, rx) = mpsc::channel::<String>();
    let reader = std::thread::spawn(move || {
        let mut buf = BufReader::new(stdout);
        let mut line = String::new();
        loop {
            line.clear();
            match buf.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    if tx.send(line.trim_end().to_string()).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let mut result: Option<String> = None;
    let mut error: Option<EngineError> = None;
    let mut cancelled = false;
    // Le statusbar de libslic3r n'est pas strictement monotone (chaque
    // sous-étape rapporte son propre pourcentage) ; on garantit une
    // progression non décroissante aux consommateurs (barre de progression UI).
    let mut max_ratio = 0.0_f32;

    loop {
        if cancel.is_cancelled() {
            let _ = child.kill();
            cancelled = true;
            break;
        }
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => match parse_line(&line) {
                Some(Event::Progress { ratio, phase }) => {
                    max_ratio = max_ratio.max(ratio);
                    progress(&phase, max_ratio);
                }
                Some(Event::Result(json)) => result = Some(json),
                Some(Event::Error(e)) => error = Some(e),
                None => {} // ligne hors protocole (log) : ignorée
            },
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    let _ = reader.join();
    let status = child.wait()?;

    if cancelled {
        return Err(EngineError::new(
            EngineErrorCode::Cancelled,
            "tranchage annulé (kill du worker)",
        ));
    }
    if let Some(e) = error {
        return Err(e);
    }
    if let Some(json) = result {
        return Ok(json);
    }
    // Ni résultat ni erreur explicite : le worker est mort en route (crash).
    Err(EngineError::new(
        EngineErrorCode::EngineCrashed,
        format!(
            "le worker s'est terminé sans résultat ({})",
            describe_status(&status)
        ),
    ))
}

fn parse_line(line: &str) -> Option<Event> {
    let (tag, rest) = line.split_once(' ').unwrap_or((line, ""));
    let mut tag_chars = tag.chars();
    let tag = tag_chars.next()?;
    if tag_chars.next().is_some() {
        return None; // préfixe multi-caractères : ligne hors protocole
    }
    match tag {
        proto::PROGRESS => {
            let (ratio, phase) = rest.split_once(' ').unwrap_or((rest, ""));
            Some(Event::Progress {
                ratio: ratio.parse().ok()?,
                phase: phase.to_string(),
            })
        }
        proto::RESULT => Some(Event::Result(rest.to_string())),
        proto::ERROR => {
            let (code, message) = rest.split_once(' ').unwrap_or((rest, ""));
            Some(Event::Error(EngineError::new(
                parse_code(code),
                message.to_string(),
            )))
        }
        _ => None,
    }
}

fn parse_code(code: &str) -> EngineErrorCode {
    match code {
        "invalid_model" => EngineErrorCode::InvalidModel,
        "invalid_config" => EngineErrorCode::InvalidConfig,
        "out_of_build_volume" => EngineErrorCode::OutOfBuildVolume,
        "cancelled" => EngineErrorCode::Cancelled,
        "unsupported" => EngineErrorCode::Unsupported,
        "io" => EngineErrorCode::Io,
        "internal" => EngineErrorCode::Internal,
        _ => EngineErrorCode::EngineCrashed,
    }
}

#[cfg(unix)]
fn describe_status(status: &std::process::ExitStatus) -> String {
    use std::os::unix::process::ExitStatusExt;
    if let Some(sig) = status.signal() {
        format!("tué par le signal {sig}")
    } else if let Some(code) = status.code() {
        format!("code de sortie {code}")
    } else {
        "état inconnu".to_string()
    }
}

#[cfg(not(unix))]
fn describe_status(status: &std::process::ExitStatus) -> String {
    match status.code() {
        Some(code) => format!("code de sortie {code}"),
        None => "état inconnu".to_string(),
    }
}

/// Localise le binaire `engine-worker` : variable d'env (test), puis voisin
/// de l'exécutable courant (target/debug), puis `PATH`.
fn worker_binary() -> EngineResult<PathBuf> {
    if let Ok(p) = std::env::var(proto::BIN_ENV) {
        return Ok(PathBuf::from(p));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for cand in [dir.join("engine-worker"), dir.join("../engine-worker")] {
                if cand.exists() {
                    return Ok(cand);
                }
            }
        }
    }
    which::which("engine-worker").map_err(|_| {
        EngineError::new(
            EngineErrorCode::Internal,
            "binaire engine-worker introuvable",
        )
    })
}
