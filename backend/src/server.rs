//! Composition root du serveur : branche l'adaptateur SQLite, le store de
//! sessions `tower-sessions` (même base) et le magasin de fichiers, puis
//! assemble le routeur. C'est le seul endroit qui connaît les implémentations
//! concrètes (constitution I).

use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use tower_sessions::SessionManagerLayer;

use crate::adapters::files::FileStore;
use crate::adapters::sessions::SqliteSessionStore;
use crate::adapters::storage::sqlite::SqliteStorage;
use crate::http::routes::router;
use crate::http::state::AppState;

/// Construit l'application axum prête à servir.
pub async fn build_app(database_url: &str, data_dir: PathBuf) -> anyhow::Result<Router> {
    let storage = SqliteStorage::connect(database_url)
        .await
        .map_err(|e| anyhow::anyhow!("stockage : {e}"))?;

    // Store de sessions sur la même base (table `sessions`, migrée avec le
    // reste du schéma).
    let session_store = SqliteSessionStore::new(storage.pool());
    let session_layer = SessionManagerLayer::new(session_store).with_secure(false);

    let files = FileStore::new(data_dir);
    let state = AppState::new(Arc::new(storage), files);
    Ok(router(state, session_layer))
}
