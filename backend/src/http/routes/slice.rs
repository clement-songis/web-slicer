//! Lancement du tranchage (`POST /api/projects/{id}/slice`, contrat http-api.md,
//! T064).
//!
//! Fige les presets actifs résolus (`resolved_settings`, reproductibilité),
//! restitue les avertissements moteur (FR-032, bornes/enums clampés), refuse de
//! trancher un plateau vide ou contenant un objet hors plateau (erreur avant
//! slice), puis crée un job `queued` par plateau ciblé. L'exécution est prise en
//! charge par le pool de workers (T063) via le runner FFI (T066).

use std::collections::HashSet;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde_json::{json, Value};

use super::presets;
use crate::domain::{NewJob, PresetId, ProjectId};
use crate::http::dto::{JobResponse, SliceRequest, SliceResponse, SliceWarning};
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;

fn parse_project_id(raw: &str) -> ApiResult<ProjectId> {
    uuid::Uuid::parse_str(raw)
        .map(ProjectId)
        .map_err(|_| ApiError::not_found("Projet"))
}

/// `POST /api/projects/{id}/slice` — crée un ou plusieurs jobs de tranchage.
pub async fn slice(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(project_raw): Path<String>,
    Json(body): Json<SliceRequest>,
) -> ApiResult<(StatusCode, Json<SliceResponse>)> {
    let project_id = parse_project_id(&project_raw)?;
    let project = state.storage.projects().get(user.id, project_id).await?; // 404, SC-008

    let plates = plate_object_ids(&project.scene);
    let plate_count = plates.len().max(1);

    // Cible : un plateau précis (borné) ou tous les plateaux préparés.
    let targets: Vec<i64> = match body.plate_index {
        Some(n) => {
            if n < 0 || n as usize >= plate_count {
                return Err(ApiError::validation(
                    "plate_index hors limites",
                    json!({ "plate_index": n, "plate_count": plate_count }),
                ));
            }
            vec![n]
        }
        None => (0..plate_count as i64).collect(),
    };

    // Garde avant slice : chaque plateau ciblé doit contenir des objets, tous
    // sur le plateau (FR-032, « hors plateau → erreur avant slice »).
    let off_bed = off_bed_ids(&project.scene);
    for &t in &targets {
        let ids = plates
            .get(t as usize)
            .cloned()
            .unwrap_or_else(|| all_object_ids(&project.scene));
        if ids.is_empty() {
            return Err(ApiError::validation(
                "plateau vide (aucun objet à trancher)",
                json!({ "plate_index": t }),
            ));
        }
        if ids.iter().any(|id| off_bed.contains(id)) {
            return Err(ApiError::validation(
                "objet hors plateau : repositionner avant de trancher",
                json!({ "plate_index": t }),
            ));
        }
    }

    // Presets actifs résolus (figés) + avertissements moteur.
    let (resolved, warnings) = resolve_active(&state, user.id, &project.active_presets).await?;

    let mut jobs = Vec::with_capacity(targets.len());
    for &t in &targets {
        let job = state
            .storage
            .jobs()
            .enqueue(
                user.id,
                NewJob {
                    project_id,
                    plate_index: t,
                    resolved_settings: resolved.clone(),
                },
            )
            .await?;
        jobs.push(JobResponse::from(job));
    }

    Ok((StatusCode::CREATED, Json(SliceResponse { jobs, warnings })))
}

/// Résout les presets actifs (`{printer, process, filaments[]}`) en une config
/// unique figée ; les valeurs hors bornes sont clampées et signalées (FR-032).
async fn resolve_active(
    state: &AppState,
    user: crate::domain::UserId,
    active: &Value,
) -> ApiResult<(Value, Vec<SliceWarning>)> {
    let (merged, warnings) = resolve_active_config(state, user, active).await?;
    Ok((Value::Object(presets::config_to_json(&merged)), warnings))
}

/// Comme [`resolve_active`], mais renvoie la `DynamicPrintConfig` typée (export
/// 3MF, T072) plutôt que sa projection JSON.
pub(crate) async fn resolve_active_config(
    state: &AppState,
    user: crate::domain::UserId,
    active: &Value,
) -> ApiResult<(engine::api::DynamicPrintConfig, Vec<SliceWarning>)> {
    // Garde de possession (Phase 14) : le preset imprimante retenu doit
    // correspondre à une imprimante **déclarée** par l'utilisateur — on ne
    // tranche/exporte pas pour une imprimante qu'il ne possède pas. L'absence de
    // clé `printer` reste tolérée (config par défaut) ; seule une valeur non
    // possédée est refusée.
    if let Some(s) = active.get("printer").and_then(Value::as_str) {
        let printer_id = parse_preset_id(s)?;
        let owned = state.storage.printers().list(user).await?;
        let owns_it = owned
            .iter()
            .any(|p| p.machine_preset_id.to_string() == printer_id.to_string());
        if !owns_it {
            return Err(ApiError::printer_not_owned(
                "Imprimante non possédée : ajoutez-la à vos imprimantes avant de trancher",
            ));
        }
    }

    let mut ids: Vec<PresetId> = Vec::new();
    for key in ["printer", "process"] {
        if let Some(s) = active.get(key).and_then(Value::as_str) {
            ids.push(parse_preset_id(s)?);
        }
    }
    if let Some(arr) = active.get("filaments").and_then(Value::as_array) {
        for v in arr {
            if let Some(s) = v.as_str() {
                ids.push(parse_preset_id(s)?);
            }
        }
    }

    let mut merged = engine::api::DynamicPrintConfig::new();
    for id in ids {
        let leaf = presets::load_visible(state, user, id).await?;
        let candidates = state
            .storage
            .presets()
            .list_by_kind(leaf.kind, user)
            .await?;
        let chain = presets::build_chain(&candidates, &leaf);
        let cfg = engine::presets::resolve_preset_chain(&chain).map_err(|e| {
            tracing::error!(error = %e, "résolution de preset pour le slice");
            ApiError::internal()
        })?;
        merged.0.extend(cfg.0);
    }

    let warnings = merged
        .validate()
        .into_iter()
        .map(|w| SliceWarning {
            key: w.key,
            message: w.message,
        })
        .collect();

    Ok((merged, warnings))
}

fn parse_preset_id(raw: &str) -> ApiResult<PresetId> {
    uuid::Uuid::parse_str(raw)
        .map(PresetId)
        .map_err(|_| ApiError::validation("preset actif invalide", json!({ "id": raw })))
}

// --- Lecture tolérante du document scène -------------------------------------

/// Liste des `objectIds` par plateau (deux formes de scène tolérées : tableau de
/// plateaux, ou document `{ plates: [...], activeId }`). Vide si aucun plateau.
fn plate_object_ids(scene: &Value) -> Vec<Vec<String>> {
    let plates = match scene.get("plates") {
        Some(Value::Array(a)) => a.clone(),
        Some(Value::Object(o)) => o
            .get("plates")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    plates
        .iter()
        .map(|p| {
            p.get("objectIds")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect()
}

/// Tous les identifiants d'objets de la scène (repli quand aucun plateau).
fn all_object_ids(scene: &Value) -> Vec<String> {
    scene
        .get("objects")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|o| o.get("id").and_then(Value::as_str).map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// Identifiants d'objets marqués hors plateau (`off_bed: true`) par le client.
fn off_bed_ids(scene: &Value) -> HashSet<String> {
    scene
        .get("objects")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter(|o| o.get("off_bed").and_then(Value::as_bool).unwrap_or(false))
                .filter_map(|o| o.get("id").and_then(Value::as_str).map(String::from))
                .collect()
        })
        .unwrap_or_default()
}
