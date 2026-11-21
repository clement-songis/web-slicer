//! Implémentations du trait `Storage` (contrat storage-trait.md). Sélection
//! runtime par `DATABASE_URL` (sqlite:// par défaut, postgres:// via feature).

pub mod sqlite;

#[cfg(feature = "postgres")]
pub mod postgres;

use std::collections::HashSet;
use std::sync::Arc;

use serde_json::Value;
use uuid::Uuid;

use crate::domain::{
    GcodeRepo, InstanceRepo, JobRepo, ModelRepo, PaCalibrationRepo, PresetRepo, PrinterRepo,
    ProjectRepo, Storage, StorageResult, UserRepo,
};

/// Stockage choisi à l'exécution selon le schéma de l'URL : `sqlite://…`
/// (défaut) ou `postgres://…` / `postgresql://…` (feature `postgres`).
pub enum AnyStorage {
    Sqlite(sqlite::SqliteStorage),
    #[cfg(feature = "postgres")]
    Postgres(postgres::PostgresStorage),
}

impl AnyStorage {
    /// Ouvre la base désignée par `url` et applique les migrations.
    pub async fn connect(url: &str) -> StorageResult<Self> {
        if is_postgres_url(url) {
            #[cfg(feature = "postgres")]
            return postgres::PostgresStorage::connect(url)
                .await
                .map(Self::Postgres);
            #[cfg(not(feature = "postgres"))]
            return Err(crate::domain::StorageError::Backend(
                "URL postgres:// : recompiler le backend avec la feature `postgres`".into(),
            ));
        }
        sqlite::SqliteStorage::connect(url).await.map(Self::Sqlite)
    }

    /// Outil des tests d'intégration : avec la feature `postgres` et
    /// `TEST_DATABASE_URL` défini, crée une base Postgres **vierge** dédiée à
    /// l'appelant (chaque test garde l'isolation d'un fichier SQLite
    /// temporaire) ; sinon ouvre `sqlite_url`.
    #[doc(hidden)]
    pub async fn connect_test(sqlite_url: &str) -> StorageResult<Self> {
        #[cfg(feature = "postgres")]
        if let Ok(base) = std::env::var("TEST_DATABASE_URL") {
            return postgres::PostgresStorage::connect_fresh(&base)
                .await
                .map(Self::Postgres);
        }
        sqlite::SqliteStorage::connect(sqlite_url)
            .await
            .map(Self::Sqlite)
    }

    fn inner(&self) -> &dyn Storage {
        match self {
            Self::Sqlite(s) => s,
            #[cfg(feature = "postgres")]
            Self::Postgres(s) => s,
        }
    }
}

/// Vrai pour une URL Postgres (`postgres://` ou `postgresql://`).
pub fn is_postgres_url(url: &str) -> bool {
    url.starts_with("postgres://") || url.starts_with("postgresql://")
}

impl Storage for AnyStorage {
    fn users(&self) -> &dyn UserRepo {
        self.inner().users()
    }
    fn projects(&self) -> &dyn ProjectRepo {
        self.inner().projects()
    }
    fn models(&self) -> &dyn ModelRepo {
        self.inner().models()
    }
    fn models_shared(&self) -> Arc<dyn ModelRepo> {
        self.inner().models_shared()
    }
    fn presets(&self) -> &dyn PresetRepo {
        self.inner().presets()
    }
    fn printers(&self) -> &dyn PrinterRepo {
        self.inner().printers()
    }
    fn jobs(&self) -> &dyn JobRepo {
        self.inner().jobs()
    }
    fn gcodes(&self) -> &dyn GcodeRepo {
        self.inner().gcodes()
    }
    fn instance(&self) -> &dyn InstanceRepo {
        self.inner().instance()
    }
    fn pa_calibrations(&self) -> &dyn PaCalibrationRepo {
        self.inner().pa_calibrations()
    }
}

/// Extrait les UUID d'un document `active_presets`. Les références peuvent être
/// directement sous `printer`/`process` ou dans le tableau `filaments` ; le
/// parcours récursif évite aussi de laisser une référence morte si le document
/// évolue sans que l'adaptateur de stockage soit modifié.
pub(crate) fn collect_uuid_strings(value: &Value, ids: &mut HashSet<Uuid>) {
    match value {
        Value::String(s) => {
            if let Ok(id) = Uuid::parse_str(s) {
                ids.insert(id);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_uuid_strings(value, ids);
            }
        }
        Value::Object(values) => {
            // `overrides` (lot 40) : valeurs de réglages, pas des références.
            for (key, value) in values {
                if key != "overrides" {
                    collect_uuid_strings(value, ids);
                }
            }
        }
        _ => {}
    }
}

pub(crate) fn json_references_id(value: &Value, id: Uuid) -> bool {
    let mut ids = HashSet::new();
    collect_uuid_strings(value, &mut ids);
    ids.contains(&id)
}
