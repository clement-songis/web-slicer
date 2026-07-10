//! Exécute la suite générique de contrat `Storage` (T024) sur l'implémentation
//! SQLite (T025). Base sur fichier temporaire (le pool partage ainsi la même
//! base entre connexions, contrairement à `:memory:`).

mod common;

use backend::adapters::storage::sqlite::SqliteStorage;

#[tokio::test]
async fn sqlite_satisfies_storage_contract() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("contract.db");
    let url = format!("sqlite://{}", db.display());

    let storage = SqliteStorage::connect(&url)
        .await
        .expect("connexion + migrations SQLite");

    common::storage_suite::run_all(&storage).await;
}
