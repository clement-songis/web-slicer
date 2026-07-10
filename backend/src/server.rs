//! Composition root du serveur : branche l'adaptateur SQLite, le store de
//! sessions `tower-sessions` (même base) et le magasin de fichiers, puis
//! assemble le routeur. C'est le seul endroit qui connaît les implémentations
//! concrètes (constitution I).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Router;
use tower_sessions::SessionManagerLayer;

use crate::adapters::files::FileStore;
use crate::adapters::sessions::SqliteSessionStore;
use crate::adapters::storage::sqlite::SqliteStorage;
use crate::domain::{presets, Storage};
use crate::http::routes::router;
use crate::http::state::{default_profiles_dir, AppState};

/// Construit l'application axum prête à servir.
pub async fn build_app(database_url: &str, data_dir: PathBuf) -> anyhow::Result<Router> {
    let storage = SqliteStorage::connect(database_url)
        .await
        .map_err(|e| anyhow::anyhow!("stockage : {e}"))?;

    let profiles_dir = default_profiles_dir();
    seed_presets_if_empty(&storage, &profiles_dir).await?;

    // Store de sessions sur la même base (table `sessions`, migrée avec le
    // reste du schéma).
    let session_store = SqliteSessionStore::new(storage.pool());
    let session_layer = SessionManagerLayer::new(session_store).with_secure(false);

    let files = FileStore::new(data_dir);
    let state = AppState::new(Arc::new(storage), files).with_profiles_dir(profiles_dir);
    Ok(router(state, session_layer))
}

/// Seed système au premier démarrage : n'importe les profils que si la base
/// n'en contient aucun (idempotent, l'admin peut re-seed via l'API ensuite).
async fn seed_presets_if_empty(storage: &SqliteStorage, profiles_dir: &Path) -> anyhow::Result<()> {
    if storage
        .presets()
        .system_count()
        .await
        .map_err(|e| anyhow::anyhow!("comptage presets : {e}"))?
        > 0
    {
        return Ok(());
    }
    match engine::presets::import_profiles(profiles_dir) {
        Ok(imported) => {
            let n = presets::reseed_system_presets(storage, &imported.presets)
                .await
                .map_err(|e| anyhow::anyhow!("seed presets : {e}"))?;
            tracing::info!(
                "{n} presets système importés depuis {}",
                profiles_dir.display()
            );
        }
        Err(e) => tracing::warn!(error = %e, "seed presets ignoré (profils indisponibles)"),
    }
    Ok(())
}
