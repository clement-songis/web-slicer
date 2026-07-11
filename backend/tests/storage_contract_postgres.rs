//! Exécute la suite générique de contrat `Storage` (T024) sur l'implémentation
//! Postgres (T026). Nécessite `--features postgres` et un serveur pointé par
//! `TEST_DATABASE_URL` (sinon le test est ignoré — pas d'échec en CI sans base).
//!
//! La suite suppose une base vierge (comme le fichier temporaire du test
//! SQLite) : on applique les migrations puis on `TRUNCATE … CASCADE` avant de
//! l'exécuter, de sorte que le test soit ré-exécutable sur la base dédiée.
#![cfg(feature = "postgres")]

mod common;

use backend::adapters::storage::postgres::PostgresStorage;

#[tokio::test]
async fn postgres_satisfies_storage_contract() {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("TEST_DATABASE_URL absent — test de contrat Postgres ignoré.");
        return;
    };

    let storage = PostgresStorage::connect(&url)
        .await
        .expect("connexion + migrations Postgres");

    // Table rase (chaîne `'static` → pas d'exigence de durée de vie). CASCADE
    // couvre les dépendances FK ; RESTART IDENTITY réinitialise les séquences.
    sqlx::query(
        "TRUNCATE users, invitations, projects, models, presets, printers, \
         slicing_jobs, gcodes, instance_settings, sessions RESTART IDENTITY CASCADE",
    )
    .execute(&storage.pool())
    .await
    .expect("nettoyage de la base de contrat");

    common::storage_suite::run_all(&storage).await;
}
