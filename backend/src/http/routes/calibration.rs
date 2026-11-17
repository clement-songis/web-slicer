//! Calibrations d'Orca côté serveur.
//!
//! - `POST /api/projects/{id}/calibration` : construit le projet de calibration
//!   par libslic3r (portage des Plater::calib_* : modèle de resources/calib,
//!   coupes, échelle, réglages d'objet) dans le projet `id` ; les presets édités
//!   par Orca sont conservés dans `scene.calibrationOverrides` (appliqués au
//!   tranchage et à l'export par-dessus les presets) et les paramètres du mode
//!   calibration dans `scene.calibration` (Print::set_calib_params).
//! - `/api/calibrations/pa` : historique des avances de pression retenues
//!   (PACalibResult / CaliHistoryDialog d'Orca), par imprimante/filament/buse.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::domain::{
    NewPaCalibration, PaCalibration, PaCalibrationFilter, PaCalibrationId, PrinterId, ProjectId,
};
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;

fn parse<T: From<uuid::Uuid>>(raw: &str, what: &str) -> ApiResult<T> {
    uuid::Uuid::parse_str(raw)
        .map(T::from)
        .map_err(|_| ApiError::not_found(what))
}

/// Dossier resources d'Orca (frère du dossier des profils).
fn resources_dir(state: &AppState) -> std::path::PathBuf {
    state
        .profiles_dir
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| state.profiles_dir.clone())
}

pub async fn create_project_calibration(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(raw_id): Path<String>,
    Json(request): Json<engine::api::calib::CalibRequest>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let project_id: ProjectId = parse(&raw_id, "Projet")?;
    let project = state.storage.projects().get(user.id, project_id).await?;
    if !engine::api::calib::CALIB_KINDS.contains(&request.kind.as_str()) {
        return Err(ApiError::validation(
            "calibration inconnue",
            json!({ "kind": request.kind, "kinds": engine::api::calib::CALIB_KINDS }),
        ));
    }
    let (mut config, _) =
        super::slice::resolve_active_config(&state, user.id, &project.active_presets).await?;
    super::slice::apply_project_config(&mut config, &project.scene);

    // Projet de calibration par engine-worker (libslic3r).
    let dir = crate::workdir::tempdir().map_err(|_| ApiError::internal())?;
    let req_path = dir.path().join("calib.json");
    std::fs::write(
        &req_path,
        serde_json::to_vec(
            &json!({ "request": request, "config": config, "resources": resources_dir(&state) }),
        )
        .map_err(|_| ApiError::internal())?,
    )
    .map_err(|_| ApiError::internal())?;
    let stdout = crate::engine::run_worker(
        &["calib-project", &req_path.to_string_lossy()],
        std::time::Duration::from_secs(170),
    )
    .await
    .map_err(|e| {
        ApiError::validation(
            format!("calibration : {e}"),
            json!({ "kind": request.kind }),
        )
    })?;
    let stdout = String::from_utf8_lossy(&stdout);
    let line = stdout
        .lines()
        .rev()
        .find_map(|l| l.strip_prefix("R "))
        .ok_or_else(ApiError::internal)?;
    let calib: engine::api::calib::CalibProject = serde_json::from_str(line).map_err(|e| {
        tracing::error!(error = %e, "réponse calib-project");
        ApiError::internal()
    })?;

    // Objets → modèles + scène, par le chemin d'import 3MF (store_bbs_3mf).
    let bytes = crate::engine::project_3mf::write_project_3mf(
        &calib.model,
        &engine::api::DynamicPrintConfig::new(),
        &[],
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "3MF de calibration");
        ApiError::internal()
    })?;
    let mut imported = crate::models::threemf::read_project(&bytes)
        .await
        .map_err(|e| ApiError::validation(e.to_string(), json!({})))?;
    imported.config.clear();
    let filename = format!("{}.3mf", calib.name);
    let (mut scene, _, models) = super::projects::import_3mf_contents(
        &state,
        user.id,
        project_id,
        &calib.name,
        &filename,
        imported,
    )
    .await?;
    if let Some(cfg) = project.scene.get("projectConfig") {
        scene["projectConfig"] = cfg.clone();
    }
    let overrides = Value::Object(super::presets::config_to_json(&calib.overrides));
    scene["calibrationOverrides"] = overrides.clone();
    if let Some(params) = &calib.calib {
        scene["calibration"] = serde_json::to_value(params).map_err(|_| ApiError::internal())?;
    }
    let updated = state
        .storage
        .projects()
        .update(
            user.id,
            project_id,
            project.version,
            scene.clone(),
            project.active_presets.clone(),
            project.thumbnail_path.clone(),
        )
        .await?;
    for (model, mesh) in models {
        let converter = state.converter.clone();
        tokio::spawn(async move { converter.convert_mesh(model, mesh).await });
    }
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "name": calib.name,
            "version": updated.version,
            "scene": scene,
            "calibration": calib.calib,
            "overrides": overrides,
        })),
    ))
}

// --- Historique PA -----------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct PaQuery {
    pub printer_id: Option<String>,
    pub machine_preset: Option<String>,
    pub filament_id: Option<String>,
    pub filament_preset: Option<String>,
    pub nozzle_diameter: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct PaBody {
    #[serde(default)]
    pub printer_id: Option<String>,
    pub machine_preset: String,
    pub filament_preset: String,
    #[serde(default)]
    pub filament_id: Option<String>,
    pub nozzle_diameter: f64,
    #[serde(default)]
    pub extruder_id: i64,
    #[serde(default)]
    pub nozzle_volume_type: Option<String>,
    pub k_value: f64,
    #[serde(default = "one")]
    pub n_coef: f64,
    pub method: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub note: Option<String>,
}

fn one() -> f64 {
    1.0
}

const METHODS: [&str; 4] = ["pa_line", "pa_pattern", "pa_tower", "manual"];

fn validate(body: PaBody) -> ApiResult<NewPaCalibration> {
    let bad = |field: &str, m: &str| ApiError::validation(m.to_string(), json!({ "field": field }));
    if body.machine_preset.trim().is_empty() {
        return Err(bad("machine_preset", "imprimante requise"));
    }
    if body.filament_preset.trim().is_empty() {
        return Err(bad("filament_preset", "filament requis"));
    }
    if !(body.nozzle_diameter.is_finite()
        && body.nozzle_diameter > 0.0
        && body.nozzle_diameter <= 2.0)
    {
        return Err(bad("nozzle_diameter", "diamètre de buse invalide"));
    }
    // Bornes de pressure_advance dans Orca (PrintConfig.cpp : 0 → 2).
    if !(body.k_value.is_finite() && (0.0..=2.0).contains(&body.k_value)) {
        return Err(bad("k_value", "avance de pression hors de [0 ; 2]"));
    }
    if !body.n_coef.is_finite() || body.extruder_id < 0 {
        return Err(bad("n_coef", "valeur invalide"));
    }
    if !METHODS.contains(&body.method.as_str()) {
        return Err(bad(
            "method",
            "méthode : pa_line | pa_pattern | pa_tower | manual",
        ));
    }
    let printer_id: Option<PrinterId> = match body.printer_id.as_deref().filter(|s| !s.is_empty()) {
        Some(raw) => Some(parse(raw, "Imprimante")?),
        None => None,
    };
    let name = if body.name.trim().is_empty() {
        format!("{} {}", body.filament_preset, body.nozzle_diameter)
    } else {
        body.name
    };
    Ok(NewPaCalibration {
        printer_id,
        machine_preset: body.machine_preset,
        filament_preset: body.filament_preset,
        filament_id: body.filament_id.filter(|s| !s.is_empty()),
        nozzle_diameter: body.nozzle_diameter,
        extruder_id: body.extruder_id,
        nozzle_volume_type: body.nozzle_volume_type.filter(|s| !s.is_empty()),
        k_value: body.k_value,
        n_coef: body.n_coef,
        method: body.method,
        name,
        note: body.note.filter(|s| !s.is_empty()),
    })
}

fn to_json(p: PaCalibration) -> Value {
    let rfc = time::format_description::well_known::Rfc3339;
    let mut out: Map<String, Value> = match serde_json::to_value(&p.data) {
        Ok(Value::Object(m)) => m,
        _ => Map::new(),
    };
    out.insert("id".into(), p.id.to_string().into());
    out.insert(
        "created_at".into(),
        p.created_at.format(&rfc).unwrap_or_default().into(),
    );
    out.insert(
        "updated_at".into(),
        p.updated_at.format(&rfc).unwrap_or_default().into(),
    );
    Value::Object(out)
}

pub async fn list_pa(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Query(q): Query<PaQuery>,
) -> ApiResult<Json<Value>> {
    let printer_id = match q.printer_id.as_deref().filter(|s| !s.is_empty()) {
        Some(raw) => Some(parse(raw, "Imprimante")?),
        None => None,
    };
    let filter = PaCalibrationFilter {
        printer_id,
        machine_preset: q.machine_preset,
        filament_id: q.filament_id,
        filament_preset: q.filament_preset,
        nozzle_diameter: q.nozzle_diameter,
    };
    let items = state
        .storage
        .pa_calibrations()
        .list(user.id, &filter)
        .await?;
    Ok(Json(Value::Array(items.into_iter().map(to_json).collect())))
}

pub async fn create_pa(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Json(body): Json<PaBody>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let data = validate(body)?;
    let created = state
        .storage
        .pa_calibrations()
        .create(user.id, data)
        .await?;
    Ok((StatusCode::CREATED, Json(to_json(created))))
}

pub async fn get_pa(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(raw): Path<String>,
) -> ApiResult<Json<Value>> {
    let id: PaCalibrationId = parse(&raw, "Calibration")?;
    Ok(Json(to_json(
        state.storage.pa_calibrations().get(user.id, id).await?,
    )))
}

pub async fn update_pa(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(raw): Path<String>,
    Json(body): Json<PaBody>,
) -> ApiResult<Json<Value>> {
    let id: PaCalibrationId = parse(&raw, "Calibration")?;
    let data = validate(body)?;
    Ok(Json(to_json(
        state
            .storage
            .pa_calibrations()
            .update(user.id, id, data)
            .await?,
    )))
}

pub async fn delete_pa(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(raw): Path<String>,
) -> ApiResult<StatusCode> {
    let id: PaCalibrationId = parse(&raw, "Calibration")?;
    state.storage.pa_calibrations().delete(user.id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
