//! Historique des envois vers les hôtes d'impression (PostgreSQL, lot 42).

use async_trait::async_trait;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::*;

use super::{enum_str, map_pg, parse_enum};

pub struct PgPrinterUploadRepo {
    pool: PgPool,
}

impl PgPrinterUploadRepo {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn uid(id: impl Into<Uuid>) -> Uuid {
    id.into()
}

fn row_to_upload(row: &PgRow) -> StorageResult<PrinterUpload> {
    Ok(PrinterUpload {
        id: row.get::<Uuid, _>("id").into(),
        user_id: row.get::<Uuid, _>("user_id").into(),
        data: NewPrinterUpload {
            printer_id: row.get::<Uuid, _>("printer_id").into(),
            gcode_id: row.get::<Option<Uuid>, _>("gcode_id").map(Into::into),
            project_id: row.get::<Option<Uuid>, _>("project_id").map(Into::into),
            filename: row.get("filename"),
            host: row.get("host"),
            host_type: row.get("host_type"),
            size_bytes: row.get("size_bytes"),
            action: row.get("action"),
        },
        status: parse_enum(&row.get::<String, _>("status"))?,
        path: row.get("path"),
        print_started: row.get("print_started"),
        error: row.get("error"),
        created_at: row.get("created_at"),
        finished_at: row.get("finished_at"),
    })
}

impl PgPrinterUploadRepo {
    async fn get(&self, owner: UserId, id: PrinterUploadId) -> StorageResult<PrinterUpload> {
        let row = sqlx::query("SELECT * FROM printer_uploads WHERE id = $1 AND user_id = $2")
            .bind(uid(id))
            .bind(uid(owner))
            .fetch_optional(&self.pool)
            .await
            .map_err(map_pg)?
            .ok_or(StorageError::NotFound)?;
        row_to_upload(&row)
    }

    async fn owned(&self, sql: &'static str, id: Uuid, owner: UserId) -> StorageResult<bool> {
        Ok(sqlx::query(sql)
            .bind(id)
            .bind(uid(owner))
            .fetch_optional(&self.pool)
            .await
            .map_err(map_pg)?
            .is_some())
    }
}

#[async_trait]
impl PrinterUploadRepo for PgPrinterUploadRepo {
    async fn start(&self, owner: UserId, d: NewPrinterUpload) -> StorageResult<PrinterUpload> {
        if !self
            .owned(
                "SELECT 1 FROM printers WHERE id = $1 AND user_id = $2",
                uid(d.printer_id),
                owner,
            )
            .await?
        {
            return Err(StorageError::NotFound);
        }
        // G-code / projet d'un autre compte : jamais rattachés (SC-008).
        let gcode = match d.gcode_id {
            Some(g) => self
                .owned(
                    "SELECT 1 FROM gcodes WHERE id = $1 AND user_id = $2",
                    uid(g),
                    owner,
                )
                .await?
                .then_some(g),
            None => None,
        };
        let project = match d.project_id {
            Some(p) => self
                .owned(
                    "SELECT 1 FROM projects WHERE id = $1 AND user_id = $2",
                    uid(p),
                    owner,
                )
                .await?
                .then_some(p),
            None => None,
        };
        let id = PrinterUploadId::new();
        sqlx::query(
            "INSERT INTO printer_uploads (id, user_id, printer_id, gcode_id, project_id, filename, host, \
             host_type, size_bytes, action, status, print_started, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, false, $12)",
        )
        .bind(uid(id))
        .bind(uid(owner))
        .bind(uid(d.printer_id))
        .bind(gcode.map(uid))
        .bind(project.map(uid))
        .bind(&d.filename)
        .bind(&d.host)
        .bind(&d.host_type)
        .bind(d.size_bytes)
        .bind(&d.action)
        .bind(enum_str(&UploadStatus::Uploading))
        .bind(OffsetDateTime::now_utc())
        .execute(&self.pool)
        .await
        .map_err(map_pg)?;
        sqlx::query(
            "DELETE FROM printer_uploads WHERE user_id = $1 AND printer_id = $2 AND id NOT IN \
             (SELECT id FROM printer_uploads WHERE user_id = $1 AND printer_id = $2 \
              ORDER BY created_at DESC, id DESC LIMIT $3)",
        )
        .bind(uid(owner))
        .bind(uid(d.printer_id))
        .bind(PRINTER_UPLOADS_KEPT)
        .execute(&self.pool)
        .await
        .map_err(map_pg)?;
        self.get(owner, id).await
    }

    async fn finish(
        &self,
        owner: UserId,
        id: PrinterUploadId,
        outcome: UploadOutcome,
    ) -> StorageResult<PrinterUpload> {
        let (status, path, started, error) = match outcome {
            UploadOutcome::Completed {
                path,
                print_started,
            } => (UploadStatus::Completed, Some(path), print_started, None),
            UploadOutcome::Failed { error } => (UploadStatus::Error, None, false, Some(error)),
        };
        let n = sqlx::query(
            "UPDATE printer_uploads SET status = $1, path = $2, print_started = $3, error = $4, \
             finished_at = $5 WHERE id = $6 AND user_id = $7",
        )
        .bind(enum_str(&status))
        .bind(path)
        .bind(started)
        .bind(error)
        .bind(OffsetDateTime::now_utc())
        .bind(uid(id))
        .bind(uid(owner))
        .execute(&self.pool)
        .await
        .map_err(map_pg)?
        .rows_affected();
        if n == 0 {
            return Err(StorageError::NotFound);
        }
        self.get(owner, id).await
    }

    async fn list(
        &self,
        owner: UserId,
        printer: Option<PrinterId>,
        limit: i64,
    ) -> StorageResult<Vec<PrinterUpload>> {
        let rows = sqlx::query(
            "SELECT * FROM printer_uploads WHERE user_id = $1 \
             AND ($2::uuid IS NULL OR printer_id = $2) \
             ORDER BY created_at DESC, id DESC LIMIT $3",
        )
        .bind(uid(owner))
        .bind(printer.map(uid))
        .bind(limit.max(0))
        .fetch_all(&self.pool)
        .await
        .map_err(map_pg)?;
        rows.iter().map(row_to_upload).collect()
    }

    async fn clear(&self, owner: UserId, printer: PrinterId) -> StorageResult<u64> {
        if !self
            .owned(
                "SELECT 1 FROM printers WHERE id = $1 AND user_id = $2",
                uid(printer),
                owner,
            )
            .await?
        {
            return Err(StorageError::NotFound);
        }
        Ok(
            sqlx::query("DELETE FROM printer_uploads WHERE user_id = $1 AND printer_id = $2")
                .bind(uid(owner))
                .bind(uid(printer))
                .execute(&self.pool)
                .await
                .map_err(map_pg)?
                .rows_affected(),
        )
    }
}
