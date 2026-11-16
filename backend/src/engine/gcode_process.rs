//! Simulation `GCodeProcessor` d'Orca d'un G-code stocké, via
//! `engine-worker gcode-process` (aucune FFI ici). Résultat mis en cache à
//! côté du G-code (`<fichier>.gcode.wspg`, écrit une fois, purgé avec l'espace
//! utilisateur) et en mémoire (LRU de 4 entrées) pour les requêtes de plages.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use engine::api::gcode_process::{ProcessedGcode, META_VERSION};
use tokio::sync::Mutex;

use super::scene::{result_json, SceneWorkerError};
use super::worker::{run_worker_at, worker_binary};

/// Un gros G-code (plusieurs millions de lignes) prend quelques secondes.
pub const PROCESS_TIMEOUT: Duration = Duration::from_secs(180);
const MEMORY_ENTRIES: usize = 4;

struct Cache {
    entries: Vec<(PathBuf, Arc<ProcessedGcode>)>,
    /// Un verrou par G-code : deux requêtes simultanées ne lancent qu'un worker.
    locks: HashMap<PathBuf, Arc<Mutex<()>>>,
}

fn cache() -> &'static Mutex<Cache> {
    static C: OnceLock<Mutex<Cache>> = OnceLock::new();
    C.get_or_init(|| {
        Mutex::new(Cache {
            entries: Vec::new(),
            locks: HashMap::new(),
        })
    })
}

pub fn cache_path(gcode: &Path) -> PathBuf {
    let mut name = gcode.as_os_str().to_owned();
    name.push(".wspg");
    PathBuf::from(name)
}

/// Résultat GCodeProcessor du G-code `gcode` (cache disque/mémoire).
pub async fn processed(gcode: &Path) -> Result<Arc<ProcessedGcode>, SceneWorkerError> {
    processed_at(&worker_binary(), gcode, PROCESS_TIMEOUT).await
}

pub async fn processed_at(
    binary: &Path,
    gcode: &Path,
    timeout: Duration,
) -> Result<Arc<ProcessedGcode>, SceneWorkerError> {
    let key = gcode.to_path_buf();
    let lock = {
        let mut c = cache().lock().await;
        if let Some((_, p)) = c.entries.iter().find(|(k, _)| *k == key) {
            return Ok(p.clone());
        }
        c.locks.entry(key.clone()).or_default().clone()
    };
    let _guard = lock.lock().await;
    {
        let c = cache().lock().await;
        if let Some((_, p)) = c.entries.iter().find(|(k, _)| *k == key) {
            return Ok(p.clone());
        }
    }
    let wspg = cache_path(gcode);
    let mut fresh = match (
        tokio::fs::metadata(&wspg).await,
        tokio::fs::metadata(gcode).await,
    ) {
        (Ok(c), Ok(g)) => match (c.modified(), g.modified()) {
            (Ok(c), Ok(g)) => c >= g,
            _ => false,
        },
        _ => false,
    };
    let mut parsed = None;
    for attempt in 0..2 {
        if !fresh {
            run_process(binary, gcode, &wspg, timeout).await?;
        }
        let bytes = tokio::fs::read(&wspg).await?;
        let p = tokio::task::spawn_blocking(move || ProcessedGcode::decode_file(&bytes))
            .await
            .map_err(|e| SceneWorkerError::Protocol(e.to_string()))?;
        match p {
            Ok(p) if p.meta["version"].as_u64() == Some(META_VERSION) => {
                parsed = Some(p);
                break;
            }
            // Cache d'un format antérieur (méta sans first_line / ranges) : retraité une fois.
            _ if attempt == 0 && fresh => fresh = false,
            Ok(_) => {
                return Err(SceneWorkerError::Protocol(
                    "version de méta inattendue".into(),
                ))
            }
            Err(e) => return Err(SceneWorkerError::Protocol(e)),
        }
    }
    let Some(parsed) = parsed else {
        return Err(SceneWorkerError::Protocol(
            "méta GCodeProcessor absent".into(),
        ));
    };
    let parsed = Arc::new(parsed);
    let mut c = cache().lock().await;
    c.entries.retain(|(k, _)| *k != key);
    c.entries.push((key.clone(), parsed.clone()));
    while c.entries.len() > MEMORY_ENTRIES {
        c.entries.remove(0);
    }
    c.locks.remove(&key);
    Ok(parsed)
}

/// `engine-worker gcode-process` → `<gcode>.wspg` (écriture atomique).
async fn run_process(
    binary: &Path,
    gcode: &Path,
    wspg: &Path,
    timeout: Duration,
) -> Result<(), SceneWorkerError> {
    let tmp = wspg.with_extension(format!("wspg.{}.tmp", uuid::Uuid::new_v4()));
    let stdout = run_worker_at(
        binary,
        &[
            "gcode-process",
            &gcode.to_string_lossy(),
            &tmp.to_string_lossy(),
        ],
        timeout,
    )
    .await;
    let stdout = match stdout {
        Ok(s) => s,
        Err(e) => {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err(super::scene::map_worker_error(e));
        }
    };
    result_json(&stdout)?;
    tokio::fs::rename(&tmp, wspg).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_path_is_sibling() {
        assert_eq!(
            cache_path(Path::new("/d/u/gcodes/x.gcode")),
            PathBuf::from("/d/u/gcodes/x.gcode.wspg")
        );
    }

    #[tokio::test]
    async fn stale_cache_version_is_reprocessed() {
        // Cache v1 (méta sans first_line/ranges) plus récent que le G-code : il
        // doit être retraité — ici le worker absent fait échouer le retraitement
        // au lieu de servir l'ancien format.
        let dir = tempfile::tempdir().unwrap();
        let g = dir.path().join("old.gcode");
        std::fs::write(&g, "G1 X1\n").unwrap();
        let mut p = ProcessedGcode::from_columns(vec![], vec![], vec![], "{}").unwrap();
        p.meta["version"] = serde_json::json!(1);
        std::fs::write(cache_path(&g), p.encode_file()).unwrap();
        let err = processed_at(
            Path::new("/nonexistent/engine-worker"),
            &g,
            Duration::from_secs(2),
        )
        .await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn missing_worker_is_an_error_and_leaves_no_cache() {
        let dir = tempfile::tempdir().unwrap();
        let g = dir.path().join("a.gcode");
        std::fs::write(&g, "G1 X1\n").unwrap();
        let err = processed_at(
            Path::new("/nonexistent/engine-worker"),
            &g,
            Duration::from_secs(2),
        )
        .await;
        assert!(err.is_err());
        assert!(!cache_path(&g).exists());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
