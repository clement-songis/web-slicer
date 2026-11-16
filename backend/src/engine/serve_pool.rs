//! Pool borné de workers **persistants** (`engine-worker serve`, JSON-lines) pour
//! les interactions continues (pinceau de peinture, pinceau de hauteur de
//! couche) : maillage + `TriangleSelector` restent en mémoire dans le worker,
//! un trait coûte un aller-retour sur un pipe au lieu d'un spawn (~200–400 ms).
//!
//! - un worker par clé (projet), au plus `max_workers` (LRU évincé = tué) ;
//! - inactif plus de `ttl` → tué au prochain accès au pool (reaper paresseux
//!   + tâche périodique) ; le worker borne lui-même ses sessions (LRU, triangles) ;
//! - requêtes sérialisées par worker (Mutex) ; délai dépassé ou EOF → worker
//!   retiré du pool, l'appelant reçoit une erreur (le client rouvrira).
//!   Aucune FFI dans ce process.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex, OnceLock};
use std::time::{Duration, Instant};

use engine::api::paint::{ServeOp, ServeRequest, ServeResponse};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

use super::scene::SceneWorkerError;

/// Délai d'une requête (un trait sur un gros maillage subdivise beaucoup).
pub const CALL_TIMEOUT: Duration = Duration::from_secs(30);
/// Ligne de réponse maximale (facettes d'un canal).
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;

struct Io {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    _child: Child,
}

/// Worker persistant (tué à la destruction : `kill_on_drop`).
pub struct ServeWorker {
    io: Mutex<Option<Io>>,
    next_id: AtomicU64,
    last_used: StdMutex<Instant>,
}

impl ServeWorker {
    async fn spawn(binary: &PathBuf) -> Result<Self, SceneWorkerError> {
        let mut child = Command::new(binary)
            .arg("serve")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| SceneWorkerError::Protocol(format!("lancement du worker : {e}")))?;
        let stdin = child.stdin.take().expect("stdin en pipe");
        let stdout = BufReader::new(child.stdout.take().expect("stdout en pipe"));
        Ok(Self {
            io: Mutex::new(Some(Io {
                stdin,
                stdout,
                _child: child,
            })),
            next_id: AtomicU64::new(1),
            last_used: StdMutex::new(Instant::now()),
        })
    }

    fn idle_for(&self) -> Duration {
        self.last_used.lock().expect("verrou").elapsed()
    }

    /// Une requête → sa réponse (lignes non `R ` ignorées : logs libslic3r).
    pub async fn call(
        &self,
        op: ServeOp,
        timeout: Duration,
    ) -> Result<ServeResponse, SceneWorkerError> {
        *self.last_used.lock().expect("verrou") = Instant::now();
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let mut line = serde_json::to_string(&ServeRequest { id, op })
            .map_err(|e| SceneWorkerError::Protocol(e.to_string()))?;
        line.push('\n');
        let mut guard = self.io.lock().await;
        let Some(io) = guard.as_mut() else {
            return Err(SceneWorkerError::Protocol("worker arrêté".into()));
        };
        let exchange = async {
            io.stdin.write_all(line.as_bytes()).await?;
            io.stdin.flush().await?;
            let mut buf = String::new();
            loop {
                buf.clear();
                if io.stdout.read_line(&mut buf).await? == 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "worker terminé",
                    ));
                }
                if buf.len() > MAX_RESPONSE_BYTES {
                    return Err(std::io::Error::other("réponse trop volumineuse"));
                }
                let Some(json) = buf.strip_prefix("R ") else {
                    continue;
                };
                let resp: ServeResponse =
                    serde_json::from_str(json.trim_end()).map_err(std::io::Error::other)?;
                if resp.id == id {
                    return Ok(resp);
                }
            }
        };
        let result = match tokio::time::timeout(timeout, exchange).await {
            Ok(Ok(resp)) => Ok(resp),
            Ok(Err(e)) => Err(SceneWorkerError::Protocol(format!(
                "worker persistant : {e}"
            ))),
            Err(_) => Err(SceneWorkerError::Protocol(
                "worker persistant : délai dépassé".into(),
            )),
        };
        if result.is_err() {
            // État du pipe inconnu : on abandonne ce worker (tué au drop).
            *guard = None;
        }
        *self.last_used.lock().expect("verrou") = Instant::now();
        result
    }

    async fn is_alive(&self) -> bool {
        self.io.lock().await.is_some()
    }
}

/// Pool borné, indexé par clé (projet).
pub struct ServePool {
    binary: PathBuf,
    max_workers: usize,
    ttl: Duration,
    workers: Mutex<HashMap<String, Arc<ServeWorker>>>,
}

impl ServePool {
    pub fn new(binary: PathBuf, max_workers: usize, ttl: Duration) -> Self {
        Self {
            binary,
            max_workers: max_workers.max(1),
            ttl,
            workers: Mutex::new(HashMap::new()),
        }
    }

    /// Pool global du backend : `ENGINE_SERVE_WORKERS` (défaut 2),
    /// `ENGINE_SERVE_TTL_SECS` (défaut 300).
    pub fn global() -> &'static ServePool {
        static POOL: OnceLock<ServePool> = OnceLock::new();
        POOL.get_or_init(|| {
            let env = |k: &str, d: u64| {
                std::env::var(k)
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(d)
            };
            let pool = ServePool::new(
                super::worker::worker_binary(),
                env("ENGINE_SERVE_WORKERS", 2) as usize,
                Duration::from_secs(env("ENGINE_SERVE_TTL_SECS", 300)),
            );
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async {
                    let mut tick = tokio::time::interval(Duration::from_secs(60));
                    loop {
                        tick.tick().await;
                        ServePool::global().reap().await;
                    }
                });
            }
            pool
        })
    }

    /// Tue les workers inactifs depuis plus que le TTL.
    pub async fn reap(&self) {
        let ttl = self.ttl;
        self.workers.lock().await.retain(|_, w| w.idle_for() < ttl);
    }

    pub async fn len(&self) -> usize {
        self.workers.lock().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.workers.lock().await.is_empty()
    }

    /// Worker de la clé (créé au besoin ; LRU évincé si le pool est plein).
    pub async fn worker(&self, key: &str) -> Result<Arc<ServeWorker>, SceneWorkerError> {
        let mut workers = self.workers.lock().await;
        let ttl = self.ttl;
        workers.retain(|_, w| w.idle_for() < ttl);
        if let Some(w) = workers.get(key) {
            if w.is_alive().await {
                return Ok(w.clone());
            }
            workers.remove(key);
        }
        while workers.len() >= self.max_workers {
            let oldest = workers
                .iter()
                .max_by_key(|(_, w)| w.idle_for())
                .map(|(k, _)| k.clone())
                .expect("non vide");
            workers.remove(&oldest);
        }
        let w = Arc::new(ServeWorker::spawn(&self.binary).await?);
        workers.insert(key.to_string(), w.clone());
        Ok(w)
    }

    /// Requête sur le worker de la clé ; une erreur de transport retire le worker.
    pub async fn call(&self, key: &str, op: ServeOp) -> Result<ServeResponse, SceneWorkerError> {
        let worker = self.worker(key).await?;
        let result = worker.call(op, CALL_TIMEOUT).await;
        if result.is_err() {
            self.workers.lock().await.remove(key);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn missing_binary_is_a_clean_error() {
        let pool = ServePool::new(
            "/nonexistent/engine-worker".into(),
            1,
            Duration::from_secs(5),
        );
        assert!(pool.call("p", ServeOp::Ping).await.is_err());
        assert_eq!(pool.len().await, 0);
    }

    /// Faux worker (script shell) : écho JSON-lines + bruit de log, LRU et TTL.
    #[tokio::test]
    async fn pool_round_trips_and_bounds_workers() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("fake-worker");
        std::fs::write(
            &script,
            "#!/bin/sh\nwhile read -r line; do\n  id=$(echo \"$line\" | sed 's/.*\"id\":\\([0-9]*\\).*/\\1/')\n  echo \"[boost] log\"\n  echo \"R {\\\"id\\\":$id,\\\"ok\\\":true}\"\ndone\n",
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let pool = ServePool::new(script, 2, Duration::from_secs(60));
        for key in ["a", "b", "a", "c"] {
            let r = pool.call(key, ServeOp::Ping).await.expect("ping");
            assert!(r.ok);
        }
        assert_eq!(pool.len().await, 2, "pool borné");
        let w = pool.worker("c").await.unwrap();
        let r1 = w.call(ServeOp::Ping, CALL_TIMEOUT).await.unwrap();
        let r2 = w.call(ServeOp::Ping, CALL_TIMEOUT).await.unwrap();
        assert_eq!(r2.id, r1.id + 1);
        let short = ServePool::new(pool.binary.clone(), 2, Duration::from_millis(1));
        short.call("x", ServeOp::Ping).await.unwrap();
        tokio::time::sleep(Duration::from_millis(5)).await;
        short.reap().await;
        assert_eq!(short.len().await, 0, "TTL");
    }
}
