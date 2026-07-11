//! Implémentations du trait `Storage` (contrat storage-trait.md). Sélection
//! runtime par `DATABASE_URL` (sqlite:// par défaut, postgres:// via feature).

pub mod sqlite;

#[cfg(feature = "postgres")]
pub mod postgres;
