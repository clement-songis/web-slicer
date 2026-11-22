//! Adaptateur de stockage Postgres (T026, feature `postgres`). Miroir de
//! l'adaptateur SQLite ; aucune logique métier ici (constitution III). Types
//! natifs (`uuid`, `jsonb`, `timestamptz`, `boolean`, `bigint`), placeholders
//! `$n`, et réclamation de job concurrente via `FOR UPDATE SKIP LOCKED`.
//!
//! Le contrat `Storage` (T024) est **validé** contre un Postgres réel par
//! `tests/storage_contract_postgres.rs` (base éphémère isolée).

mod pa_calib;
mod repos;
mod uploads;

use std::sync::Arc;

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::domain::{
    GcodeRepo, InstanceRepo, JobRepo, ModelRepo, PaCalibrationRepo, PresetRepo, PrinterRepo,
    PrinterUploadRepo, ProjectRepo, Storage, StorageError, StorageResult, UserRepo,
};

use repos::{
    PgGcodeRepo, PgInstanceRepo, PgJobRepo, PgModelRepo, PgPresetRepo, PgPrinterRepo,
    PgProjectRepo, PgUserRepo,
};

/// Stockage Postgres : un repo par entité, chacun partageant le pool.
pub struct PostgresStorage {
    users: PgUserRepo,
    projects: PgProjectRepo,
    models: Arc<PgModelRepo>,
    presets: PgPresetRepo,
    printers: PgPrinterRepo,
    jobs: PgJobRepo,
    gcodes: PgGcodeRepo,
    instance: PgInstanceRepo,
    pa_calibrations: pa_calib::PgPaCalibrationRepo,
    printer_uploads: uploads::PgPrinterUploadRepo,
    /// Lot 46 : chemins de fichiers relatifs au répertoire de données.
    paths: super::paths::DataPaths,
    pool: PgPool,
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

    /// Crée une base vierge nommée `ws_test_<uuid>` sur le serveur de `base_url`
    /// puis s'y connecte (migrations appliquées). Réservé aux tests
    /// d'intégration : une base par test, comme un fichier SQLite temporaire.
    #[doc(hidden)]
    pub async fn connect_fresh(base_url: &str) -> StorageResult<Self> {
        use sqlx::postgres::PgConnectOptions;
        use sqlx::{ConnectOptions, Connection};
        use std::str::FromStr;

        let options = PgConnectOptions::from_str(base_url).map_err(map_pg)?;
        let name = format!("ws_test_{}", uuid::Uuid::new_v4().simple());
        let mut admin = options.connect().await.map_err(map_pg)?;
        // Nom généré ci-dessus (préfixe + uuid hexadécimal) : aucune entrée externe.
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!("CREATE DATABASE \"{name}\"")))
            .execute(&mut admin)
            .await
            .map_err(map_pg)?;
        admin.close().await.map_err(map_pg)?;
        // Pool réduit : des centaines de tests ouvrent chacun leur base.
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect_with(options.database(&name))
            .await
            .map_err(map_pg)?;
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

    /// Codec des chemins de fichiers (racine fixée par `AnyStorage::set_data_root`).
    pub fn data_paths(&self) -> &super::paths::DataPaths {
        &self.paths
    }

    /// Lot 46 (OLD-a) : voir `SqliteStorage::relativize_paths`.
    pub async fn relativize_paths(&self) -> StorageResult<u64> {
        let mut changed = 0u64;
        for (table, column) in super::paths::PATH_COLUMNS {
            // Noms de table / colonne : constantes de PATH_COLUMNS.
            let rows: Vec<(String, String)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
                "SELECT id::text, {column} FROM {table} WHERE {column} LIKE '/%'"
            )))
            .fetch_all(&self.pool)
            .await
            .map_err(|e| crate::domain::StorageError::Backend(e.to_string()))?;
            for (id, old) in rows {
                let new = self.paths.encode(&old);
                if new == old {
                    continue;
                }
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "UPDATE {table} SET {column} = $1 WHERE id::text = $2"
                )))
                .bind(&new)
                .bind(&id)
                .execute(&self.pool)
                .await
                .map_err(|e| crate::domain::StorageError::Backend(e.to_string()))?;
                changed += 1;
            }
        }
        Ok(changed)
    }

    fn from_pool(pool: PgPool) -> Self {
        let paths = super::paths::DataPaths::default();
        Self {
            paths: paths.clone(),
            pool: pool.clone(),
            users: PgUserRepo::new(pool.clone()),
            projects: PgProjectRepo::with_paths(pool.clone(), paths.clone()),
            models: Arc::new(PgModelRepo::with_paths(pool.clone(), paths.clone())),
            presets: PgPresetRepo::new(pool.clone()),
            printers: PgPrinterRepo::new(pool.clone()),
            jobs: PgJobRepo::new(pool.clone()),
            gcodes: PgGcodeRepo::with_paths(pool.clone(), paths),
            pa_calibrations: pa_calib::PgPaCalibrationRepo::new(pool.clone()),
            printer_uploads: uploads::PgPrinterUploadRepo::new(pool.clone()),
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
        self.models.as_ref()
    }
    fn models_shared(&self) -> Arc<dyn ModelRepo> {
        self.models.clone()
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
    fn pa_calibrations(&self) -> &dyn PaCalibrationRepo {
        &self.pa_calibrations
    }
    fn printer_uploads(&self) -> &dyn PrinterUploadRepo {
        &self.printer_uploads
    }
}

// --- Conversions & erreurs ---------------------------------------------------

/// Traduit une erreur sqlx en erreur du domaine (violation d'unicité → Conflict).
/// Générique sur le SGBD : `is_unique_violation()` couvre le SQLSTATE 23505 (PG).
/// Message client d'une violation d'unicité (sans détail SQL).
pub(crate) const UNIQUE_CONFLICT_MESSAGE: &str = "un élément portant ce nom existe déjà";

pub(crate) fn map_pg(e: sqlx::Error) -> StorageError {
    if let sqlx::Error::Database(db) = &e {
        if db.is_unique_violation() {
            // Le message brut du SGBD (« UNIQUE constraint failed: … ») part
            // dans les logs, jamais au client (lot 32).
            tracing::debug!(error = %db.message(), "violation d'unicité");
            return StorageError::Conflict(UNIQUE_CONFLICT_MESSAGE.to_string());
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
