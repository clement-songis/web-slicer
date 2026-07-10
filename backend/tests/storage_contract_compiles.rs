//! Vérifie que la suite de contrat `Storage` (T024) compile contre le trait,
//! avant toute implémentation (TDD). Elle est **exécutée** sur un backend réel
//! par `storage_contract_sqlite.rs` (T025) et Postgres (T026).

mod common;

use backend::domain::Storage;

/// Vérifie la signature : `run_all` accepte bien un `&dyn Storage`. Jamais
/// appelée (aucune implémentation à ce stade) — sert de contrôle de compilation.
#[allow(dead_code)]
async fn typecheck(s: &dyn Storage) {
    common::storage_suite::run_all(s).await;
}

#[test]
fn suite_compiles() {}
