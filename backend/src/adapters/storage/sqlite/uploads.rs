//! Historique des envois vers les hôtes d'impression (SQLite, lot 42).

use async_trait::async_trait;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};

use crate::domain::*;

use super::{enum_str, id_str, map_sqlx, now, parse_enum, parse_id, parse_time};

pub struct SqlitePrinterUploadRepo {
    pool: SqlitePool,
}

impl SqlitePrinterUploadRepo {
    pub(crate) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn opt_id<T: From<uuid::Uuid>>(row: &SqliteRow, col: &str) -> StorageResult<Option<T>> {
    row.get::<Option<String>, _>(col)
        .map(|s| parse_id(&s))
        .transpose()
}

fn row_to_upload(row: &SqliteRow) -> StorageResult<PrinterUpload> {
    Ok(PrinterUpload {
        id: parse_id(&row.get::<String, _>("id"))?,
        user_id: parse_id(&row.get::<String, _>("user_id"))?,
        data: NewPrinterUpload {
            printer_id: parse_id(&row.get::<String, _>("printer_id"))?,
            gcode_id: opt_id(row, "gcode_id")?,
            project_id: opt_id(row, "project_id")?,
            filename: row.get("filename"),
            host: row.get("host"),
            host_type: row.get("host_type"),
            size_bytes: row.get("size_bytes"),
            action: row.get("action"),
        },
        status: parse_enum(&row.get::<String, _>("status"))?,
        path: row.get("path"),
        print_started: row.get::<i64, _>("print_started") != 0,
        error: row.get("error"),
        created_at: parse_time(&row.get::<String, _>("created_at"))?,
        finished_at: row
            .get::<Option<String>, _>("finished_at")
            .map(|s| parse_time(&s))
            .transpose()?,
    })
}

impl SqlitePrinterUploadRepo {
    async fn get(&self, owner: UserId, id: PrinterUploadId) -> StorageResult<PrinterUpload> {
        let row = sqlx::query("SELECT * FROM printer_uploads WHERE id = ? AND user_id = ?")
            .bind(id_str(id))
            .bind(id_str(owner))
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx)?
            .ok_or(StorageError::NotFound)?;
        row_to_upload(&row)
    }

    async fn ensure_printer(&self, owner: UserId, printer: PrinterId) -> StorageResult<()> {
        sqlx::query("SELECT 1 FROM printers WHERE id = ? AND user_id = ?")
            .bind(id_str(printer))
            .bind(id_str(owner))
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx)?
            .map(|_| ())
            .ok_or(StorageError::NotFound)
    }
}

#[async_trait]
impl PrinterUploadRepo for SqlitePrinterUploadRepo {
    async fn start(&self, owner: UserId, d: NewPrinterUpload) -> StorageResult<PrinterUpload> {
        self.ensure_printer(owner, d.printer_id).await?;
        // G-code / projet d'un autre compte : jamais rattachés (SC-008).
        let gcode = match d.gcode_id {
            Some(g) => sqlx::query("SELECT 1 FROM gcodes WHERE id = ? AND user_id = ?")
                .bind(id_str(g))
                .bind(id_str(owner))
                .fetch_optional(&self.pool)
                .await
                .map_err(map_sqlx)?
                .map(|_| g),
            None => None,
        };
        let project = match d.project_id {
            Some(p) => sqlx::query("SELECT 1 FROM projects WHERE id = ? AND user_id = ?")
                .bind(id_str(p))
                .bind(id_str(owner))
                .fetch_optional(&self.pool)
                .await
                .map_err(map_sqlx)?
                .map(|_| p),
            None => None,
        };
        let id = PrinterUploadId::new();
        sqlx::query(
            "INSERT INTO printer_uploads (id, user_id, printer_id, gcode_id, project_id, filename, host, \
             host_type, size_bytes, action, status, print_started, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?)",
        )
        .bind(id_str(id))
        .bind(id_str(owner))
        .bind(id_str(d.printer_id))
        .bind(gcode.map(id_str))
        .bind(project.map(id_str))
        .bind(&d.filename)
        .bind(&d.host)
        .bind(&d.host_type)
        .bind(d.size_bytes)
        .bind(&d.action)
        .bind(enum_str(&UploadStatus::Uploading))
        .bind(now())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        sqlx::query(
            "DELETE FROM printer_uploads WHERE user_id = ? AND printer_id = ? AND id NOT IN \
             (SELECT id FROM printer_uploads WHERE user_id = ? AND printer_id = ? \
              ORDER BY created_at DESC, rowid DESC LIMIT ?)",
        )
        .bind(id_str(owner))
        .bind(id_str(d.printer_id))
        .bind(id_str(owner))
        .bind(id_str(d.printer_id))
        .bind(PRINTER_UPLOADS_KEPT)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
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
            "UPDATE printer_uploads SET status = ?, path = ?, print_started = ?, error = ?, \
             finished_at = ? WHERE id = ? AND user_id = ?",
        )
        .bind(enum_str(&status))
        .bind(path)
        .bind(started as i64)
        .bind(error)
        .bind(now())
        .bind(id_str(id))
        .bind(id_str(owner))
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?
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
            "SELECT * FROM printer_uploads WHERE user_id = ? AND (? IS NULL OR printer_id = ?) \
             ORDER BY created_at DESC, rowid DESC LIMIT ?",
        )
        .bind(id_str(owner))
        .bind(printer.map(id_str))
        .bind(printer.map(id_str))
        .bind(limit.max(0))
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        rows.iter().map(row_to_upload).collect()
    }

    async fn clear(&self, owner: UserId, printer: PrinterId) -> StorageResult<u64> {
        self.ensure_printer(owner, printer).await?;
        Ok(
            sqlx::query("DELETE FROM printer_uploads WHERE user_id = ? AND printer_id = ?")
                .bind(id_str(owner))
                .bind(id_str(printer))
                .execute(&self.pool)
                .await
                .map_err(map_sqlx)?
                .rows_affected(),
        )
    }
}
