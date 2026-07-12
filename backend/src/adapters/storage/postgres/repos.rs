//! Implémentations Postgres des repos du domaine. Requêtes runtime `sqlx`
//! (placeholders `$n`), types natifs (uuid/jsonb/timestamptz/bool/bigint).
//! Miroir fonctionnel de l'adaptateur SQLite ; seule la réclamation de job
//! diffère (`FOR UPDATE SKIP LOCKED` pour la concurrence).

use async_trait::async_trait;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::*;

use super::{enum_str, map_pg, parse_enum};

// --- Helpers -----------------------------------------------------------------

/// Newtype d'id → `Uuid` natif (pour bind).
fn uid(id: impl Into<Uuid>) -> Uuid {
    id.into()
}

fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

fn get_id<T: From<Uuid>>(row: &PgRow, col: &str) -> T {
    T::from(row.get::<Uuid, _>(col))
}

fn get_opt_id<T: From<Uuid>>(row: &PgRow, col: &str) -> Option<T> {
    row.get::<Option<Uuid>, _>(col).map(T::from)
}

// --- Mappers ligne → entité --------------------------------------------------

fn row_to_user(row: &PgRow) -> StorageResult<User> {
    Ok(User {
        id: get_id(row, "id"),
        email: row.get("email"),
        password_hash: row.get("password_hash"),
        role: parse_enum(&row.get::<String, _>("role"))?,
        status: parse_enum(&row.get::<String, _>("status"))?,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn row_to_project(row: &PgRow) -> StorageResult<Project> {
    Ok(Project {
        id: get_id(row, "id"),
        user_id: get_id(row, "user_id"),
        name: row.get("name"),
        thumbnail_path: row.get("thumbnail_path"),
        version: row.get("version"),
        scene: row.get("scene"),
        active_presets: row.get("active_presets"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn row_to_model(row: &PgRow) -> StorageResult<Model> {
    Ok(Model {
        id: get_id(row, "id"),
        user_id: get_id(row, "user_id"),
        project_id: get_opt_id(row, "project_id"),
        filename: row.get("filename"),
        format: parse_enum(&row.get::<String, _>("format"))?,
        file_path: row.get("file_path"),
        mesh_path: row.get("mesh_path"),
        size_bytes: row.get("size_bytes"),
        triangle_count: row.get("triangle_count"),
        repair_report: row.get("repair_report"),
        conversion_error: row.get("conversion_error"),
    })
}

fn row_to_preset(row: &PgRow) -> StorageResult<Preset> {
    Ok(Preset {
        id: get_id(row, "id"),
        kind: parse_enum(&row.get::<String, _>("kind"))?,
        name: row.get("name"),
        origin: parse_enum(&row.get::<String, _>("origin"))?,
        user_id: get_opt_id(row, "user_id"),
        vendor: row.get("vendor"),
        inherits: row.get("inherits"),
        instantiation: row.get("instantiation"),
        setting_id: row.get("setting_id"),
        filament_id: row.get("filament_id"),
        compatible_printers: row.get("compatible_printers"),
        values: row.get("values_json"),
    })
}

fn row_to_printer(row: &PgRow) -> StorageResult<Printer> {
    Ok(Printer {
        id: get_id(row, "id"),
        user_id: get_id(row, "user_id"),
        name: row.get("name"),
        moonraker_url: row.get("moonraker_url"),
        api_key: row.get("api_key"),
        machine_preset_id: get_id(row, "machine_preset_id"),
    })
}

fn row_to_job(row: &PgRow) -> StorageResult<SlicingJob> {
    Ok(SlicingJob {
        id: get_id(row, "id"),
        user_id: get_id(row, "user_id"),
        project_id: get_id(row, "project_id"),
        plate_index: row.get("plate_index"),
        status: parse_enum(&row.get::<String, _>("status"))?,
        progress: row.get("progress"),
        phase: row.get("phase"),
        resolved_settings: row.get("resolved_settings"),
        error: row.get("error"),
        gcode_id: get_opt_id(row, "gcode_id"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn row_to_gcode(row: &PgRow) -> StorageResult<Gcode> {
    Ok(Gcode {
        id: get_id(row, "id"),
        user_id: get_id(row, "user_id"),
        job_id: get_id(row, "job_id"),
        file_path: row.get("file_path"),
        preview_path: row.get("preview_path"),
        stats: row.get("stats"),
        thumbnails: row.get("thumbnails"),
    })
}

fn row_to_invitation(row: &PgRow) -> StorageResult<Invitation> {
    Ok(Invitation {
        id: get_id(row, "id"),
        token: row.get("token"),
        issued_by: get_id(row, "issued_by"),
        expires_at: row.get("expires_at"),
        used: row.get("used"),
        created_at: row.get("created_at"),
    })
}

// --- Users -------------------------------------------------------------------

pub(crate) struct PgUserRepo {
    pool: PgPool,
}
impl PgUserRepo {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    pub(crate) fn pool(&self) -> PgPool {
        self.pool.clone()
    }
}

#[async_trait]
impl UserRepo for PgUserRepo {
    async fn create(&self, user: NewUser) -> StorageResult<User> {
        let id = UserId::new();
        let ts = now();
        sqlx::query(
            "INSERT INTO users (id, email, password_hash, role, status, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, 'active', $5, $6)",
        )
        .bind(uid(id))
        .bind(user.email.to_lowercase())
        .bind(&user.password_hash)
        .bind(enum_str(&user.role))
        .bind(ts)
        .bind(ts)
        .execute(&self.pool)
        .await
        .map_err(map_pg)?;
        self.get(id).await
    }

    async fn get(&self, id: UserId) -> StorageResult<User> {
        let row = sqlx::query("SELECT * FROM users WHERE id = $1")
            .bind(uid(id))
            .fetch_optional(&self.pool)
            .await
            .map_err(map_pg)?
            .ok_or(StorageError::NotFound)?;
        row_to_user(&row)
    }

    async fn find_by_email(&self, email: &str) -> StorageResult<Option<User>> {
        let row = sqlx::query("SELECT * FROM users WHERE email = $1")
            .bind(email.to_lowercase())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_pg)?;
        row.as_ref().map(row_to_user).transpose()
    }

    async fn list(&self) -> StorageResult<Vec<User>> {
        let rows = sqlx::query("SELECT * FROM users ORDER BY created_at")
            .fetch_all(&self.pool)
            .await
            .map_err(map_pg)?;
        rows.iter().map(row_to_user).collect()
    }

    async fn count(&self) -> StorageResult<i64> {
        let row = sqlx::query("SELECT COUNT(*) AS n FROM users")
            .fetch_one(&self.pool)
            .await
            .map_err(map_pg)?;
        Ok(row.get("n"))
    }

    async fn set_password_hash(&self, id: UserId, password_hash: &str) -> StorageResult<()> {
        let n = sqlx::query("UPDATE users SET password_hash = $1, updated_at = $2 WHERE id = $3")
            .bind(password_hash)
            .bind(now())
            .bind(uid(id))
            .execute(&self.pool)
            .await
            .map_err(map_pg)?
            .rows_affected();
        if n == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    async fn set_status(&self, id: UserId, status: UserStatus) -> StorageResult<()> {
        let n = sqlx::query("UPDATE users SET status = $1, updated_at = $2 WHERE id = $3")
            .bind(enum_str(&status))
            .bind(now())
            .bind(uid(id))
            .execute(&self.pool)
            .await
            .map_err(map_pg)?
            .rows_affected();
        if n == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    async fn delete(&self, id: UserId) -> StorageResult<()> {
        let n = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(uid(id))
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

// --- Projects ----------------------------------------------------------------

pub(crate) struct PgProjectRepo {
    pool: PgPool,
}
impl PgProjectRepo {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProjectRepo for PgProjectRepo {
    async fn create(&self, owner: UserId, project: NewProject) -> StorageResult<Project> {
        let id = ProjectId::new();
        let ts = now();
        sqlx::query(
            "INSERT INTO projects \
             (id, user_id, name, thumbnail_path, version, scene, active_presets, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, 1, $5, $6, $7, $8)",
        )
        .bind(uid(id))
        .bind(uid(owner))
        .bind(&project.name)
        .bind(&project.thumbnail_path)
        .bind(project.scene.clone())
        .bind(project.active_presets.clone())
        .bind(ts)
        .bind(ts)
        .execute(&self.pool)
        .await
        .map_err(map_pg)?;
        self.get(owner, id).await
    }

    async fn get(&self, owner: UserId, id: ProjectId) -> StorageResult<Project> {
        let row = sqlx::query("SELECT * FROM projects WHERE id = $1 AND user_id = $2")
            .bind(uid(id))
            .bind(uid(owner))
            .fetch_optional(&self.pool)
            .await
            .map_err(map_pg)?
            .ok_or(StorageError::NotFound)?;
        row_to_project(&row)
    }

    async fn list(&self, owner: UserId) -> StorageResult<Vec<Project>> {
        let rows =
            sqlx::query("SELECT * FROM projects WHERE user_id = $1 ORDER BY updated_at DESC")
                .bind(uid(owner))
                .fetch_all(&self.pool)
                .await
                .map_err(map_pg)?;
        rows.iter().map(row_to_project).collect()
    }

    async fn update(
        &self,
        owner: UserId,
        id: ProjectId,
        expected_version: i64,
        scene: serde_json::Value,
        active_presets: serde_json::Value,
        thumbnail_path: Option<String>,
    ) -> StorageResult<Project> {
        let row = sqlx::query(
            "UPDATE projects SET scene = $1, active_presets = $2, thumbnail_path = $3, \
             version = version + 1, updated_at = $4 \
             WHERE id = $5 AND user_id = $6 AND version = $7 RETURNING *",
        )
        .bind(scene)
        .bind(active_presets)
        .bind(&thumbnail_path)
        .bind(now())
        .bind(uid(id))
        .bind(uid(owner))
        .bind(expected_version)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_pg)?;

        match row {
            Some(r) => row_to_project(&r),
            None => {
                // Distingue « introuvable » de « version périmée ».
                let current = self.get(owner, id).await?;
                Err(StorageError::VersionConflict {
                    expected: expected_version,
                    actual: current.version,
                })
            }
        }
    }

    async fn rename(&self, owner: UserId, id: ProjectId, name: &str) -> StorageResult<Project> {
        let row = sqlx::query(
            "UPDATE projects SET name = $1, updated_at = $2 \
             WHERE id = $3 AND user_id = $4 RETURNING *",
        )
        .bind(name)
        .bind(now())
        .bind(uid(id))
        .bind(uid(owner))
        .fetch_optional(&self.pool)
        .await
        .map_err(map_pg)?
        .ok_or(StorageError::NotFound)?;
        row_to_project(&row)
    }

    async fn delete(&self, owner: UserId, id: ProjectId) -> StorageResult<()> {
        let n = sqlx::query("DELETE FROM projects WHERE id = $1 AND user_id = $2")
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

// --- Models ------------------------------------------------------------------

pub(crate) struct PgModelRepo {
    pool: PgPool,
}
impl PgModelRepo {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ModelRepo for PgModelRepo {
    async fn create(&self, owner: UserId, model: NewModel) -> StorageResult<Model> {
        let id = ModelId::new();
        sqlx::query(
            "INSERT INTO models \
             (id, user_id, project_id, filename, format, file_path, mesh_path, size_bytes, triangle_count, repair_report) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(uid(id))
        .bind(uid(owner))
        .bind(model.project_id.map(uid))
        .bind(&model.filename)
        .bind(enum_str(&model.format))
        .bind(&model.file_path)
        .bind(&model.mesh_path)
        .bind(model.size_bytes)
        .bind(model.triangle_count)
        .bind(model.repair_report.clone())
        .execute(&self.pool)
        .await
        .map_err(map_pg)?;
        self.get(owner, id).await
    }

    async fn get(&self, owner: UserId, id: ModelId) -> StorageResult<Model> {
        let row = sqlx::query("SELECT * FROM models WHERE id = $1 AND user_id = $2")
            .bind(uid(id))
            .bind(uid(owner))
            .fetch_optional(&self.pool)
            .await
            .map_err(map_pg)?
            .ok_or(StorageError::NotFound)?;
        row_to_model(&row)
    }

    async fn list(&self, owner: UserId, project: Option<ProjectId>) -> StorageResult<Vec<Model>> {
        let rows = match project {
            Some(p) => {
                sqlx::query("SELECT * FROM models WHERE user_id = $1 AND project_id = $2")
                    .bind(uid(owner))
                    .bind(uid(p))
                    .fetch_all(&self.pool)
                    .await
            }
            None => {
                sqlx::query("SELECT * FROM models WHERE user_id = $1")
                    .bind(uid(owner))
                    .fetch_all(&self.pool)
                    .await
            }
        }
        .map_err(map_pg)?;
        rows.iter().map(row_to_model).collect()
    }

    async fn delete(&self, owner: UserId, id: ModelId) -> StorageResult<()> {
        let n = sqlx::query("DELETE FROM models WHERE id = $1 AND user_id = $2")
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

    async fn set_mesh(
        &self,
        owner: UserId,
        id: ModelId,
        mesh_path: &str,
        triangle_count: i64,
    ) -> StorageResult<()> {
        let n = sqlx::query(
            "UPDATE models SET mesh_path = $1, triangle_count = $2, conversion_error = NULL \
             WHERE id = $3 AND user_id = $4",
        )
        .bind(mesh_path)
        .bind(triangle_count)
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

    async fn mark_conversion_failed(
        &self,
        owner: UserId,
        id: ModelId,
        error: &str,
    ) -> StorageResult<()> {
        let n =
            sqlx::query("UPDATE models SET conversion_error = $1 WHERE id = $2 AND user_id = $3")
                .bind(error)
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

// --- Presets -----------------------------------------------------------------

pub(crate) struct PgPresetRepo {
    pool: PgPool,
}
impl PgPresetRepo {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn insert(&self, preset: &Preset) -> StorageResult<()> {
        sqlx::query(
            "INSERT INTO presets \
             (id, kind, name, origin, user_id, vendor, inherits, instantiation, setting_id, filament_id, compatible_printers, values_json) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(uid(preset.id))
        .bind(enum_str(&preset.kind))
        .bind(&preset.name)
        .bind(enum_str(&preset.origin))
        .bind(preset.user_id.map(uid))
        .bind(&preset.vendor)
        .bind(&preset.inherits)
        .bind(preset.instantiation)
        .bind(&preset.setting_id)
        .bind(&preset.filament_id)
        .bind(preset.compatible_printers.clone())
        .bind(preset.values.clone())
        .execute(&self.pool)
        .await
        .map_err(map_pg)?;
        Ok(())
    }
}

#[async_trait]
impl PresetRepo for PgPresetRepo {
    async fn reseed_system(&self, presets: Vec<Preset>) -> StorageResult<u64> {
        let mut tx = self.pool.begin().await.map_err(map_pg)?;
        sqlx::query("DELETE FROM presets WHERE user_id IS NULL")
            .execute(&mut *tx)
            .await
            .map_err(map_pg)?;
        let mut count = 0u64;
        for p in &presets {
            sqlx::query(
                "INSERT INTO presets \
                 (id, kind, name, origin, user_id, vendor, inherits, instantiation, setting_id, filament_id, compatible_printers, values_json) \
                 VALUES ($1, $2, $3, 'system', NULL, $4, $5, $6, $7, $8, $9, $10)",
            )
            .bind(uid(p.id))
            .bind(enum_str(&p.kind))
            .bind(&p.name)
            .bind(&p.vendor)
            .bind(&p.inherits)
            .bind(p.instantiation)
            .bind(&p.setting_id)
            .bind(&p.filament_id)
            .bind(p.compatible_printers.clone())
            .bind(p.values.clone())
            .execute(&mut *tx)
            .await
            .map_err(map_pg)?;
            count += 1;
        }
        tx.commit().await.map_err(map_pg)?;
        Ok(count)
    }

    async fn system_count(&self) -> StorageResult<i64> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM presets WHERE user_id IS NULL")
            .fetch_one(&self.pool)
            .await
            .map_err(map_pg)?;
        Ok(count)
    }

    async fn list_compatible(
        &self,
        kind: PresetKind,
        printer_name: Option<&str>,
        user: UserId,
    ) -> StorageResult<Vec<Preset>> {
        let rows = sqlx::query(
            "SELECT * FROM presets \
             WHERE kind = $1 AND instantiation = true AND (user_id IS NULL OR user_id = $2) \
             ORDER BY origin, name",
        )
        .bind(enum_str(&kind))
        .bind(uid(user))
        .fetch_all(&self.pool)
        .await
        .map_err(map_pg)?;
        let presets: StorageResult<Vec<Preset>> = rows.iter().map(row_to_preset).collect();
        let presets = presets?;
        // Filtre de compatibilité imprimante (FR-021) : null = universel.
        Ok(presets
            .into_iter()
            .filter(|p| match (printer_name, &p.compatible_printers) {
                (Some(name), Some(serde_json::Value::Array(list))) => {
                    list.iter().any(|v| v.as_str() == Some(name))
                }
                _ => true,
            })
            .collect())
    }

    async fn get(&self, id: PresetId) -> StorageResult<Preset> {
        let row = sqlx::query("SELECT * FROM presets WHERE id = $1")
            .bind(uid(id))
            .fetch_optional(&self.pool)
            .await
            .map_err(map_pg)?
            .ok_or(StorageError::NotFound)?;
        row_to_preset(&row)
    }

    async fn create_user_preset(&self, owner: UserId, mut preset: Preset) -> StorageResult<Preset> {
        preset.origin = PresetOrigin::User;
        preset.user_id = Some(owner);
        self.insert(&preset).await?;
        Ok(preset)
    }

    async fn list_by_kind(&self, kind: PresetKind, user: UserId) -> StorageResult<Vec<Preset>> {
        let rows = sqlx::query(
            "SELECT * FROM presets \
             WHERE kind = $1 AND (user_id IS NULL OR user_id = $2) ORDER BY name",
        )
        .bind(enum_str(&kind))
        .bind(uid(user))
        .fetch_all(&self.pool)
        .await
        .map_err(map_pg)?;
        rows.iter().map(row_to_preset).collect()
    }

    async fn update_user_preset(
        &self,
        owner: UserId,
        id: PresetId,
        name: &str,
        values: serde_json::Value,
    ) -> StorageResult<Preset> {
        let row = sqlx::query(
            "UPDATE presets SET name = $1, values_json = $2 \
             WHERE id = $3 AND user_id = $4 RETURNING *",
        )
        .bind(name)
        .bind(values)
        .bind(uid(id))
        .bind(uid(owner))
        .fetch_optional(&self.pool)
        .await
        .map_err(map_pg)?
        .ok_or(StorageError::NotFound)?;
        row_to_preset(&row)
    }

    async fn delete_user_preset(&self, owner: UserId, id: PresetId) -> StorageResult<()> {
        let n = sqlx::query("DELETE FROM presets WHERE id = $1 AND user_id = $2")
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

// --- Printers ----------------------------------------------------------------

pub(crate) struct PgPrinterRepo {
    pool: PgPool,
}
impl PgPrinterRepo {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PrinterRepo for PgPrinterRepo {
    async fn create(&self, owner: UserId, printer: NewPrinter) -> StorageResult<Printer> {
        let id = PrinterId::new();
        sqlx::query(
            "INSERT INTO printers (id, user_id, name, moonraker_url, api_key, machine_preset_id) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(uid(id))
        .bind(uid(owner))
        .bind(&printer.name)
        .bind(&printer.moonraker_url)
        .bind(&printer.api_key)
        .bind(uid(printer.machine_preset_id))
        .execute(&self.pool)
        .await
        .map_err(map_pg)?;
        self.get(owner, id).await
    }

    async fn get(&self, owner: UserId, id: PrinterId) -> StorageResult<Printer> {
        let row = sqlx::query("SELECT * FROM printers WHERE id = $1 AND user_id = $2")
            .bind(uid(id))
            .bind(uid(owner))
            .fetch_optional(&self.pool)
            .await
            .map_err(map_pg)?
            .ok_or(StorageError::NotFound)?;
        row_to_printer(&row)
    }

    async fn list(&self, owner: UserId) -> StorageResult<Vec<Printer>> {
        let rows = sqlx::query("SELECT * FROM printers WHERE user_id = $1 ORDER BY name")
            .bind(uid(owner))
            .fetch_all(&self.pool)
            .await
            .map_err(map_pg)?;
        rows.iter().map(row_to_printer).collect()
    }

    async fn update(
        &self,
        owner: UserId,
        id: PrinterId,
        printer: NewPrinter,
    ) -> StorageResult<Printer> {
        let n = sqlx::query(
            "UPDATE printers SET name = $1, moonraker_url = $2, api_key = $3, machine_preset_id = $4 \
             WHERE id = $5 AND user_id = $6",
        )
        .bind(&printer.name)
        .bind(&printer.moonraker_url)
        .bind(&printer.api_key)
        .bind(uid(printer.machine_preset_id))
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

    async fn delete(&self, owner: UserId, id: PrinterId) -> StorageResult<()> {
        let n = sqlx::query("DELETE FROM printers WHERE id = $1 AND user_id = $2")
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

// --- Jobs --------------------------------------------------------------------

pub(crate) struct PgJobRepo {
    pool: PgPool,
}
impl PgJobRepo {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl JobRepo for PgJobRepo {
    async fn enqueue(&self, owner: UserId, job: NewJob) -> StorageResult<SlicingJob> {
        let id = JobId::new();
        let ts = now();
        sqlx::query(
            "INSERT INTO slicing_jobs \
             (id, user_id, project_id, plate_index, status, progress, phase, resolved_settings, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, 'queued', 0, '', $5, $6, $7)",
        )
        .bind(uid(id))
        .bind(uid(owner))
        .bind(uid(job.project_id))
        .bind(job.plate_index)
        .bind(job.resolved_settings.clone())
        .bind(ts)
        .bind(ts)
        .execute(&self.pool)
        .await
        .map_err(map_pg)?;
        self.get(owner, id).await
    }

    async fn get(&self, owner: UserId, id: JobId) -> StorageResult<SlicingJob> {
        let row = sqlx::query("SELECT * FROM slicing_jobs WHERE id = $1 AND user_id = $2")
            .bind(uid(id))
            .bind(uid(owner))
            .fetch_optional(&self.pool)
            .await
            .map_err(map_pg)?
            .ok_or(StorageError::NotFound)?;
        row_to_job(&row)
    }

    async fn list(&self, owner: UserId) -> StorageResult<Vec<SlicingJob>> {
        let rows =
            sqlx::query("SELECT * FROM slicing_jobs WHERE user_id = $1 ORDER BY created_at DESC")
                .bind(uid(owner))
                .fetch_all(&self.pool)
                .await
                .map_err(map_pg)?;
        rows.iter().map(row_to_job).collect()
    }

    async fn claim_next(&self) -> StorageResult<Option<SlicingJob>> {
        // `FOR UPDATE SKIP LOCKED` : chaque réclamant concurrent verrouille et
        // saute les lignes déjà prises → un job livré une seule fois (pas de
        // double délivrance), idiome Postgres pour une file de travaux.
        let row = sqlx::query(
            "UPDATE slicing_jobs SET status = 'running', updated_at = $1 \
             WHERE id = ( \
                 SELECT id FROM slicing_jobs WHERE status = 'queued' \
                 ORDER BY created_at FOR UPDATE SKIP LOCKED LIMIT 1 \
             ) \
             RETURNING *",
        )
        .bind(now())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_pg)?;
        row.as_ref().map(row_to_job).transpose()
    }

    async fn requeue_running(&self) -> StorageResult<u64> {
        let n = sqlx::query(
            "UPDATE slicing_jobs SET status = 'queued', updated_at = $1 WHERE status = 'running'",
        )
        .bind(now())
        .execute(&self.pool)
        .await
        .map_err(map_pg)?
        .rows_affected();
        Ok(n)
    }

    async fn update_progress(&self, id: JobId, progress: f64, phase: &str) -> StorageResult<()> {
        let n = sqlx::query(
            "UPDATE slicing_jobs SET progress = $1, phase = $2, updated_at = $3 WHERE id = $4",
        )
        .bind(progress)
        .bind(phase)
        .bind(now())
        .bind(uid(id))
        .execute(&self.pool)
        .await
        .map_err(map_pg)?
        .rows_affected();
        if n == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    async fn finish(&self, id: JobId, outcome: JobOutcome) -> StorageResult<()> {
        let (status, gcode_id, error) = match outcome {
            JobOutcome::Succeeded { gcode_id } => ("succeeded", Some(uid(gcode_id)), None),
            JobOutcome::Failed { error } => ("failed", None, Some(error)),
        };
        let n = sqlx::query(
            "UPDATE slicing_jobs SET status = $1, gcode_id = $2, error = $3, progress = 1, updated_at = $4 WHERE id = $5",
        )
        .bind(status)
        .bind(gcode_id)
        .bind(error)
        .bind(now())
        .bind(uid(id))
        .execute(&self.pool)
        .await
        .map_err(map_pg)?
        .rows_affected();
        if n == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    async fn cancel(&self, owner: UserId, id: JobId) -> StorageResult<()> {
        let n = sqlx::query(
            "UPDATE slicing_jobs SET status = 'cancelled', updated_at = $1 \
             WHERE id = $2 AND user_id = $3 AND status IN ('queued', 'running')",
        )
        .bind(now())
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

// --- Gcodes ------------------------------------------------------------------

pub(crate) struct PgGcodeRepo {
    pool: PgPool,
}
impl PgGcodeRepo {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GcodeRepo for PgGcodeRepo {
    async fn create(&self, owner: UserId, gcode: NewGcode) -> StorageResult<Gcode> {
        let id = GcodeId::new();
        sqlx::query(
            "INSERT INTO gcodes (id, user_id, job_id, file_path, preview_path, stats, thumbnails) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(uid(id))
        .bind(uid(owner))
        .bind(uid(gcode.job_id))
        .bind(&gcode.file_path)
        .bind(&gcode.preview_path)
        .bind(gcode.stats.clone())
        .bind(gcode.thumbnails.clone())
        .execute(&self.pool)
        .await
        .map_err(map_pg)?;
        self.get(owner, id).await
    }

    async fn get(&self, owner: UserId, id: GcodeId) -> StorageResult<Gcode> {
        let row = sqlx::query("SELECT * FROM gcodes WHERE id = $1 AND user_id = $2")
            .bind(uid(id))
            .bind(uid(owner))
            .fetch_optional(&self.pool)
            .await
            .map_err(map_pg)?
            .ok_or(StorageError::NotFound)?;
        row_to_gcode(&row)
    }

    async fn delete(&self, owner: UserId, id: GcodeId) -> StorageResult<()> {
        let n = sqlx::query("DELETE FROM gcodes WHERE id = $1 AND user_id = $2")
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

// --- Instance (réglages + invitations) --------------------------------------

pub(crate) struct PgInstanceRepo {
    pool: PgPool,
}
impl PgInstanceRepo {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Garantit la ligne singleton (id=1) avec les défauts.
    async fn ensure_row(&self) -> StorageResult<()> {
        sqlx::query(
            "INSERT INTO instance_settings (id, registration_policy, upload_limit_bytes) \
             VALUES (1, 'open', 524288000) ON CONFLICT (id) DO NOTHING",
        )
        .execute(&self.pool)
        .await
        .map_err(map_pg)?;
        Ok(())
    }
}

#[async_trait]
impl InstanceRepo for PgInstanceRepo {
    async fn settings(&self) -> StorageResult<InstanceSettings> {
        self.ensure_row().await?;
        let row = sqlx::query("SELECT * FROM instance_settings WHERE id = 1")
            .fetch_one(&self.pool)
            .await
            .map_err(map_pg)?;
        Ok(InstanceSettings {
            registration_policy: parse_enum(&row.get::<String, _>("registration_policy"))?,
            upload_limit_bytes: row.get("upload_limit_bytes"),
        })
    }

    async fn set_registration_policy(&self, policy: RegistrationPolicy) -> StorageResult<()> {
        self.ensure_row().await?;
        sqlx::query("UPDATE instance_settings SET registration_policy = $1 WHERE id = 1")
            .bind(enum_str(&policy))
            .execute(&self.pool)
            .await
            .map_err(map_pg)?;
        Ok(())
    }

    async fn set_upload_limit(&self, bytes: i64) -> StorageResult<()> {
        self.ensure_row().await?;
        sqlx::query("UPDATE instance_settings SET upload_limit_bytes = $1 WHERE id = 1")
            .bind(bytes)
            .execute(&self.pool)
            .await
            .map_err(map_pg)?;
        Ok(())
    }

    async fn create_invitation(&self, invitation: NewInvitation) -> StorageResult<Invitation> {
        let id = InvitationId::new();
        let ts = now();
        sqlx::query(
            "INSERT INTO invitations (id, token, issued_by, expires_at, used, created_at) \
             VALUES ($1, $2, $3, $4, false, $5)",
        )
        .bind(uid(id))
        .bind(&invitation.token)
        .bind(uid(invitation.issued_by))
        .bind(invitation.expires_at)
        .bind(ts)
        .execute(&self.pool)
        .await
        .map_err(map_pg)?;
        Ok(Invitation {
            id,
            token: invitation.token,
            issued_by: invitation.issued_by,
            expires_at: invitation.expires_at,
            used: false,
            created_at: ts,
        })
    }

    async fn consume_invitation(&self, token: &str) -> StorageResult<Invitation> {
        let row = sqlx::query(
            "UPDATE invitations SET used = true \
             WHERE token = $1 AND used = false AND expires_at > $2 RETURNING *",
        )
        .bind(token)
        .bind(now())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_pg)?
        .ok_or(StorageError::NotFound)?;
        row_to_invitation(&row)
    }

    async fn list_invitations(&self) -> StorageResult<Vec<Invitation>> {
        let rows = sqlx::query("SELECT * FROM invitations ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(map_pg)?;
        rows.iter().map(row_to_invitation).collect()
    }
}
