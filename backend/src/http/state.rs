//! État partagé des handlers (composition root). Ne contient que des ports
//! abstraits : `Arc<dyn Storage>` et le magasin de fichiers.

use std::sync::Arc;

use crate::adapters::files::FileStore;
use crate::domain::Storage;

/// État applicatif injecté dans chaque handler (axum `State`).
#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn Storage>,
    pub files: FileStore,
}

impl AppState {
    pub fn new(storage: Arc<dyn Storage>, files: FileStore) -> Self {
        Self { storage, files }
    }
}
