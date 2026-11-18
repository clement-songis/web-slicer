//! Service de conversion de modèle (T123, retournement R7).
//!
//! À l'upload, le maillage d'affichage n'est plus produit côté client mais
//! **décodé par le moteur** (libslic3r, seule source de vérité) dans le process
//! isolé `engine-worker` : une tâche `tokio` bornée par sémaphore décode le
//! fichier en WSMh (`load-model`, T121 via `run_worker`, T122), l'écrit via le
//! `FileStore`, met à jour `models.mesh_path`, puis publie l'événement WS
//! `model.converted`. Un échec est persisté (`conversion_error`) et notifié
//! silencieusement (l'UI le lira au prochain `/mesh`, T125/T127).
//!
//! Le décodage passe par le port [`MeshDecoder`] — l'implémentation de
//! production lance le worker ; les tests injectent un stub.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::Semaphore;

use crate::adapters::files::{FileStore, FilesError};
use crate::domain::{Model, ModelFormat, ModelRepo, StorageError};
use crate::engine::{run_worker, WorkerError, DEFAULT_TIMEOUT};
use crate::http::ws::EventHub;

/// Décodeur d'un fichier modèle vers le maillage d'affichage WSMh.
#[async_trait]
pub trait MeshDecoder: Send + Sync {
    /// Décode `source` (format `format`) en octets WSMh, ou échoue.
    async fn decode(&self, source: &Path, format: ModelFormat) -> Result<Vec<u8>, ConvertError>;
}

/// Échec d'une conversion (décodage moteur, E/S ou persistance).
#[derive(Debug, thiserror::Error)]
pub enum ConvertError {
    #[error("décodage moteur : {0}")]
    Worker(#[from] WorkerError),
    #[error("le worker n'a pas produit de ligne résultat `R`")]
    NoResult,
    #[error("format {0:?} sans conversion moteur")]
    UnsupportedFormat(ModelFormat),
    #[error("WSMh invalide : {0}")]
    InvalidMesh(String),
    #[error("E/S : {0}")]
    Io(#[from] std::io::Error),
    #[error("fichiers : {0}")]
    Files(#[from] FilesError),
    #[error("stockage : {0}")]
    Storage(#[from] StorageError),
}

impl ConvertError {
    /// Cause précise de l'échec, destinée à l'utilisateur : message du worker
    /// (ligne `E <code> <message>` du protocole, ex. « STL tronqué ») plutôt que
    /// le texte technique complet (code de sortie, stderr), qui part dans les logs.
    pub fn cause(&self) -> String {
        match self {
            ConvertError::Worker(WorkerError::Failed { stdout, stderr, .. }) => {
                worker_error_line(&String::from_utf8_lossy(stdout))
                    .or_else(|| worker_error_line(stderr))
                    .unwrap_or_else(|| "le moteur n'a pas pu lire le fichier".into())
            }
            ConvertError::Worker(WorkerError::Timeout(d)) => {
                format!(
                    "lecture du fichier interrompue après {} s (délai dépassé)",
                    d.as_secs()
                )
            }
            ConvertError::Worker(WorkerError::Crashed { .. }) => {
                "le moteur s'est arrêté brutalement en lisant le fichier (fichier corrompu ?)"
                    .into()
            }
            other => other.to_string(),
        }
    }
}

/// Message de la première ligne `E <code> <message>` (protocole worker).
fn worker_error_line(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("E "))
        .map(|line| {
            line.split_once(' ')
                .map_or(line, |(_, m)| m)
                .trim()
                .to_string()
        })
        .filter(|m| !m.is_empty())
}

/// Cause lisible d'un `conversion_error` persisté : les échecs enregistrés
/// avant le lot 27 contiennent le texte technique complet du worker
/// (« worker sorti en code 1 — stderr : … — stdout : E invalid_model … ») ; on
/// en extrait le message du moteur. Sinon le texte persisté tel quel.
pub fn conversion_cause(stored: &str) -> String {
    let stdout = stored.rsplit_once("stdout : ").map_or(stored, |(_, s)| s);
    worker_error_line(stdout).unwrap_or_else(|| stored.trim().to_string())
}

/// Décodeur de production : lance `engine-worker load-model` et récupère le WSMh
/// via la ligne `R <chemin>` (le worker écrit le binaire dans un fichier, car
/// libslic3r pollue stdout — cf. T121).
pub struct WorkerMeshDecoder {
    timeout: Duration,
}

impl WorkerMeshDecoder {
    pub fn new() -> Self {
        Self {
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

impl Default for WorkerMeshDecoder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MeshDecoder for WorkerMeshDecoder {
    async fn decode(&self, source: &Path, format: ModelFormat) -> Result<Vec<u8>, ConvertError> {
        let fmt = format_arg(format).ok_or(ConvertError::UnsupportedFormat(format))?;
        let source = source.to_string_lossy().to_string();
        let stdout = run_worker(&["load-model", &source, fmt], self.timeout).await?;
        let path = parse_result_path(&stdout).ok_or(ConvertError::NoResult)?;
        let bytes = tokio::fs::read(&path).await?;
        // Le fichier temporaire du worker est à usage unique.
        let _ = tokio::fs::remove_file(&path).await;
        Ok(bytes)
    }
}

/// Extrait le chemin de la ligne `R <chemin>` du protocole worker (tolère le
/// bruit de log libslic3r qui peut précéder sur stdout).
fn parse_result_path(stdout: &[u8]) -> Option<String> {
    String::from_utf8_lossy(stdout)
        .lines()
        .find_map(|l| l.strip_prefix("R ").map(str::to_string))
}

/// Argument `<format>` du worker `load-model` (None = pas de conversion moteur).
pub(crate) fn format_arg(format: ModelFormat) -> Option<&'static str> {
    match format {
        ModelFormat::Stl => Some("stl"),
        ModelFormat::Obj => Some("obj"),
        ModelFormat::ThreeMf => Some("3mf"),
        ModelFormat::Step => Some("step"),
        ModelFormat::Amf => Some("amf"),
        // Draco : décodé par libslic3r (Model::read_from_file → load_drc), comme
        // les modèles de calibration d'Orca (resources/calib/*.drc).
        ModelFormat::Drc => Some("drc"),
        // SVG : libslic3r extrude les profils (Format/svg.cpp, load_svg) comme
        // l'import SVG d'Orca.
        ModelFormat::Svg => Some("svg"),
    }
}

/// Nombre de triangles d'un WSMh (index_count / 3, en-tête little-endian).
fn wsmh_triangle_count(bytes: &[u8]) -> Result<i64, ConvertError> {
    let mesh = engine::api::DisplayMesh::decode(bytes)
        .map_err(|e| ConvertError::InvalidMesh(e.to_string()))?;
    Ok((mesh.indices.len() / 3) as i64)
}

/// Service de conversion : orchestre décodage → stockage → persistance →
/// notification, avec une concurrence bornée (sémaphore). Clonable (Arc partout)
/// pour être capturé dans une tâche `tokio::spawn` par la route d'upload (T124).
#[derive(Clone)]
pub struct ModelConverter {
    decoder: Arc<dyn MeshDecoder>,
    files: FileStore,
    models: Arc<dyn ModelRepo>,
    events: Arc<EventHub>,
    limiter: Arc<Semaphore>,
}

impl ModelConverter {
    pub fn new(
        decoder: Arc<dyn MeshDecoder>,
        files: FileStore,
        models: Arc<dyn ModelRepo>,
        events: Arc<EventHub>,
        max_concurrent: usize,
    ) -> Self {
        Self {
            decoder,
            files,
            models,
            events,
            limiter: Arc::new(Semaphore::new(max_concurrent.max(1))),
        }
    }

    /// Convertit `model` (borné par le sémaphore). En cas d'échec, persiste
    /// l'erreur ; ne panique jamais (une conversion ratée ne doit pas propager).
    pub async fn convert(&self, model: Model) {
        // Le permis est relâché à la fin (drop) : borne la charge moteur.
        let _permit = match self.limiter.acquire().await {
            Ok(permit) => permit,
            // Sémaphore fermé : arrêt en cours, on abandonne silencieusement.
            Err(_) => return,
        };
        if let Err(error) = self.run(&model).await {
            tracing::warn!(model = %model.id, error = %error, "conversion du modèle en échec");
            let message = error.cause();
            // Persiste l'échec (best-effort : l'erreur de persistance est loguée
            // mais ne masque pas l'échec de conversion d'origine).
            let _ = self
                .models
                .mark_conversion_failed(model.user_id, model.id, &message)
                .await;
            self.events.publish_model_conversion_failed(
                model.user_id,
                &model.id.to_string(),
                &message,
            );
        }
    }

    /// Convertit une géométrie déjà extraite d'un projet 3MF. Le cycle de vie
    /// reste identique à un upload classique : en attente à la réponse HTTP,
    /// persistance du maillage puis événement `model.converted` en tâche de fond.
    pub async fn convert_mesh(&self, model: Model, mesh: engine::api::TriangleMesh) {
        let _permit = match self.limiter.acquire().await {
            Ok(permit) => permit,
            Err(_) => return,
        };
        if let Err(error) = self.store_mesh(&model, mesh.encode_display()).await {
            tracing::warn!(model = %model.id, error = %error, "conversion du modèle en échec");
            let message = error.cause();
            let _ = self
                .models
                .mark_conversion_failed(model.user_id, model.id, &message)
                .await;
            self.events.publish_model_conversion_failed(
                model.user_id,
                &model.id.to_string(),
                &message,
            );
        }
    }

    /// Chemin heureux : décode, stocke, met à jour, notifie.
    async fn run(&self, model: &Model) -> Result<(), ConvertError> {
        let source = Path::new(&model.file_path);
        let wsmh = self.decoder.decode(source, model.format).await?;
        self.store_mesh(model, wsmh).await
    }

    async fn store_mesh(&self, model: &Model, wsmh: Vec<u8>) -> Result<(), ConvertError> {
        let triangles = wsmh_triangle_count(&wsmh)?;
        let mesh_path = self
            .files
            .write_mesh(model.user_id, model.id, &wsmh)
            .await?;
        let mesh_path = mesh_path.to_string_lossy().to_string();
        self.models
            .set_mesh(model.user_id, model.id, &mesh_path, triangles)
            .await?;
        let mesh_url = format!("/api/models/{}/mesh", model.id);
        self.events
            .publish_model_converted(model.user_id, &model.id.to_string(), &mesh_url);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cause_is_the_worker_message_not_the_technical_dump() {
        let error = ConvertError::Worker(WorkerError::Failed {
            code: 1,
            stdout:
                b"[log] bruit\nE invalid_model Failed loading the input model: STL tronqu\xc3\xa9\n"
                    .to_vec(),
            stderr: "trace C++".into(),
        });
        assert_eq!(error.cause(), "Failed loading the input model: STL tronqué");
        assert!(
            ConvertError::Worker(WorkerError::Timeout(Duration::from_secs(60)))
                .cause()
                .contains("60 s")
        );
        // Ancien format persisté (avant lot 27) : cause extraite.
        let stored = "décodage moteur : worker sorti en code 1 — stderr :  — stdout : E invalid_model Fichier STEP illisible\n";
        assert_eq!(conversion_cause(stored), "Fichier STEP illisible");
        assert_eq!(
            conversion_cause("WSMh invalide : vide"),
            "WSMh invalide : vide"
        );
    }

    #[test]
    fn parses_result_path_ignoring_log_noise() {
        let stdout = b"[2026-07-12 14:00] [error] bruit libslic3r\nR /tmp/wsm-load-42.bin\n";
        assert_eq!(
            parse_result_path(stdout),
            Some("/tmp/wsm-load-42.bin".to_string())
        );
        assert_eq!(parse_result_path(b"pas de ligne R"), None);
    }

    #[test]
    fn format_arg_maps_supported_formats() {
        assert_eq!(format_arg(ModelFormat::Stl), Some("stl"));
        assert_eq!(format_arg(ModelFormat::ThreeMf), Some("3mf"));
        assert_eq!(format_arg(ModelFormat::Step), Some("step"));
        assert_eq!(format_arg(ModelFormat::Svg), Some("svg"));
        assert_eq!(format_arg(ModelFormat::Drc), Some("drc"));
    }

    #[test]
    fn triangle_count_reads_wsmh_header() {
        let mesh = engine::api::TriangleMesh {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            indices: vec![[0, 1, 2]],
        };
        let wsmh = mesh.encode_display();
        assert_eq!(wsmh_triangle_count(&wsmh).unwrap(), 1);
        assert!(wsmh_triangle_count(b"garbage").is_err());
    }
}
