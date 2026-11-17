//! Historique PA (PostgreSQL).

use async_trait::async_trait;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::*;

use super::map_pg;

pub struct PgPaCalibrationRepo {
    pool: PgPool,
}

impl PgPaCalibrationRepo {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn uid(id: impl Into<Uuid>) -> Uuid {
    id.into()
}

fn row_to_pa(row: &PgRow) -> PaCalibration {
    PaCalibration {
        id: row.get::<Uuid, _>("id").into(),
        user_id: row.get::<Uuid, _>("user_id").into(),
        data: NewPaCalibration {
            printer_id: row.get::<Option<Uuid>, _>("printer_id").map(Into::into),
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
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

async fn ensure_printer(
    pool: &PgPool,
    owner: UserId,
    printer: Option<PrinterId>,
) -> StorageResult<()> {
    let Some(printer) = printer else {
        return Ok(());
    };
    sqlx::query("SELECT 1 FROM printers WHERE id = $1 AND user_id = $2")
        .bind(uid(printer))
        .bind(uid(owner))
        .fetch_optional(pool)
        .await
        .map_err(map_pg)?
        .map(|_| ())
        .ok_or(StorageError::NotFound)
}

#[async_trait]
impl PaCalibrationRepo for PgPaCalibrationRepo {
    async fn create(&self, owner: UserId, d: NewPaCalibration) -> StorageResult<PaCalibration> {
        ensure_printer(&self.pool, owner, d.printer_id).await?;
        let id = PaCalibrationId::new();
        let at = OffsetDateTime::now_utc();
        sqlx::query(
            "INSERT INTO pa_calibrations (id, user_id, printer_id, machine_preset, filament_preset, filament_id, \
             nozzle_diameter, extruder_id, nozzle_volume_type, k_value, n_coef, method, name, note, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $15)",
        )
        .bind(uid(id))
        .bind(uid(owner))
        .bind(d.printer_id.map(uid))
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
        .bind(at)
        .execute(&self.pool)
        .await
        .map_err(map_pg)?;
        self.get(owner, id).await
    }

    async fn get(&self, owner: UserId, id: PaCalibrationId) -> StorageResult<PaCalibration> {
        sqlx::query("SELECT * FROM pa_calibrations WHERE id = $1 AND user_id = $2")
            .bind(uid(id))
            .bind(uid(owner))
            .fetch_optional(&self.pool)
            .await
            .map_err(map_pg)?
            .map(|r| row_to_pa(&r))
            .ok_or(StorageError::NotFound)
    }

    async fn list(
        &self,
        owner: UserId,
        f: &PaCalibrationFilter,
    ) -> StorageResult<Vec<PaCalibration>> {
        let rows = sqlx::query(
            "SELECT * FROM pa_calibrations WHERE user_id = $1 \
             AND ($2::uuid IS NULL OR printer_id = $2) AND ($3::text IS NULL OR machine_preset = $3) \
             AND ($4::text IS NULL OR filament_id = $4) AND ($5::text IS NULL OR filament_preset = $5) \
             AND ($6::double precision IS NULL OR abs(nozzle_diameter - $6) < 1e-6) \
             ORDER BY created_at DESC, id DESC",
        )
        .bind(uid(owner))
        .bind(f.printer_id.map(uid))
        .bind(&f.machine_preset)
        .bind(&f.filament_id)
        .bind(&f.filament_preset)
        .bind(f.nozzle_diameter)
        .fetch_all(&self.pool)
        .await
        .map_err(map_pg)?;
        Ok(rows.iter().map(row_to_pa).collect())
    }

    async fn update(
        &self,
        owner: UserId,
        id: PaCalibrationId,
        d: NewPaCalibration,
    ) -> StorageResult<PaCalibration> {
        ensure_printer(&self.pool, owner, d.printer_id).await?;
        let n = sqlx::query(
            "UPDATE pa_calibrations SET printer_id = $1, machine_preset = $2, filament_preset = $3, filament_id = $4, \
             nozzle_diameter = $5, extruder_id = $6, nozzle_volume_type = $7, k_value = $8, n_coef = $9, method = $10, \
             name = $11, note = $12, updated_at = $13 WHERE id = $14 AND user_id = $15",
        )
        .bind(d.printer_id.map(uid))
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

    async fn delete(&self, owner: UserId, id: PaCalibrationId) -> StorageResult<()> {
        let n = sqlx::query("DELETE FROM pa_calibrations WHERE id = $1 AND user_id = $2")
            .bind(uid(id))
            .bind(uid(owner))
            .execute(&self.pool)
            .await
            .map_err(map_pg)?
            .rows_affected();
        if n == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }
}
