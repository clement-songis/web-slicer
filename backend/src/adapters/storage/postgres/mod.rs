//! Adaptateur de stockage Postgres (T026, feature `postgres`). Miroir de
//! l'adaptateur SQLite ; aucune logique métier ici (constitution III). Types
//! natifs (`uuid`, `jsonb`, `timestamptz`, `boolean`, `bigint`), placeholders
//! `$n`, et réclamation de job concurrente via `FOR UPDATE SKIP LOCKED`.
//!
//! Le contrat `Storage` (T024) est **validé** contre un Postgres réel par
//! `tests/storage_contract_postgres.rs` (base éphémère isolée).

mod repos;

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::domain::{
    GcodeRepo, InstanceRepo, JobRepo, ModelRepo, PresetRepo, PrinterRepo, ProjectRepo, Storage,
    StorageError, StorageResult, UserRepo,
};

use repos::{
    PgGcodeRepo, PgInstanceRepo, PgJobRepo, PgModelRepo, PgPresetRepo, PgPrinterRepo,
    PgProjectRepo, PgUserRepo,
};

/// Stockage Postgres : un repo par entité, chacun partageant le pool.
pub struct PostgresStorage {
    users: PgUserRepo,
    projects: PgProjectRepo,
    models: PgModelRepo,
    presets: PgPresetRepo,
    printers: PgPrinterRepo,
    jobs: PgJobRepo,
    gcodes: PgGcodeRepo,
    instance: PgInstanceRepo,
}

impl PostgresStorage {
    /// Se connecte à `url` (postgres://…) et applique les migrations.
    pub async fn connect(url: &str) -> StorageResult<Self> {
        let pool = PgPoolOptions::new().connect(url).await.map_err(map_pg)?;
        sqlx::migrate!("migrations/postgres")
            .run(&pool)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(Self::from_pool(pool))
    }

    /// Pool sous-jacent (pour le store de sessions partageant la même base).
    pub fn pool(&self) -> PgPool {
        self.users.pool()
    }

    fn from_pool(pool: PgPool) -> Self {
        Self {
            users: PgUserRepo::new(pool.clone()),
            projects: PgProjectRepo::new(pool.clone()),
            models: PgModelRepo::new(pool.clone()),
            presets: PgPresetRepo::new(pool.clone()),
            printers: PgPrinterRepo::new(pool.clone()),
            jobs: PgJobRepo::new(pool.clone()),
            gcodes: PgGcodeRepo::new(pool.clone()),
            instance: PgInstanceRepo::new(pool),
        }
    }
}

impl Storage for PostgresStorage {
    fn users(&self) -> &dyn UserRepo {
        &self.users
    }
    fn projects(&self) -> &dyn ProjectRepo {
        &self.projects
    }
    fn models(&self) -> &dyn ModelRepo {
        &self.models
    }
    fn presets(&self) -> &dyn PresetRepo {
        &self.presets
    }
    fn printers(&self) -> &dyn PrinterRepo {
        &self.printers
    }
    fn jobs(&self) -> &dyn JobRepo {
        &self.jobs
    }
    fn gcodes(&self) -> &dyn GcodeRepo {
        &self.gcodes
    }
    fn instance(&self) -> &dyn InstanceRepo {
        &self.instance
    }
}

// --- Conversions & erreurs ---------------------------------------------------

/// Traduit une erreur sqlx en erreur du domaine (violation d'unicité → Conflict).
/// Générique sur le SGBD : `is_unique_violation()` couvre le SQLSTATE 23505 (PG).
pub(crate) fn map_pg(e: sqlx::Error) -> StorageError {
    if let sqlx::Error::Database(db) = &e {
        if db.is_unique_violation() {
            return StorageError::Conflict(db.message().to_string());
        }
    }
    StorageError::Backend(e.to_string())
}

/// Enum unité → texte stable (via serde `rename_all`).
pub(crate) fn enum_str<T: Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(Value::String(s)) => s,
        other => other.map(|v| v.to_string()).unwrap_or_default(),
    }
}

/// Texte → enum unité.
pub(crate) fn parse_enum<T: DeserializeOwned>(s: &str) -> StorageResult<T> {
    serde_json::from_value(Value::String(s.to_string()))
        .map_err(|e| StorageError::Backend(format!("enum invalide « {s} » : {e}")))
}
