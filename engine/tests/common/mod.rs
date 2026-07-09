//! Outillage commun des tests d'intégration du moteur.

pub mod trait_suite;

use std::path::PathBuf;

/// Chemin d'une fixture du corpus (engine/tests/fixtures, T004).
pub fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}
