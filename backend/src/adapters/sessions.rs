//! Store de sessions `tower-sessions` adossé à SQLite via sqlx 0.9.
//!
//! Le crate `tower-sessions-sqlx-store` est épinglé sur sqlx 0.8 (et un
//! `tower-sessions-core` 0.14), incompatible avec notre sqlx 0.9 / core 0.15.
//! On réimplémente donc le trait `SessionStore` directement sur notre pool.
//! L'enregistrement complet est sérialisé en JSON ; `expiry_date` est isolé
//! pour permettre la purge SQL des sessions expirées.

use async_trait::async_trait;
use sqlx::SqlitePool;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tower_sessions::session::{Id, Record};
use tower_sessions::session_store::{Error as SessionError, Result as SessionResult};
use tower_sessions::{ExpiredDeletion, SessionStore};

/// Store de sessions SQLite (partage le pool applicatif).
#[derive(Clone, Debug)]
pub struct SqliteSessionStore {
    pool: SqlitePool,
}

impl SqliteSessionStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    fn encode(record: &Record) -> SessionResult<(String, String, String)> {
        let id = record.id.to_string();
        let body =
            serde_json::to_string(record).map_err(|e| SessionError::Encode(e.to_string()))?;
        let expiry = record
            .expiry_date
            .format(&Rfc3339)
            .map_err(|e| SessionError::Encode(e.to_string()))?;
        Ok((id, body, expiry))
    }
}

#[async_trait]
impl SessionStore for SqliteSessionStore {
    async fn create(&self, record: &mut Record) -> SessionResult<()> {
        // Boucle anti-collision : réessaie avec un nouvel ID tant que l'INSERT
        // heurte la clé primaire.
        loop {
            let (id, body, expiry) = Self::encode(record)?;
            let res =
                sqlx::query("INSERT INTO sessions (id, record, expiry_date) VALUES (?1, ?2, ?3)")
                    .bind(&id)
                    .bind(&body)
                    .bind(&expiry)
                    .execute(&self.pool)
                    .await;
            match res {
                Ok(_) => return Ok(()),
                Err(sqlx::Error::Database(db)) if db.is_unique_violation() => {
                    record.id = Id::default();
                }
                Err(e) => return Err(SessionError::Backend(e.to_string())),
            }
        }
    }

    async fn save(&self, record: &Record) -> SessionResult<()> {
        let (id, body, expiry) = Self::encode(record)?;
        sqlx::query(
            "INSERT INTO sessions (id, record, expiry_date) VALUES (?1, ?2, ?3) \
             ON CONFLICT(id) DO UPDATE SET record = excluded.record, \
             expiry_date = excluded.expiry_date",
        )
        .bind(&id)
        .bind(&body)
        .bind(&expiry)
        .execute(&self.pool)
        .await
        .map_err(|e| SessionError::Backend(e.to_string()))?;
        Ok(())
    }

    async fn load(&self, session_id: &Id) -> SessionResult<Option<Record>> {
        let id = session_id.to_string();
        let now = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .map_err(|e| SessionError::Backend(e.to_string()))?;
        let row: Option<(String,)> =
            sqlx::query_as("SELECT record FROM sessions WHERE id = ?1 AND expiry_date > ?2")
                .bind(&id)
                .bind(&now)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| SessionError::Backend(e.to_string()))?;
        match row {
            Some((body,)) => {
                let record =
                    serde_json::from_str(&body).map_err(|e| SessionError::Decode(e.to_string()))?;
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    async fn delete(&self, session_id: &Id) -> SessionResult<()> {
        let id = session_id.to_string();
        sqlx::query("DELETE FROM sessions WHERE id = ?1")
            .bind(&id)
            .execute(&self.pool)
            .await
            .map_err(|e| SessionError::Backend(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl ExpiredDeletion for SqliteSessionStore {
    async fn delete_expired(&self) -> SessionResult<()> {
        let now = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .map_err(|e| SessionError::Backend(e.to_string()))?;
        sqlx::query("DELETE FROM sessions WHERE expiry_date <= ?1")
            .bind(&now)
            .execute(&self.pool)
            .await
            .map_err(|e| SessionError::Backend(e.to_string()))?;
        Ok(())
    }
}
