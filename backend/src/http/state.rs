//! État partagé des handlers (composition root). Ne contient que des ports
//! abstraits : `Arc<dyn Storage>` et le magasin de fichiers.

use std::path::PathBuf;
use std::sync::Arc;

use crate::adapters::files::FileStore;
use crate::domain::Storage;

/// État applicatif injecté dans chaque handler (axum `State`).
#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn Storage>,
    pub files: FileStore,
    /// Répertoire des profils système OrcaSlicer (`resources/profiles`), source
    /// du seed/reseed des presets (T038).
    pub profiles_dir: PathBuf,
}

impl AppState {
    pub fn new(storage: Arc<dyn Storage>, files: FileStore) -> Self {
        Self {
            storage,
            files,
            profiles_dir: default_profiles_dir(),
        }
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
