//! Historique PA (SQLite).

use async_trait::async_trait;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};

use crate::domain::*;

use super::{id_str, map_sqlx, now, parse_id, parse_time};

pub struct SqlitePaCalibrationRepo {
    pool: SqlitePool,
}

impl SqlitePaCalibrationRepo {
    pub(crate) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn row_to_pa(row: &SqliteRow) -> StorageResult<PaCalibration> {
    let printer_id = match row.get::<Option<String>, _>("printer_id") {
        Some(s) => Some(parse_id(&s)?),
        None => None,
    };
    Ok(PaCalibration {
        id: parse_id(&row.get::<String, _>("id"))?,
        user_id: parse_id(&row.get::<String, _>("user_id"))?,
        data: NewPaCalibration {
            printer_id,
            machine_preset: row.get("machine_preset"),
            filament_preset: row.get("filament_preset"),
            filament_id: row.get("filament_id"),
            nozzle_diameter: row.get("nozzle_diameter"),
            extruder_id: row.get("extruder_id"),
            nozzle_volume_type: row.get("nozzle_volume_type"),
            k_value: row.get("k_value"),
            n_coef: row.get("n_coef"),
            method: row.get("method"),
            name: row.get("name"),
            note: row.get("note"),
        },
        created_at: parse_time(&row.get::<String, _>("created_at"))?,
        updated_at: parse_time(&row.get::<String, _>("updated_at"))?,
    })
}

/// L'imprimante liée doit appartenir au compte.
async fn ensure_printer(
    pool: &SqlitePool,
    owner: UserId,
    printer: Option<PrinterId>,
) -> StorageResult<()> {
    let Some(printer) = printer else {
        return Ok(());
    };
    sqlx::query("SELECT 1 FROM printers WHERE id = ? AND user_id = ?")
        .bind(id_str(printer))
        .bind(id_str(owner))
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx)?
        .map(|_| ())
        .ok_or(StorageError::NotFound)
}

#[async_trait]
impl PaCalibrationRepo for SqlitePaCalibrationRepo {
    async fn create(&self, owner: UserId, d: NewPaCalibration) -> StorageResult<PaCalibration> {
        ensure_printer(&self.pool, owner, d.printer_id).await?;
        let id = PaCalibrationId::new();
        let at = now();
        sqlx::query(
            "INSERT INTO pa_calibrations (id, user_id, printer_id, machine_preset, filament_preset, filament_id, \
             nozzle_diameter, extruder_id, nozzle_volume_type, k_value, n_coef, method, name, note, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id_str(id))
        .bind(id_str(owner))
        .bind(d.printer_id.map(id_str))
        .bind(&d.machine_preset)
        .bind(&d.filament_preset)
        .bind(&d.filament_id)
        .bind(d.nozzle_diameter)
        .bind(d.extruder_id)
        .bind(&d.nozzle_volume_type)
        .bind(d.k_value)
        .bind(d.n_coef)
        .bind(&d.method)
        .bind(&d.name)
        .bind(&d.note)
        .bind(&at)
        .bind(&at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        self.get(owner, id).await
    }

    async fn get(&self, owner: UserId, id: PaCalibrationId) -> StorageResult<PaCalibration> {
        let row = sqlx::query("SELECT * FROM pa_calibrations WHERE id = ? AND user_id = ?")
            .bind(id_str(id))
            .bind(id_str(owner))
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx)?
            .ok_or(StorageError::NotFound)?;
        row_to_pa(&row)
    }

    async fn list(
        &self,
        owner: UserId,
        f: &PaCalibrationFilter,
    ) -> StorageResult<Vec<PaCalibration>> {
        let rows = sqlx::query(
            "SELECT * FROM pa_calibrations WHERE user_id = ? \
             AND (? IS NULL OR printer_id = ?) AND (? IS NULL OR machine_preset = ?) \
             AND (? IS NULL OR filament_id = ?) AND (? IS NULL OR filament_preset = ?) \
             AND (? IS NULL OR abs(nozzle_diameter - ?) < 1e-6) \
             ORDER BY created_at DESC, rowid DESC",
        )
        .bind(id_str(owner))
        .bind(f.printer_id.map(id_str))
        .bind(f.printer_id.map(id_str))
        .bind(&f.machine_preset)
        .bind(&f.machine_preset)
        .bind(&f.filament_id)
        .bind(&f.filament_id)
        .bind(&f.filament_preset)
        .bind(&f.filament_preset)
        .bind(f.nozzle_diameter)
        .bind(f.nozzle_diameter)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        rows.iter().map(row_to_pa).collect()
    }

    async fn update(
        &self,
        owner: UserId,
        id: PaCalibrationId,
        d: NewPaCalibration,
    ) -> StorageResult<PaCalibration> {
        ensure_printer(&self.pool, owner, d.printer_id).await?;
        let n = sqlx::query(
            "UPDATE pa_calibrations SET printer_id = ?, machine_preset = ?, filament_preset = ?, filament_id = ?, \
             nozzle_diameter = ?, extruder_id = ?, nozzle_volume_type = ?, k_value = ?, n_coef = ?, method = ?, \
             name = ?, note = ?, updated_at = ? WHERE id = ? AND user_id = ?",
        )
        .bind(d.printer_id.map(id_str))
        .bind(&d.machine_preset)
        .bind(&d.filament_preset)
        .bind(&d.filament_id)
        .bind(d.nozzle_diameter)
        .bind(d.extruder_id)
        .bind(&d.nozzle_volume_type)
        .bind(d.k_value)
        .bind(d.n_coef)
        .bind(&d.method)
        .bind(&d.name)
        .bind(&d.note)
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

    async fn delete(&self, owner: UserId, id: PaCalibrationId) -> StorageResult<()> {
        let n = sqlx::query("DELETE FROM pa_calibrations WHERE id = ? AND user_id = ?")
            .bind(id_str(id))
            .bind(id_str(owner))
            .execute(&self.pool)
            .await
            .map_err(map_sqlx)?
            .rows_affected();
        if n == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }
}
