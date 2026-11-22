//! Adaptateur de stockage SQLite (défaut). Implémente le trait `Storage` du
//! domaine ; aucune logique métier ici (constitution III). UUID/JSON/dates en
//! TEXT ; l'isolation repose sur `user_id` + FK ON DELETE CASCADE (PRAGMA
//! foreign_keys activé sur la connexion).

mod pa_calib;
mod repos;
mod uploads;

use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{ConnectOptions, Connection, Sqlite, SqlitePool, Transaction};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::{
    GcodeRepo, InstanceRepo, JobRepo, ModelRepo, PaCalibrationRepo, PresetRepo, PrinterRepo,
    PrinterUploadRepo, ProjectRepo, Storage, StorageError, StorageResult, UserRepo,
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
    pa_calibrations: pa_calib::SqlitePaCalibrationRepo,
    printer_uploads: uploads::SqlitePrinterUploadRepo,
    /// Lot 46 : chemins de fichiers relatifs au répertoire de données.
    paths: super::paths::DataPaths,
}

impl SqliteStorage {
    /// Ouvre (ou crée) la base à `url`, active les FK et applique les migrations.
    pub async fn connect(url: &str) -> StorageResult<Self> {
        let options = SqliteConnectOptions::from_str(url)
            .map_err(map_sqlx)?
            .create_if_missing(true)
            .foreign_keys(true)
            // Les requêtes HTTP et le worker de file partagent cette base : le
            // busy timeout absorbe les courtes contentions au lieu de renvoyer 500.
            .busy_timeout(Duration::from_secs(30));
        enable_wal(&options).await?;
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

    /// Codec des chemins de fichiers (racine fixée par `AnyStorage::set_data_root`).
    pub fn data_paths(&self) -> &super::paths::DataPaths {
        &self.paths
    }

    /// Lot 46 (OLD-a) : réécrit en relatif (`users/<uid>/…`) les chemins
    /// absolus stockés, relocalisés depuis un ancien répertoire de données
    /// quand le fichier existe sous la racine courante. Idempotent ; renvoie
    /// le nombre de valeurs réécrites.
    pub async fn relativize_paths(&self) -> StorageResult<u64> {
        let pool = self.users.pool();
        let mut changed = 0u64;
        for (table, column) in super::paths::PATH_COLUMNS {
            // Noms de table / colonne : constantes de PATH_COLUMNS.
            let rows: Vec<(String, String)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
                "SELECT id, {column} FROM {table} WHERE {column} LIKE '/%'"
            )))
            .fetch_all(&pool)
            .await
            .map_err(map_sqlx)?;
            let updates: Vec<(String, String)> = rows
                .into_iter()
                .filter_map(|(id, old)| {
                    let new = self.paths.encode(&old);
                    (new != old).then_some((id, new))
                })
                .collect();
            if updates.is_empty() {
                continue;
            }
            let mut tx = begin_write(&pool).await.map_err(map_sqlx)?;
            for (id, new) in &updates {
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "UPDATE {table} SET {column} = ? WHERE id = ?"
                )))
                .bind(new)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx)?;
            }
            tx.commit().await.map_err(map_sqlx)?;
            changed += updates.len() as u64;
        }
        Ok(changed)
    }

    fn from_pool(pool: SqlitePool) -> Self {
        let paths = super::paths::DataPaths::default();
        Self {
            paths: paths.clone(),
            users: SqliteUserRepo::new(pool.clone()),
            projects: SqliteProjectRepo::with_paths(pool.clone(), paths.clone()),
            models: Arc::new(SqliteModelRepo::with_paths(pool.clone(), paths.clone())),
            presets: SqlitePresetRepo::new(pool.clone()),
            printers: SqlitePrinterRepo::new(pool.clone()),
            jobs: SqliteJobRepo::new(pool.clone()),
            gcodes: SqliteGcodeRepo::with_paths(pool.clone(), paths),
            pa_calibrations: pa_calib::SqlitePaCalibrationRepo::new(pool.clone()),
            printer_uploads: uploads::SqlitePrinterUploadRepo::new(pool.clone()),
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
    fn pa_calibrations(&self) -> &dyn PaCalibrationRepo {
        &self.pa_calibrations
    }
    fn printer_uploads(&self) -> &dyn PrinterUploadRepo {
        &self.printer_uploads
    }
}

// --- Concurrence --------------------------------------------------------------
//
// Cause des « database is locked » (500) observés dès 2 clients concurrents :
// 1. les transactions d'écriture démarraient en `BEGIN` (DEFERRED) puis lisaient
//    avant d'écrire. La promotion verrou SHARED → RESERVED au premier INSERT/
//    UPDATE échoue **immédiatement** en SQLITE_BUSY si une autre connexion
//    écrit déjà : SQLite n'appelle pas le busy handler dans ce cas (il y aurait
//    interblocage), donc `busy_timeout(30 s)` est sans effet. Correctif :
//    `BEGIN IMMEDIATE` ([`begin_write`]), qui prend le verrou d'écriture au
//    début — là où le busy timeout s'applique ;
// 2. la base restait en journal `delete` : un lecteur bloque le COMMIT d'un
//    écrivain et inversement. En WAL, lecteurs et écrivain ne se bloquent plus.
//
// Pourquoi `journal_mode(Wal)` sur les options du pool échouait : sqlx émet le
// PRAGMA à **chaque** ouverture de connexion, et le passage en WAL exige un
// verrou exclusif qui ne peut pas être attendu via `sqlite3_busy_timeout` (cf.
// sqlx#1930). Dès qu'une autre connexion est ouverte sur la base (seconde
// connexion du pool, autre process qui lit la base — une base créée/lue par un
// script Python garde volontiers une transaction implicite ouverte), le PRAGMA
// renvoie aussitôt « database is locked ». Le mode WAL étant **persistant**
// dans le fichier, on le pose une seule fois au démarrage sur une connexion
// dédiée, avec réessais, puis le pool n'émet plus jamais le PRAGMA.

/// Passe la base en WAL (persistant) via une connexion dédiée. Réessaie tant
/// qu'une autre connexion empêche le verrou exclusif ; à défaut, continue en
/// mode de journal courant (les écritures restent correctes grâce à
/// [`begin_write`]) en le signalant.
async fn enable_wal(options: &SqliteConnectOptions) -> StorageResult<()> {
    if options.get_filename() == std::path::Path::new(":memory:") {
        return Ok(()); // pas de WAL pour une base mémoire
    }
    let wal = options.clone().journal_mode(SqliteJournalMode::Wal);
    let mut last_error = None;
    for _ in 0..40 {
        match wal.connect().await {
            Ok(conn) => {
                let _ = conn.close().await;
                return Ok(());
            }
            Err(e) if is_busy(&e) => {
                last_error = Some(e);
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
            Err(e) => return Err(map_sqlx(e)),
        }
    }
    tracing::warn!(
        error = ?last_error,
        "passage en journal WAL impossible (base verrouillée par un autre process) : \
         mode de journal inchangé"
    );
    Ok(())
}

fn is_busy(e: &sqlx::Error) -> bool {
    match e {
        // SQLITE_BUSY (5) et ses codes étendus (BUSY_RECOVERY, BUSY_SNAPSHOT…).
        sqlx::Error::Database(db) => db
            .code()
            .and_then(|c| c.parse::<i32>().ok())
            .is_some_and(|c| c & 0xff == 5),
        _ => false,
    }
}

/// Transaction d'écriture : `BEGIN IMMEDIATE` (verrou d'écriture pris au début,
/// attente bornée par le busy timeout) au lieu d'un `BEGIN` différé dont la
/// promotion échoue sans attendre sous contention.
pub(crate) async fn begin_write(
    pool: &SqlitePool,
) -> Result<Transaction<'static, Sqlite>, sqlx::Error> {
    pool.begin_with("BEGIN IMMEDIATE").await
}

// --- Conversions & erreurs ---------------------------------------------------

/// Traduit une erreur sqlx en erreur du domaine (violation d'unicité → Conflict).
/// Message client d'une violation d'unicité (sans détail SQL).
pub(crate) const UNIQUE_CONFLICT_MESSAGE: &str = "un élément portant ce nom existe déjà";

pub(crate) fn map_sqlx(e: sqlx::Error) -> StorageError {
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
