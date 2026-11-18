//! Infra de spawn du process `engine-worker` (T122).
//!
//! Le slicing et le décodage de modèles s'exécutent dans un **process isolé**
//! (`engine-worker`, crate `engine`) : un crash C++ de libslic3r est ainsi
//! contenu (constitution : isolation moteur) et l'annulation se fait par kill.
//! Ce module lance ce binaire, applique un **timeout**, capture stdout, et mappe
//! **crash (signal) / code de sortie ≠ 0 / timeout** en erreur typée.
//!
//! Infra **partagée** : le service de conversion de modèle (T123) l'utilise pour
//! `load-model`, et le futur runner de tranchage réel s'y branchera aussi.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;

/// Nom du binaire worker, déployé à côté du backend.
const WORKER_BIN_NAME: &str = "engine-worker";

/// Timeout par défaut d'une opération worker (décodage/tranchage).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);
/// Bornes mémoire des flux capturés (les gros artefacts passent par fichiers).
pub const MAX_WORKER_OUTPUT_BYTES: usize = 16 * 1024 * 1024;

/// Échec d'une exécution du worker.
#[derive(Debug, thiserror::Error)]
pub enum WorkerError {
    /// Le binaire n'a pas pu être lancé (introuvable, permissions…).
    #[error("lancement du worker « {} » impossible : {source}", .binary.display())]
    Spawn {
        binary: PathBuf,
        source: std::io::Error,
    },
    /// Dépassement du délai : le worker a été tué.
    #[error("worker interrompu après {0:?} (tué)")]
    Timeout(Duration),
    /// Terminaison par signal (crash C++ contenu : abort/segfault).
    #[error("worker terminé par signal (crash contenu) — stderr : {stderr}")]
    Crashed { stderr: String },
    /// Sortie en code ≠ 0 (erreur applicative : ligne `E` sur stdout/stderr).
    /// `stdout` est conservé : le protocole worker y porte le détail structuré
    /// `E <code> <message>`, notamment les refus de `Print::validate`.
    #[error("worker sorti en code {code} — stderr : {stderr} — stdout : {}", String::from_utf8_lossy(.stdout))]
    Failed {
        code: i32,
        stdout: Vec<u8>,
        stderr: String,
    },
    /// Erreur d'E/S pendant la lecture des flux.
    #[error("E/S worker : {0}")]
    Io(#[from] std::io::Error),
    /// Log/protocole excessif : la lecture est bornée et le process est tué au
    /// plus tard au timeout plutôt que de laisser grossir la mémoire backend.
    #[error("flux {stream} du worker au-delà de {limit} octets")]
    OutputTooLarge { stream: &'static str, limit: usize },
}

/// Résout le chemin du binaire worker : `ENGINE_WORKER_BIN` s'il est défini,
/// sinon à côté de l'exécutable courant (déploiement côte à côte).
pub fn worker_binary() -> PathBuf {
    resolve_binary(
        std::env::var("ENGINE_WORKER_BIN").ok().as_deref(),
        std::env::current_exe().ok().as_deref(),
    )
}

/// Logique pure de résolution (testable sans toucher à l'environnement réel).
fn resolve_binary(env_override: Option<&str>, current_exe: Option<&Path>) -> PathBuf {
    if let Some(path) = env_override.filter(|p| !p.is_empty()) {
        return PathBuf::from(path);
    }
    current_exe
        .and_then(Path::parent)
        .map(|dir| {
            // Binaire de test dans target/<profil>/deps : le worker est dans
            // le répertoire parent. En production, le voisin direct fait foi.
            let sibling = dir.join(WORKER_BIN_NAME);
            if sibling.exists() {
                return sibling;
            }
            let parent = dir.join("..").join(WORKER_BIN_NAME);
            if parent.exists() {
                parent
            } else {
                sibling
            }
        })
        .unwrap_or_else(|| PathBuf::from(WORKER_BIN_NAME))
}

/// Lance le worker résolu (`worker_binary`) avec `args`, borné par `timeout`.
///
/// Passe par un worker **préchauffé à usage unique** (voir [`warm`]) : même
/// isolation (un process neuf par opération), sans l'initialisation statique
/// de libslic3r (~0,1 s) sur le chemin de la requête. Actif par défaut quand le
/// binaire est le voisin déployé ; `ENGINE_WORKER_BIN` explicite (doubles de
/// test qui ne parlent pas le protocole stdin) → lancement à froid, sauf
/// `ENGINE_WORKER_WARM=1`. `ENGINE_WORKER_WARM=0` force le lancement à froid.
pub async fn run_worker(args: &[&str], timeout: Duration) -> Result<Vec<u8>, WorkerError> {
    let binary = worker_binary();
    let explicit_bin = std::env::var("ENGINE_WORKER_BIN").is_ok_and(|v| !v.is_empty());
    if !warm::enabled(
        std::env::var("ENGINE_WORKER_WARM").ok().as_deref(),
        explicit_bin,
    ) {
        return run_worker_at(&binary, args, timeout).await;
    }
    run_worker_warm_at(&binary, args, timeout).await
}

/// Comme [`run_worker_at`], via un worker préchauffé de `binary` (pris s'il
/// est prêt, sinon lancé ; un remplaçant est relancé dans tous les cas). Le
/// binaire doit comprendre `--args-from-stdin` (engine-worker ≥ lot 18).
pub async fn run_worker_warm_at(
    binary: &Path,
    args: &[&str],
    timeout: Duration,
) -> Result<Vec<u8>, WorkerError> {
    let mut child = warm::take_or_spawn(binary)?;
    // Sous-commande en une ligne JSON, puis stdin refermé. Échec d'écriture :
    // le worker en attente a disparu, rien n'a été exécuté → lancement à froid.
    let mut line = serde_json::to_vec(args).expect("arguments sérialisables");
    line.push(b'\n');
    let sent = {
        use std::io::Write;
        let mut stdin = child.0.stdin.take().expect("stdin demandé en pipe");
        stdin.write_all(&line)
    };
    if sent.is_err() {
        return run_worker_at(binary, args, timeout).await;
    }
    let stdout = tokio::process::ChildStdout::from_std(
        child.0.stdout.take().expect("stdout demandé en pipe"),
    )?;
    let stderr = tokio::process::ChildStderr::from_std(
        child.0.stderr.take().expect("stderr demandé en pipe"),
    )?;
    let (stdout_reader, stderr_reader) = drain(stdout, stderr);
    let waited = tokio::time::timeout(timeout, async {
        // Process std (indépendant du runtime qui l'a lancé) : attente par
        // sondage court, négligeable devant la durée d'une opération moteur.
        loop {
            if let Some(status) = child.0.try_wait()? {
                return Ok::<_, std::io::Error>(status);
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await;
    let status = match waited {
        Ok(result) => result?,
        Err(_elapsed) => {
            child.kill();
            let _ = stdout_reader.await;
            let _ = stderr_reader.await;
            return Err(WorkerError::Timeout(timeout));
        }
    };
    finish(status, stdout_reader, stderr_reader).await
}

/// Workers préchauffés : un process `engine-worker --args-from-stdin` lancé à
/// l'avance attend sa sous-commande sur stdin. Chaque prise relance aussitôt
/// un remplaçant, qui s'initialise pendant l'opération en cours. Process
/// `std` (pas `tokio`) : l'emplacement survit aux runtimes (tests).
mod warm {
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, Stdio};
    use std::sync::{Mutex, OnceLock};

    use super::WorkerError;

    /// Process tué et récolté à la destruction s'il n'a pas terminé
    /// (abandon du futur, timeout, excédent de l'emplacement).
    pub(super) struct Owned(pub(super) Child);

    impl Owned {
        pub(super) fn kill(&mut self) {
            if matches!(self.0.try_wait(), Ok(None)) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
    }

    impl Drop for Owned {
        fn drop(&mut self) {
            self.kill();
        }
    }

    struct Slot {
        binary: PathBuf,
        child: Owned,
    }

    fn slot() -> &'static Mutex<Option<Slot>> {
        static SLOT: OnceLock<Mutex<Option<Slot>>> = OnceLock::new();
        SLOT.get_or_init(|| Mutex::new(None))
    }

    /// `ENGINE_WORKER_WARM` : "0" froid, "1" préchauffé ; sinon préchauffé
    /// sauf binaire explicite (`ENGINE_WORKER_BIN`).
    pub(super) fn enabled(var: Option<&str>, explicit_bin: bool) -> bool {
        match var {
            Some("0") => false,
            Some("1") => true,
            _ => !explicit_bin,
        }
    }

    fn spawn(binary: &Path) -> Result<Owned, WorkerError> {
        Command::new(binary)
            .arg("--args-from-stdin")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map(Owned)
            .map_err(|source| WorkerError::Spawn {
                binary: binary.to_path_buf(),
                source,
            })
    }

    /// Prend le worker en attente (s'il est vivant et du même binaire), sinon en
    /// lance un ; dans les deux cas, prépare le suivant.
    pub(super) fn take_or_spawn(binary: &Path) -> Result<Owned, WorkerError> {
        let ready = slot().lock().expect("verrou worker préchauffé").take();
        let ready = ready.and_then(|mut s| {
            let alive = matches!(s.child.0.try_wait(), Ok(None));
            (alive && s.binary == binary).then_some(s.child)
        });
        let child = match ready {
            Some(child) => child,
            None => spawn(binary)?,
        };
        refill(binary);
        Ok(child)
    }

    fn refill(binary: &Path) {
        // Échec de lancement : pas de remplaçant, la prochaine prise lancera
        // elle-même et remontera l'erreur.
        if let Ok(child) = spawn(binary) {
            let mut guard = slot().lock().expect("verrou worker préchauffé");
            // Un appel concurrent a pu remplir l'emplacement : l'excédent est
            // tué à la destruction.
            if guard.is_none() {
                *guard = Some(Slot {
                    binary: binary.to_path_buf(),
                    child,
                });
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::enabled;

        #[test]
        fn warm_by_default_cold_for_explicit_binary_or_opt_out() {
            assert!(enabled(None, false));
            assert!(!enabled(None, true));
            assert!(enabled(Some("1"), true));
            assert!(!enabled(Some("0"), false));
            assert!(enabled(Some(""), false));
        }
    }
}

/// Comme [`run_worker`] mais sur un binaire explicite — évite toute dépendance à
/// l'environnement (injection de chemin en test). Renvoie **stdout brut** en cas
/// de succès (le protocole `P`/`R`/`E` est décodé par l'appelant).
pub async fn run_worker_at(
    binary: &Path,
    args: &[&str],
    timeout: Duration,
) -> Result<Vec<u8>, WorkerError> {
    let mut child = Command::new(binary)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Le worker est tué si le futur est abandonné (timeout, annulation).
        .kill_on_drop(true)
        .spawn()
        .map_err(|source| WorkerError::Spawn {
            binary: binary.to_path_buf(),
            source,
        })?;

    let stdout = child.stdout.take().expect("stdout demandé en pipe");
    let stderr = child.stderr.take().expect("stderr demandé en pipe");
    let (stdout_reader, stderr_reader) = drain(stdout, stderr);

    let status = match tokio::time::timeout(timeout, child.wait()).await {
        Ok(result) => result?,
        Err(_elapsed) => {
            let _ = child.kill().await;
            let _ = stdout_reader.await;
            let _ = stderr_reader.await;
            return Err(WorkerError::Timeout(timeout));
        }
    };
    finish(status, stdout_reader, stderr_reader).await
}

type Reader = tokio::task::JoinHandle<std::io::Result<Vec<u8>>>;

/// Lecture bornée des deux flux en tâches de fond.
fn drain(
    stdout: impl tokio::io::AsyncRead + Unpin + Send + 'static,
    stderr: impl tokio::io::AsyncRead + Unpin + Send + 'static,
) -> (Reader, Reader) {
    fn read_bounded(stream: impl tokio::io::AsyncRead + Unpin + Send + 'static) -> Reader {
        use tokio::io::AsyncReadExt;
        tokio::spawn(async move {
            let mut bytes = Vec::new();
            stream
                .take((MAX_WORKER_OUTPUT_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
                .await?;
            Ok(bytes)
        })
    }
    (read_bounded(stdout), read_bounded(stderr))
}

/// Décode l'issue d'un worker terminé.
async fn finish(
    status: std::process::ExitStatus,
    stdout_reader: Reader,
    stderr_reader: Reader,
) -> Result<Vec<u8>, WorkerError> {
    let join_io = |error: tokio::task::JoinError| {
        WorkerError::Io(std::io::Error::other(format!(
            "lecture des flux worker : {error}"
        )))
    };
    let stdout = stdout_reader.await.map_err(join_io)??;
    let stderr = stderr_reader.await.map_err(join_io)??;
    if stdout.len() > MAX_WORKER_OUTPUT_BYTES {
        return Err(WorkerError::OutputTooLarge {
            stream: "stdout",
            limit: MAX_WORKER_OUTPUT_BYTES,
        });
    }
    if stderr.len() > MAX_WORKER_OUTPUT_BYTES {
        return Err(WorkerError::OutputTooLarge {
            stream: "stderr",
            limit: MAX_WORKER_OUTPUT_BYTES,
        });
    }
    let output = std::process::Output {
        status,
        stdout,
        stderr,
    };

    if output.status.success() {
        return Ok(output.stdout);
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    match output.status.code() {
        Some(code) => Err(WorkerError::Failed {
            code,
            stdout: output.stdout,
            stderr,
        }),
        // Pas de code = terminaison par signal (crash C++ contenu).
        None => Err(WorkerError::Crashed { stderr }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_override_wins_over_sibling() {
        let exe = PathBuf::from("/opt/app/backend");
        let got = resolve_binary(Some("/custom/engine-worker"), Some(&exe));
        assert_eq!(got, PathBuf::from("/custom/engine-worker"));
    }

    #[test]
    fn empty_env_falls_back_to_sibling() {
        let exe = PathBuf::from("/opt/app/backend");
        let got = resolve_binary(Some(""), Some(&exe));
        assert_eq!(got, PathBuf::from("/opt/app/engine-worker"));
    }

    #[test]
    fn no_env_uses_sibling_of_current_exe() {
        let exe = PathBuf::from("/opt/app/backend");
        let got = resolve_binary(None, Some(&exe));
        assert_eq!(got, PathBuf::from("/opt/app/engine-worker"));
    }

    #[test]
    fn no_exe_falls_back_to_bare_name() {
        let got = resolve_binary(None, None);
        assert_eq!(got, PathBuf::from(WORKER_BIN_NAME));
    }
}
