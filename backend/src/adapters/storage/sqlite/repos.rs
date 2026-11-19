//! Implémentations SQLite des repos du domaine. Requêtes runtime `sqlx`
//! (pas de macro compile-time : aucune `DATABASE_URL` requise au build).

use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqliteConnection, SqlitePool};

use crate::domain::*;

use super::super::{collect_uuid_strings, json_references_id};
use super::{
    begin_write, enum_str, id_str, json_text, map_sqlx, now, parse_enum, parse_id, parse_json,
    parse_time,
};

// --- Mappers ligne → entité --------------------------------------------------

fn row_to_user(row: &SqliteRow) -> StorageResult<User> {
    Ok(User {
        id: parse_id(&row.get::<String, _>("id"))?,
        email: row.get("email"),
        password_hash: row.get("password_hash"),
        role: parse_enum(&row.get::<String, _>("role"))?,
        status: parse_enum(&row.get::<String, _>("status"))?,
        created_at: parse_time(&row.get::<String, _>("created_at"))?,
        updated_at: parse_time(&row.get::<String, _>("updated_at"))?,
    })
}

fn opt_id<T: From<uuid::Uuid>>(row: &SqliteRow, col: &str) -> StorageResult<Option<T>> {
    match row.get::<Option<String>, _>(col) {
        Some(s) => Ok(Some(parse_id(&s)?)),
        None => Ok(None),
    }
}

fn opt_json(row: &SqliteRow, col: &str) -> StorageResult<Option<serde_json::Value>> {
    match row.get::<Option<String>, _>(col) {
        Some(s) => Ok(Some(parse_json(&s)?)),
        None => Ok(None),
    }
}

fn row_to_project(row: &SqliteRow) -> StorageResult<Project> {
    Ok(Project {
        id: parse_id(&row.get::<String, _>("id"))?,
        user_id: parse_id(&row.get::<String, _>("user_id"))?,
        name: row.get("name"),
        thumbnail_path: row.get("thumbnail_path"),
        version: row.get("version"),
        scene: parse_json(&row.get::<String, _>("scene"))?,
        active_presets: parse_json(&row.get::<String, _>("active_presets"))?,
        created_at: parse_time(&row.get::<String, _>("created_at"))?,
        updated_at: parse_time(&row.get::<String, _>("updated_at"))?,
    })
}

fn row_to_model(row: &SqliteRow) -> StorageResult<Model> {
    Ok(Model {
        id: parse_id(&row.get::<String, _>("id"))?,
        user_id: parse_id(&row.get::<String, _>("user_id"))?,
        project_id: opt_id(row, "project_id")?,
        filename: row.get("filename"),
        format: parse_enum(&row.get::<String, _>("format"))?,
        file_path: row.get("file_path"),
        mesh_path: row.get("mesh_path"),
        size_bytes: row.get("size_bytes"),
        triangle_count: row.get("triangle_count"),
        repair_report: opt_json(row, "repair_report")?,
        conversion_error: row.get("conversion_error"),
    })
}

fn row_to_preset(row: &SqliteRow) -> StorageResult<Preset> {
    Ok(Preset {
        id: parse_id(&row.get::<String, _>("id"))?,
        kind: parse_enum(&row.get::<String, _>("kind"))?,
        name: row.get("name"),
        origin: parse_enum(&row.get::<String, _>("origin"))?,
        user_id: opt_id(row, "user_id")?,
        vendor: row.get("vendor"),
        inherits: row.get("inherits"),
        instantiation: row.get::<i64, _>("instantiation") != 0,
        setting_id: row.get("setting_id"),
        filament_id: row.get("filament_id"),
        compatible_printers: opt_json(row, "compatible_printers")?,
        values: parse_json(&row.get::<String, _>("values_json"))?,
    })
}

fn row_to_printer(row: &SqliteRow) -> StorageResult<Printer> {
    Ok(Printer {
        id: parse_id(&row.get::<String, _>("id"))?,
        user_id: parse_id(&row.get::<String, _>("user_id"))?,
        name: row.get("name"),
        moonraker_url: row.get("moonraker_url"),
        api_key: row.get("api_key"),
        machine_preset_id: parse_id(&row.get::<String, _>("machine_preset_id"))?,
        host_type: row.get("host_type"),
        host_options: serde_json::from_str(&row.get::<String, _>("host_options"))
            .unwrap_or_else(|_| serde_json::json!({})),
    })
}

fn row_to_job(row: &SqliteRow) -> StorageResult<SlicingJob> {
    Ok(SlicingJob {
        id: parse_id(&row.get::<String, _>("id"))?,
        user_id: parse_id(&row.get::<String, _>("user_id"))?,
        project_id: parse_id(&row.get::<String, _>("project_id"))?,
        plate_index: row.get("plate_index"),
        status: parse_enum(&row.get::<String, _>("status"))?,
        progress: row.get("progress"),
        phase: row.get("phase"),
        resolved_settings: parse_json(&row.get::<String, _>("resolved_settings"))?,
        scene: opt_json(row, "scene")?,
        error: opt_json(row, "error")?,
        gcode_id: opt_id(row, "gcode_id")?,
        created_at: parse_time(&row.get::<String, _>("created_at"))?,
        updated_at: parse_time(&row.get::<String, _>("updated_at"))?,
    })
}

fn row_to_gcode(row: &SqliteRow) -> StorageResult<Gcode> {
    Ok(Gcode {
        id: parse_id(&row.get::<String, _>("id"))?,
        user_id: parse_id(&row.get::<String, _>("user_id"))?,
        job_id: parse_id(&row.get::<String, _>("job_id"))?,
        file_path: row.get("file_path"),
        preview_path: row.get("preview_path"),
        stats: parse_json(&row.get::<String, _>("stats"))?,
        thumbnails: parse_json(&row.get::<String, _>("thumbnails"))?,
    })
}

fn row_to_invitation(row: &SqliteRow) -> StorageResult<Invitation> {
    Ok(Invitation {
        id: parse_id(&row.get::<String, _>("id"))?,
        token: row.get("token"),
        issued_by: parse_id(&row.get::<String, _>("issued_by"))?,
        expires_at: parse_time(&row.get::<String, _>("expires_at"))?,
        used: row.get::<i64, _>("used") != 0,
        created_at: parse_time(&row.get::<String, _>("created_at"))?,
    })
}

// --- Users -------------------------------------------------------------------

pub(crate) struct SqliteUserRepo {
    pool: SqlitePool,
}
impl SqliteUserRepo {
    pub(crate) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
    pub(crate) fn pool(&self) -> SqlitePool {
        self.pool.clone()
    }
}

#[async_trait]
impl UserRepo for SqliteUserRepo {
    async fn create(&self, user: NewUser) -> StorageResult<User> {
        let id = UserId::new();
        let ts = now();
        sqlx::query(
            "INSERT INTO users (id, email, password_hash, role, status, created_at, updated_at) \
             VALUES (?, ?, ?, ?, 'active', ?, ?)",
        )
        .bind(id_str(id))
        .bind(user.email.to_lowercase())
        .bind(&user.password_hash)
        .bind(enum_str(&user.role))
        .bind(&ts)
        .bind(&ts)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        self.get(id).await
    }

    async fn get(&self, id: UserId) -> StorageResult<User> {
        let row = sqlx::query("SELECT * FROM users WHERE id = ?")
            .bind(id_str(id))
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx)?
            .ok_or(StorageError::NotFound)?;
        row_to_user(&row)
    }

    async fn find_by_email(&self, email: &str) -> StorageResult<Option<User>> {
        let row = sqlx::query("SELECT * FROM users WHERE email = ?")
            .bind(email.to_lowercase())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx)?;
        row.as_ref().map(row_to_user).transpose()
    }

    async fn list(&self) -> StorageResult<Vec<User>> {
        let rows = sqlx::query("SELECT * FROM users ORDER BY created_at")
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?;
        rows.iter().map(row_to_user).collect()
    }

    async fn count(&self) -> StorageResult<i64> {
        let row = sqlx::query("SELECT COUNT(*) AS n FROM users")
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx)?;
        Ok(row.get("n"))
    }

    async fn set_password_hash(&self, id: UserId, password_hash: &str) -> StorageResult<()> {
        let n = sqlx::query("UPDATE users SET password_hash = ?, updated_at = ? WHERE id = ?")
            .bind(password_hash)
            .bind(now())
            .bind(id_str(id))
            .execute(&self.pool)
            .await
            .map_err(map_sqlx)?
            .rows_affected();
        if n == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    async fn set_status(&self, id: UserId, status: UserStatus) -> StorageResult<()> {
        let n = sqlx::query("UPDATE users SET status = ?, updated_at = ? WHERE id = ?")
            .bind(enum_str(&status))
            .bind(now())
            .bind(id_str(id))
            .execute(&self.pool)
            .await
            .map_err(map_sqlx)?
            .rows_affected();
        if n == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    async fn get_preferences(&self, id: UserId) -> StorageResult<serde_json::Value> {
        let row = sqlx::query("SELECT prefs_json FROM user_preferences WHERE user_id = ?")
            .bind(id_str(id))
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx)?;
        Ok(match row {
            Some(row) => {
                let text: String = row.get("prefs_json");
                serde_json::from_str(&text).unwrap_or_else(|_| serde_json::json!({}))
            }
            None => serde_json::json!({}),
        })
    }

    async fn set_preferences(&self, id: UserId, prefs: serde_json::Value) -> StorageResult<()> {
        self.get(id).await?; // NotFound si le compte n'existe pas
        sqlx::query(
            "INSERT INTO user_preferences (user_id, prefs_json, updated_at) VALUES (?, ?, ?) \
             ON CONFLICT(user_id) DO UPDATE SET prefs_json = excluded.prefs_json, \
             updated_at = excluded.updated_at",
        )
        .bind(id_str(id))
        .bind(json_text(&prefs))
        .bind(now())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    async fn delete(&self, id: UserId) -> StorageResult<()> {
        let n = sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(id_str(id))
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

// --- Projects ----------------------------------------------------------------

pub(crate) struct SqliteProjectRepo {
    pool: SqlitePool,
}
impl SqliteProjectRepo {
    pub(crate) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProjectRepo for SqliteProjectRepo {
    async fn create(&self, owner: UserId, project: NewProject) -> StorageResult<Project> {
        let id = ProjectId::new();
        let ts = now();
        sqlx::query(
            "INSERT INTO projects \
             (id, user_id, name, thumbnail_path, version, scene, active_presets, created_at, updated_at) \
             VALUES (?, ?, ?, ?, 1, ?, ?, ?, ?)",
        )
        .bind(id_str(id))
        .bind(id_str(owner))
        .bind(&project.name)
        .bind(&project.thumbnail_path)
        .bind(json_text(&project.scene))
        .bind(json_text(&project.active_presets))
        .bind(&ts)
        .bind(&ts)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        self.get(owner, id).await
    }

    async fn get(&self, owner: UserId, id: ProjectId) -> StorageResult<Project> {
        let row = sqlx::query("SELECT * FROM projects WHERE id = ? AND user_id = ?")
            .bind(id_str(id))
            .bind(id_str(owner))
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx)?
            .ok_or(StorageError::NotFound)?;
        row_to_project(&row)
    }

    async fn list(&self, owner: UserId) -> StorageResult<Vec<Project>> {
        let rows = sqlx::query("SELECT * FROM projects WHERE user_id = ? ORDER BY updated_at DESC")
            .bind(id_str(owner))
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?;
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
            "UPDATE projects SET scene = ?, active_presets = ?, thumbnail_path = ?, \
             version = version + 1, updated_at = ? \
             WHERE id = ? AND user_id = ? AND version = ? RETURNING *",
        )
        .bind(json_text(&scene))
        .bind(json_text(&active_presets))
        .bind(&thumbnail_path)
        .bind(now())
        .bind(id_str(id))
        .bind(id_str(owner))
        .bind(expected_version)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;

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
            "UPDATE projects SET name = ?, updated_at = ? WHERE id = ? AND user_id = ? RETURNING *",
        )
        .bind(name)
        .bind(now())
        .bind(id_str(id))
        .bind(id_str(owner))
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?
        .ok_or(StorageError::NotFound)?;
        row_to_project(&row)
    }

    async fn delete(&self, owner: UserId, id: ProjectId) -> StorageResult<()> {
        let n = sqlx::query("DELETE FROM projects WHERE id = ? AND user_id = ?")
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

// --- Models ------------------------------------------------------------------

pub(crate) struct SqliteModelRepo {
    pool: SqlitePool,
}
impl SqliteModelRepo {
    pub(crate) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ModelRepo for SqliteModelRepo {
    async fn create(&self, owner: UserId, model: NewModel) -> StorageResult<Model> {
        let id = ModelId::new();
        sqlx::query(
            "INSERT INTO models \
             (id, user_id, project_id, filename, format, file_path, mesh_path, size_bytes, triangle_count, repair_report) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id_str(id))
        .bind(id_str(owner))
        .bind(model.project_id.map(id_str))
        .bind(&model.filename)
        .bind(enum_str(&model.format))
        .bind(&model.file_path)
        .bind(&model.mesh_path)
        .bind(model.size_bytes)
        .bind(model.triangle_count)
        .bind(model.repair_report.as_ref().map(json_text))
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        self.get(owner, id).await
    }

    async fn get(&self, owner: UserId, id: ModelId) -> StorageResult<Model> {
        let row = sqlx::query("SELECT * FROM models WHERE id = ? AND user_id = ?")
            .bind(id_str(id))
            .bind(id_str(owner))
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx)?
            .ok_or(StorageError::NotFound)?;
        row_to_model(&row)
    }

    async fn list(&self, owner: UserId, project: Option<ProjectId>) -> StorageResult<Vec<Model>> {
        let rows = match project {
            Some(p) => {
                sqlx::query("SELECT * FROM models WHERE user_id = ? AND project_id = ?")
                    .bind(id_str(owner))
                    .bind(id_str(p))
                    .fetch_all(&self.pool)
                    .await
            }
            None => {
                sqlx::query("SELECT * FROM models WHERE user_id = ?")
                    .bind(id_str(owner))
                    .fetch_all(&self.pool)
                    .await
            }
        }
        .map_err(map_sqlx)?;
        rows.iter().map(row_to_model).collect()
    }

    async fn delete(&self, owner: UserId, id: ModelId) -> StorageResult<()> {
        let n = sqlx::query("DELETE FROM models WHERE id = ? AND user_id = ?")
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

    async fn set_mesh(
        &self,
        owner: UserId,
        id: ModelId,
        mesh_path: &str,
        triangle_count: i64,
    ) -> StorageResult<()> {
        let n = sqlx::query(
            "UPDATE models SET mesh_path = ?, triangle_count = ?, conversion_error = NULL \
             WHERE id = ? AND user_id = ?",
        )
        .bind(mesh_path)
        .bind(triangle_count)
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

    async fn mark_conversion_failed(
        &self,
        owner: UserId,
        id: ModelId,
        error: &str,
    ) -> StorageResult<()> {
        let n = sqlx::query("UPDATE models SET conversion_error = ? WHERE id = ? AND user_id = ?")
            .bind(error)
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

    async fn set_repair_result(
        &self,
        owner: UserId,
        id: ModelId,
        mesh_path: &str,
        triangle_count: i64,
        repair_report: serde_json::Value,
    ) -> StorageResult<()> {
        let n = sqlx::query(concat!(
            "UPDATE models SET mesh_path = ?, triangle_count = ?, repair_report = ?, ",
            "conversion_error = NULL WHERE id = ? AND user_id = ?"
        ))
        .bind(mesh_path)
        .bind(triangle_count)
        .bind(json_text(&repair_report))
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

// --- Presets -----------------------------------------------------------------

pub(crate) struct SqlitePresetRepo {
    pool: SqlitePool,
}
impl SqlitePresetRepo {
    pub(crate) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    async fn insert(&self, preset: &Preset) -> StorageResult<()> {
        sqlx::query(
            "INSERT INTO presets \
             (id, kind, name, origin, user_id, vendor, inherits, instantiation, setting_id, filament_id, compatible_printers, values_json) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id_str(preset.id))
        .bind(enum_str(&preset.kind))
        .bind(&preset.name)
        .bind(enum_str(&preset.origin))
        .bind(preset.user_id.map(id_str))
        .bind(&preset.vendor)
        .bind(&preset.inherits)
        .bind(preset.instantiation as i64)
        .bind(&preset.setting_id)
        .bind(&preset.filament_id)
        .bind(preset.compatible_printers.as_ref().map(json_text))
        .bind(json_text(&preset.values))
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }
}

#[async_trait]
impl PresetRepo for SqlitePresetRepo {
    async fn reseed_system(&self, presets: Vec<Preset>) -> StorageResult<u64> {
        let mut tx = begin_write(&self.pool).await.map_err(map_sqlx)?;
        type SystemKey = (String, String, Option<String>);

        // Réconciliation par clé naturelle : ne jamais supprimer/réinsérer un
        // preset conservé, car la FK des imprimantes a un ON DELETE CASCADE et
        // les projets retiennent son UUID dans un document JSON.
        let existing_rows =
            sqlx::query("SELECT id, kind, name, vendor FROM presets WHERE user_id IS NULL")
                .fetch_all(&mut *tx)
                .await
                .map_err(map_sqlx)?;
        let mut existing: Vec<(PresetId, SystemKey)> = Vec::with_capacity(existing_rows.len());
        let mut existing_by_key: HashMap<SystemKey, PresetId> = HashMap::new();
        for row in &existing_rows {
            let id = parse_id(&row.get::<String, _>("id"))?;
            let key = (
                row.get("kind"),
                row.get("name"),
                row.get::<Option<String>, _>("vendor"),
            );
            existing.push((id, key.clone()));
            existing_by_key.insert(key, id);
        }

        let mut batch_keys = HashSet::with_capacity(presets.len());
        let mut retained = HashSet::with_capacity(presets.len());
        for p in &presets {
            let key = (enum_str(&p.kind), p.name.clone(), p.vendor.clone());
            if !batch_keys.insert(key.clone()) {
                return Err(StorageError::Conflict(format!(
                    "preset système dupliqué dans le lot : {:?} / {}",
                    p.kind, p.name
                )));
            }
            let id = existing_by_key.get(&key).copied().unwrap_or(p.id);
            retained.insert(id);

            if existing_by_key.contains_key(&key) {
                sqlx::query(
                    "UPDATE presets SET origin = 'system', user_id = NULL, vendor = ?, \
                     inherits = ?, instantiation = ?, setting_id = ?, filament_id = ?, \
                     compatible_printers = ?, values_json = ? \
                     WHERE id = ? AND user_id IS NULL",
                )
                .bind(&p.vendor)
                .bind(&p.inherits)
                .bind(p.instantiation as i64)
                .bind(&p.setting_id)
                .bind(&p.filament_id)
                .bind(p.compatible_printers.as_ref().map(json_text))
                .bind(json_text(&p.values))
                .bind(id_str(id))
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx)?;
            } else {
                sqlx::query(
                    "INSERT INTO presets \
                     (id, kind, name, origin, user_id, vendor, inherits, instantiation, setting_id, filament_id, compatible_printers, values_json) \
                     VALUES (?, ?, ?, 'system', NULL, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(id_str(id))
                .bind(enum_str(&p.kind))
                .bind(&p.name)
                .bind(&p.vendor)
                .bind(&p.inherits)
                .bind(p.instantiation as i64)
                .bind(&p.setting_id)
                .bind(&p.filament_id)
                .bind(p.compatible_printers.as_ref().map(json_text))
                .bind(json_text(&p.values))
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx)?;
            }
        }

        let obsolete: Vec<PresetId> = existing
            .iter()
            .map(|(id, _)| *id)
            .filter(|id| !retained.contains(id))
            .collect();
        if !obsolete.is_empty() {
            let project_rows = sqlx::query("SELECT active_presets FROM projects")
                .fetch_all(&mut *tx)
                .await
                .map_err(map_sqlx)?;
            let mut project_refs = HashSet::new();
            for row in &project_rows {
                let active = parse_json(&row.get::<String, _>("active_presets"))?;
                collect_uuid_strings(&active, &mut project_refs);
            }
            for id in obsolete {
                let printer_ref = sqlx::query(
                    "SELECT 1 AS found FROM printers WHERE machine_preset_id = ? LIMIT 1",
                )
                .bind(id_str(id))
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_sqlx)?;
                if printer_ref.is_some() || project_refs.contains(&id.0) {
                    return Err(StorageError::Conflict(format!(
                        "reseed refusé : le preset système {id} est encore référencé"
                    )));
                }
                sqlx::query("DELETE FROM presets WHERE id = ? AND user_id IS NULL")
                    .bind(id_str(id))
                    .execute(&mut *tx)
                    .await
                    .map_err(map_sqlx)?;
            }
        }
        tx.commit().await.map_err(map_sqlx)?;
        Ok(presets.len() as u64)
    }

    async fn system_count(&self) -> StorageResult<i64> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM presets WHERE user_id IS NULL")
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx)?;
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
             WHERE kind = ? AND instantiation = 1 AND (user_id IS NULL OR user_id = ?) \
             ORDER BY origin, name",
        )
        .bind(enum_str(&kind))
        .bind(id_str(user))
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
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
        let row = sqlx::query("SELECT * FROM presets WHERE id = ?")
            .bind(id_str(id))
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx)?
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
             WHERE kind = ? AND (user_id IS NULL OR user_id = ?) ORDER BY name",
        )
        .bind(enum_str(&kind))
        .bind(id_str(user))
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        rows.iter().map(row_to_preset).collect()
    }

    async fn list_by_name(
        &self,
        kind: PresetKind,
        name: &str,
        user: UserId,
    ) -> StorageResult<Vec<Preset>> {
        let rows = sqlx::query(
            "SELECT * FROM presets \
             WHERE kind = ? AND name = ? AND (user_id IS NULL OR user_id = ?)",
        )
        .bind(enum_str(&kind))
        .bind(name)
        .bind(id_str(user))
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
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
            "UPDATE presets SET name = ?, values_json = ? \
             WHERE id = ? AND user_id = ? RETURNING *",
        )
        .bind(name)
        .bind(json_text(&values))
        .bind(id_str(id))
        .bind(id_str(owner))
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?
        .ok_or(StorageError::NotFound)?;
        row_to_preset(&row)
    }

    async fn detach_user_preset(
        &self,
        owner: UserId,
        id: PresetId,
        values: serde_json::Value,
    ) -> StorageResult<Preset> {
        let row = sqlx::query(
            "UPDATE presets SET inherits = NULL, values_json = ? \
             WHERE id = ? AND user_id = ? RETURNING *",
        )
        .bind(json_text(&values))
        .bind(id_str(id))
        .bind(id_str(owner))
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?
        .ok_or(StorageError::NotFound)?;
        row_to_preset(&row)
    }

    async fn delete_user_preset(&self, owner: UserId, id: PresetId) -> StorageResult<()> {
        let mut tx = begin_write(&self.pool).await.map_err(map_sqlx)?;
        let owned = sqlx::query("SELECT 1 AS found FROM presets WHERE id = ? AND user_id = ?")
            .bind(id_str(id))
            .bind(id_str(owner))
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_sqlx)?;
        if owned.is_none() {
            return Err(StorageError::NotFound);
        }
        let printer_ref =
            sqlx::query("SELECT 1 AS found FROM printers WHERE machine_preset_id = ? LIMIT 1")
                .bind(id_str(id))
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_sqlx)?;
        if printer_ref.is_some() {
            return Err(StorageError::Conflict(
                "preset référencé par une imprimante".into(),
            ));
        }

        // SQLite/Postgres ne peuvent pas déclarer une FK vers des UUID contenus
        // dans du JSON. Le filtre SQL limite les lectures, puis la comparaison
        // exacte du JSON élimine les simples occurrences textuelles.
        let candidates =
            sqlx::query("SELECT active_presets FROM projects WHERE active_presets LIKE ?")
                .bind(format!("%{}%", id_str(id)))
                .fetch_all(&mut *tx)
                .await
                .map_err(map_sqlx)?;
        for row in &candidates {
            let active = parse_json(&row.get::<String, _>("active_presets"))?;
            if json_references_id(&active, id.into()) {
                return Err(StorageError::Conflict(
                    "preset référencé par un projet".into(),
                ));
            }
        }

        sqlx::query("DELETE FROM presets WHERE id = ? AND user_id = ?")
            .bind(id_str(id))
            .bind(id_str(owner))
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }
}

// --- Printers ----------------------------------------------------------------

async fn ensure_machine_preset_visible(
    conn: &mut SqliteConnection,
    owner: UserId,
    id: PresetId,
) -> StorageResult<()> {
    let row = sqlx::query("SELECT user_id, kind FROM presets WHERE id = ?")
        .bind(id_str(id))
        .fetch_optional(conn)
        .await
        .map_err(map_sqlx)?
        .ok_or(StorageError::NotFound)?;
    let preset_owner = row.get::<Option<String>, _>("user_id");
    if preset_owner.is_some() && preset_owner != Some(id_str(owner)) {
        return Err(StorageError::NotFound);
    }
    let kind: PresetKind = parse_enum(&row.get::<String, _>("kind"))?;
    if kind != PresetKind::Machine {
        return Err(StorageError::Conflict(
            "machine_preset_id doit référencer un preset machine".into(),
        ));
    }
    Ok(())
}

pub(crate) struct SqlitePrinterRepo {
    pool: SqlitePool,
}
impl SqlitePrinterRepo {
    pub(crate) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PrinterRepo for SqlitePrinterRepo {
    async fn set_host(
        &self,
        owner: UserId,
        id: PrinterId,
        host_type: &str,
        host_options: &serde_json::Value,
    ) -> StorageResult<Printer> {
        let n = sqlx::query(
            "UPDATE printers SET host_type = ?, host_options = ? WHERE id = ? AND user_id = ?",
        )
        .bind(host_type)
        .bind(host_options.to_string())
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

    async fn create(&self, owner: UserId, printer: NewPrinter) -> StorageResult<Printer> {
        let mut tx = begin_write(&self.pool).await.map_err(map_sqlx)?;
        ensure_machine_preset_visible(&mut tx, owner, printer.machine_preset_id).await?;
        let id = PrinterId::new();
        sqlx::query(
            "INSERT INTO printers (id, user_id, name, moonraker_url, api_key, machine_preset_id) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id_str(id))
        .bind(id_str(owner))
        .bind(&printer.name)
        .bind(&printer.moonraker_url)
        .bind(&printer.api_key)
        .bind(id_str(printer.machine_preset_id))
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        self.get(owner, id).await
    }

    async fn get(&self, owner: UserId, id: PrinterId) -> StorageResult<Printer> {
        let row = sqlx::query("SELECT * FROM printers WHERE id = ? AND user_id = ?")
            .bind(id_str(id))
            .bind(id_str(owner))
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx)?
            .ok_or(StorageError::NotFound)?;
        row_to_printer(&row)
    }

    async fn list(&self, owner: UserId) -> StorageResult<Vec<Printer>> {
        let rows = sqlx::query("SELECT * FROM printers WHERE user_id = ? ORDER BY name")
            .bind(id_str(owner))
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?;
        rows.iter().map(row_to_printer).collect()
    }

    async fn update(
        &self,
        owner: UserId,
        id: PrinterId,
        printer: NewPrinter,
    ) -> StorageResult<Printer> {
        let mut tx = begin_write(&self.pool).await.map_err(map_sqlx)?;
        ensure_machine_preset_visible(&mut tx, owner, printer.machine_preset_id).await?;
        let n = sqlx::query(
            "UPDATE printers SET name = ?, moonraker_url = ?, api_key = ?, machine_preset_id = ? \
             WHERE id = ? AND user_id = ?",
        )
        .bind(&printer.name)
        .bind(&printer.moonraker_url)
        .bind(&printer.api_key)
        .bind(id_str(printer.machine_preset_id))
        .bind(id_str(id))
        .bind(id_str(owner))
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?
        .rows_affected();
        if n == 0 {
            return Err(StorageError::NotFound);
        }
        tx.commit().await.map_err(map_sqlx)?;
        self.get(owner, id).await
    }

    async fn delete(&self, owner: UserId, id: PrinterId) -> StorageResult<()> {
        let n = sqlx::query("DELETE FROM printers WHERE id = ? AND user_id = ?")
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

// --- Jobs --------------------------------------------------------------------

pub(crate) struct SqliteJobRepo {
    pool: SqlitePool,
}
impl SqliteJobRepo {
    pub(crate) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    async fn insert(
        &self,
        owner: UserId,
        job: NewJob,
        scene: Option<serde_json::Value>,
    ) -> StorageResult<SlicingJob> {
        let id = JobId::new();
        let ts = now();
        sqlx::query(
            "INSERT INTO slicing_jobs \
             (id, user_id, project_id, plate_index, status, progress, phase, resolved_settings, scene, created_at, updated_at) \
             VALUES (?, ?, ?, ?, 'queued', 0, '', ?, ?, ?, ?)",
        )
        .bind(id_str(id))
        .bind(id_str(owner))
        .bind(id_str(job.project_id))
        .bind(job.plate_index)
        .bind(json_text(&job.resolved_settings))
        .bind(scene.as_ref().map(json_text))
        .bind(&ts)
        .bind(&ts)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        self.get(owner, id).await
    }
}

#[async_trait]
impl JobRepo for SqliteJobRepo {
    async fn enqueue(&self, owner: UserId, job: NewJob) -> StorageResult<SlicingJob> {
        self.insert(owner, job, None).await
    }

    async fn enqueue_with_scene(
        &self,
        owner: UserId,
        job: NewJob,
        scene: serde_json::Value,
    ) -> StorageResult<SlicingJob> {
        self.insert(owner, job, Some(scene)).await
    }

    async fn get(&self, owner: UserId, id: JobId) -> StorageResult<SlicingJob> {
        let row = sqlx::query("SELECT * FROM slicing_jobs WHERE id = ? AND user_id = ?")
            .bind(id_str(id))
            .bind(id_str(owner))
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx)?
            .ok_or(StorageError::NotFound)?;
        row_to_job(&row)
    }

    async fn list(&self, owner: UserId) -> StorageResult<Vec<SlicingJob>> {
        let rows =
            sqlx::query("SELECT * FROM slicing_jobs WHERE user_id = ? ORDER BY created_at DESC")
                .bind(id_str(owner))
                .fetch_all(&self.pool)
                .await
                .map_err(map_sqlx)?;
        rows.iter().map(row_to_job).collect()
    }

    async fn claim_next(&self) -> StorageResult<Option<SlicingJob>> {
        // UPDATE ... RETURNING atomique : sous le verrou d'écriture SQLite, un
        // seul appelant réclame un job donné (pas de double délivrance).
        let row = sqlx::query(
            "UPDATE slicing_jobs SET status = 'running', updated_at = ? \
             WHERE id = (SELECT id FROM slicing_jobs WHERE status = 'queued' ORDER BY created_at LIMIT 1) \
             RETURNING *",
        )
        .bind(now())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        row.as_ref().map(row_to_job).transpose()
    }

    async fn requeue_running(&self) -> StorageResult<u64> {
        let n = sqlx::query(
            "UPDATE slicing_jobs SET status = 'queued', updated_at = ? WHERE status = 'running'",
        )
        .bind(now())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?
        .rows_affected();
        Ok(n)
    }

    async fn update_progress(&self, id: JobId, progress: f64, phase: &str) -> StorageResult<()> {
        let n = sqlx::query(
            "UPDATE slicing_jobs SET progress = ?, phase = ?, updated_at = ? WHERE id = ?",
        )
        .bind(progress)
        .bind(phase)
        .bind(now())
        .bind(id_str(id))
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?
        .rows_affected();
        if n == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    async fn finish(&self, id: JobId, outcome: JobOutcome) -> StorageResult<()> {
        let (status, gcode_id, error) = match outcome {
            JobOutcome::Succeeded { gcode_id } => ("succeeded", Some(id_str(gcode_id)), None),
            JobOutcome::Failed { error } => ("failed", None, Some(json_text(&error))),
        };
        let n = sqlx::query(
            "UPDATE slicing_jobs SET status = ?, gcode_id = ?, error = ?, progress = 1, updated_at = ? WHERE id = ?",
        )
        .bind(status)
        .bind(gcode_id)
        .bind(error)
        .bind(now())
        .bind(id_str(id))
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?
        .rows_affected();
        if n == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    async fn cancel(&self, owner: UserId, id: JobId) -> StorageResult<()> {
        let n = sqlx::query(
            "UPDATE slicing_jobs SET status = 'cancelled', updated_at = ? \
             WHERE id = ? AND user_id = ? AND status IN ('queued', 'running')",
        )
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
        Ok(())
    }
}

// --- Gcodes ------------------------------------------------------------------

pub(crate) struct SqliteGcodeRepo {
    pool: SqlitePool,
}
impl SqliteGcodeRepo {
    pub(crate) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GcodeRepo for SqliteGcodeRepo {
    async fn create(&self, owner: UserId, gcode: NewGcode) -> StorageResult<Gcode> {
        let id = GcodeId::new();
        sqlx::query(
            "INSERT INTO gcodes (id, user_id, job_id, file_path, preview_path, stats, thumbnails) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id_str(id))
        .bind(id_str(owner))
        .bind(id_str(gcode.job_id))
        .bind(&gcode.file_path)
        .bind(&gcode.preview_path)
        .bind(json_text(&gcode.stats))
        .bind(json_text(&gcode.thumbnails))
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        self.get(owner, id).await
    }

    async fn get(&self, owner: UserId, id: GcodeId) -> StorageResult<Gcode> {
        let row = sqlx::query("SELECT * FROM gcodes WHERE id = ? AND user_id = ?")
            .bind(id_str(id))
            .bind(id_str(owner))
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx)?
            .ok_or(StorageError::NotFound)?;
        row_to_gcode(&row)
    }

    async fn delete(&self, owner: UserId, id: GcodeId) -> StorageResult<()> {
        let n = sqlx::query("DELETE FROM gcodes WHERE id = ? AND user_id = ?")
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

// --- Instance (réglages + invitations) --------------------------------------

pub(crate) struct SqliteInstanceRepo {
    pool: SqlitePool,
}
impl SqliteInstanceRepo {
    pub(crate) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Garantit la ligne singleton (id=1) avec les défauts.
    async fn ensure_row(&self) -> StorageResult<()> {
        sqlx::query(
            "INSERT INTO instance_settings (id, registration_policy, upload_limit_bytes) \
             VALUES (1, 'open', 524288000) ON CONFLICT(id) DO NOTHING",
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }
}

#[async_trait]
impl InstanceRepo for SqliteInstanceRepo {
    async fn settings(&self) -> StorageResult<InstanceSettings> {
        self.ensure_row().await?;
        let row = sqlx::query("SELECT * FROM instance_settings WHERE id = 1")
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx)?;
        Ok(InstanceSettings {
            registration_policy: parse_enum(&row.get::<String, _>("registration_policy"))?,
            upload_limit_bytes: row.get("upload_limit_bytes"),
            storage_quota_bytes: row.get("storage_quota_bytes"),
            max_concurrent_slices: row.get("max_concurrent_slices"),
        })
    }

    async fn set_registration_policy(&self, policy: RegistrationPolicy) -> StorageResult<()> {
        self.ensure_row().await?;
        sqlx::query("UPDATE instance_settings SET registration_policy = ? WHERE id = 1")
            .bind(enum_str(&policy))
            .execute(&self.pool)
            .await
            .map_err(map_sqlx)?;
        Ok(())
    }

    async fn set_upload_limit(&self, bytes: i64) -> StorageResult<()> {
        self.ensure_row().await?;
        sqlx::query("UPDATE instance_settings SET upload_limit_bytes = ? WHERE id = 1")
            .bind(bytes)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx)?;
        Ok(())
    }

    async fn set_storage_quota(&self, bytes: i64) -> StorageResult<()> {
        self.ensure_row().await?;
        sqlx::query("UPDATE instance_settings SET storage_quota_bytes = ? WHERE id = 1")
            .bind(bytes)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx)?;
        Ok(())
    }

    async fn set_max_concurrent_slices(&self, n: i64) -> StorageResult<()> {
        self.ensure_row().await?;
        sqlx::query("UPDATE instance_settings SET max_concurrent_slices = ? WHERE id = 1")
            .bind(n)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx)?;
        Ok(())
    }

    async fn create_invitation(&self, invitation: NewInvitation) -> StorageResult<Invitation> {
        let id = InvitationId::new();
        let ts = now();
        let expires = invitation
            .expires_at
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|e| StorageError::Backend(e.to_string()))?;
        sqlx::query(
            "INSERT INTO invitations (id, token, issued_by, expires_at, used, created_at) \
             VALUES (?, ?, ?, ?, 0, ?)",
        )
        .bind(id_str(id))
        .bind(&invitation.token)
        .bind(id_str(invitation.issued_by))
        .bind(&expires)
        .bind(&ts)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(Invitation {
            id,
            token: invitation.token,
            issued_by: invitation.issued_by,
            expires_at: invitation.expires_at,
            used: false,
            created_at: parse_time(&ts)?,
        })
    }

    async fn consume_invitation(&self, token: &str) -> StorageResult<Invitation> {
        let row = sqlx::query(
            "UPDATE invitations SET used = 1 \
             WHERE token = ? AND used = 0 AND expires_at > ? RETURNING *",
        )
        .bind(token)
        .bind(now())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?
        .ok_or(StorageError::NotFound)?;
        row_to_invitation(&row)
    }

    async fn list_invitations(&self) -> StorageResult<Vec<Invitation>> {
        let rows = sqlx::query("SELECT * FROM invitations ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?;
        rows.iter().map(row_to_invitation).collect()
    }
}
