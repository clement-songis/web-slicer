//! Adaptateur de stockage SQLite (défaut). Implémente le trait `Storage` du
//! domaine ; aucune logique métier ici (constitution III). UUID/JSON/dates en
//! TEXT ; l'isolation repose sur `user_id` + FK ON DELETE CASCADE (PRAGMA
//! foreign_keys activé sur la connexion).

mod repos;

use std::str::FromStr;
use std::sync::Arc;

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::{
    GcodeRepo, InstanceRepo, JobRepo, ModelRepo, PresetRepo, PrinterRepo, ProjectRepo, Storage,
    StorageError, StorageResult, UserRepo,
};

use repos::{
    SqliteGcodeRepo, SqliteInstanceRepo, SqliteJobRepo, SqliteModelRepo, SqlitePresetRepo,
    SqlitePrinterRepo, SqliteProjectRepo, SqliteUserRepo,
};

/// Stockage SQLite : détient un repo par entité (chacun clone le pool).
pub struct SqliteStorage {
    users: SqliteUserRepo,
    projects: SqliteProjectRepo,
    models: Arc<SqliteModelRepo>,
    presets: SqlitePresetRepo,
    printers: SqlitePrinterRepo,
    jobs: SqliteJobRepo,
    gcodes: SqliteGcodeRepo,
    instance: SqliteInstanceRepo,
}

impl SqliteStorage {
    /// Ouvre (ou crée) la base à `url`, active les FK et applique les migrations.
    pub async fn connect(url: &str) -> StorageResult<Self> {
        let options = SqliteConnectOptions::from_str(url)
            .map_err(map_sqlx)?
            .create_if_missing(true)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .connect_with(options)
            .await
            .map_err(map_sqlx)?;
        sqlx::migrate!("migrations/sqlite")
            .run(&pool)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(Self::from_pool(pool))
    }

    /// Pool sous-jacent (pour le store de sessions `tower-sessions`, qui
    /// partage la même base). Spécifique SQLite, hors du trait `Storage`.
    pub fn pool(&self) -> SqlitePool {
        self.users.pool()
    }

    fn from_pool(pool: SqlitePool) -> Self {
        Self {
            users: SqliteUserRepo::new(pool.clone()),
            projects: SqliteProjectRepo::new(pool.clone()),
            models: Arc::new(SqliteModelRepo::new(pool.clone())),
            presets: SqlitePresetRepo::new(pool.clone()),
            printers: SqlitePrinterRepo::new(pool.clone()),
            jobs: SqliteJobRepo::new(pool.clone()),
            gcodes: SqliteGcodeRepo::new(pool.clone()),
            instance: SqliteInstanceRepo::new(pool),
        }
    }
}

impl Storage for SqliteStorage {
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
}

// --- Conversions & erreurs ---------------------------------------------------

/// Traduit une erreur sqlx en erreur du domaine (violation d'unicité → Conflict).
pub(crate) fn map_sqlx(e: sqlx::Error) -> StorageError {
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

pub(crate) fn id_str(id: impl Into<Uuid>) -> String {
    id.into().to_string()
}

pub(crate) fn parse_id<T: From<Uuid>>(s: &str) -> StorageResult<T> {
    Uuid::parse_str(s)
        .map(T::from)
        .map_err(|e| StorageError::Backend(format!("uuid invalide : {e}")))
}

pub(crate) fn json_text(value: &Value) -> String {
    value.to_string()
}

pub(crate) fn parse_json(s: &str) -> StorageResult<Value> {
    serde_json::from_str(s).map_err(|e| StorageError::Backend(format!("json invalide : {e}")))
}

pub(crate) fn now() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_default()
}

pub(crate) fn parse_time(s: &str) -> StorageResult<OffsetDateTime> {
    OffsetDateTime::parse(s, &Rfc3339)
        .map_err(|e| StorageError::Backend(format!("horodatage invalide : {e}")))
}
