//! État partagé des handlers (composition root). Ne contient que des ports
//! abstraits : `Arc<dyn Storage>` et le magasin de fichiers.

use std::path::PathBuf;
use std::sync::Arc;

use crate::adapters::files::FileStore;
use crate::auth::SecretBox;
use crate::domain::Storage;
use crate::http::printer_relay::PrinterRelays;
use crate::http::ws::EventHub;
use crate::models::{ModelConverter, WorkerMeshDecoder};

/// Nombre de conversions de modèles simultanées (charge moteur bornée, T124).
/// Surchargeable par `MODEL_CONVERSION_CONCURRENCY`.
fn conversion_concurrency() -> usize {
    std::env::var("MODEL_CONVERSION_CONCURRENCY")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n| n > 0)
        .unwrap_or(2)
}

/// État applicatif injecté dans chaque handler (axum `State`).
#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn Storage>,
    pub files: FileStore,
    /// Répertoire des profils système OrcaSlicer (`resources/profiles`), source
    /// du seed/reseed des presets (T038).
    pub profiles_dir: PathBuf,
    /// Bus d'événements WebSocket (progression des jobs, conversions), T065.
    pub events: Arc<EventHub>,
    /// Coffre de chiffrement des secrets d'instance (clés API imprimante), T075.
    pub secrets: SecretBox,
    /// Relais de suivi d'impression Moonraker → canal WS (T076).
    pub relays: Arc<PrinterRelays>,
    /// Service de conversion de modèle (décodage moteur → maillage, T123/T124).
    pub converter: ModelConverter,
}

impl AppState {
    pub fn new(storage: Arc<dyn Storage>, files: FileStore) -> Self {
        let events = Arc::new(EventHub::new());
        let converter = ModelConverter::new(
            Arc::new(WorkerMeshDecoder::new()),
            files.clone(),
            storage.models_shared(),
            events.clone(),
            conversion_concurrency(),
        );
        Self {
            storage,
            files,
            profiles_dir: default_profiles_dir(),
            events,
            secrets: SecretBox::from_env(),
            relays: Arc::new(PrinterRelays::new()),
            converter,
        }
    }

    /// Surcharge le coffre de secrets (tests avec clé déterministe).
    pub fn with_secrets(mut self, secrets: SecretBox) -> Self {
        self.secrets = secrets;
        self
    }

    /// Surcharge le répertoire des profils (tests, déploiement).
    pub fn with_profiles_dir(mut self, dir: PathBuf) -> Self {
        self.profiles_dir = dir;
        self
    }
}

/// Répertoire de profils par défaut : `PROFILES_DIR` sinon les profils vendus
/// avec OrcaSlicer dans le dépôt.
pub fn default_profiles_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("PROFILES_DIR") {
        return PathBuf::from(dir);
    }
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../vendor/OrcaSlicer/resources/profiles"
    ))
}
