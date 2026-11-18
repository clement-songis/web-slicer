//! Plateaux tranchés en lot (« Slice all » d'Orca déjà couvert par
//! `POST /api/projects/{id}/slice` sans `plate_index`, un job par plateau) :
//!
//! - `POST /api/projects/{id}/export/sliced` — « Export plate sliced file » /
//!   « Export all plate sliced file » (`Plater::export_gcode_3mf`) : .gcode.3mf
//!   écrit par libslic3r (`store_bbs_3mf`, WithGcode | SkipModel) avec un
//!   `Metadata/plate_<n>.gcode` par plateau tranché ;
//! - `POST /api/printers/{id}/upload-sliced` — « Send all » / « Print all »
//!   (`Plater::send_gcode_legacy(PLATE_ALL_IDX)`) : si l'imprimante accepte le
//!   3MF (`use_3mf`), un seul .gcode.3mf ; sinon un G-code par plateau
//!   (`<projet>_plateau-<n>.gcode`). Le démarrage immédiat n'est possible que
//!   pour un fichier unique.

use std::collections::BTreeMap;

use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::domain::{JobId, JobStatus, ProjectId, SlicingJob, UserId};
use crate::engine::gcode_3mf::PlateGcode;
use crate::http::dto::PrinterUploadResponse;
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;

/// Corps commun : jobs de tranchage (un par plateau) d'un même projet.
#[derive(Debug, Deserialize)]
pub struct SlicedRequest {
    /// Jobs réussis du projet (ceux renvoyés par `POST /slice`).
    pub job_ids: Vec<String>,
    /// Un seul plateau (« Export plate sliced file ») ; absent = tous.
    #[serde(default)]
    pub plate_index: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct UploadSlicedRequest {
    pub project_id: String,
    pub job_ids: Vec<String>,
    #[serde(default)]
    pub start_now: bool,
}

#[derive(Debug, Serialize)]
pub struct UploadSlicedResponse {
    /// `gcode_3mf` (un fichier) ou `gcode` (un fichier par plateau).
    pub format: &'static str,
    pub files: Vec<PrinterUploadResponse>,
}

fn validation(message: &str, details: serde_json::Value) -> ApiError {
    ApiError::validation(message, details)
}

/// Plateaux tranchés prêts à empaqueter : modèle complet (tous plateaux) et
/// config figée du job le plus récent, G-code stocké par plateau (le plus
/// récent si un plateau a plusieurs jobs).
struct SlicedBundle {
    model: engine::api::Model,
    config: engine::api::DynamicPrintConfig,
    gcodes: Vec<PlateGcode>,
    /// `gcodes.stats` de chaque plateau (même ordre que `gcodes`) : nom Orca.
    stats: Vec<serde_json::Value>,
    project_name: String,
}

async fn sliced_bundle(
    state: &AppState,
    user: UserId,
    project_id: ProjectId,
    job_ids: &[String],
) -> ApiResult<SlicedBundle> {
    let project = state.storage.projects().get(user, project_id).await?; // 404, SC-008
    if job_ids.is_empty() {
        return Err(validation(
            "aucun job de tranchage fourni",
            serde_json::json!({}),
        ));
    }
    let mut latest: BTreeMap<i64, (SlicingJob, crate::domain::Gcode)> = BTreeMap::new();
    for raw in job_ids {
        let id = uuid::Uuid::parse_str(raw)
            .map(JobId)
            .map_err(|_| ApiError::not_found("Job"))?;
        let job = state.storage.jobs().get(user, id).await?; // 404 si autre compte
        if job.project_id != project_id {
            return Err(validation(
                "job d'un autre projet",
                serde_json::json!({ "job_id": raw }),
            ));
        }
        let (JobStatus::Succeeded, Some(gcode_id)) = (job.status, job.gcode_id) else {
            return Err(validation(
                "plateau non tranché (job non réussi)",
                serde_json::json!({ "job_id": raw, "plate_index": job.plate_index }),
            ));
        };
        let gcode = state.storage.gcodes().get(user, gcode_id).await?;
        let newer = latest
            .get(&job.plate_index)
            .is_none_or(|(prev, _)| job.created_at > prev.created_at);
        if newer {
            latest.insert(job.plate_index, (job, gcode));
        }
    }
    let newest = latest
        .values()
        .map(|(job, _)| job)
        .max_by_key(|job| job.created_at)
        .expect("au moins un job");
    let scene = newest
        .scene
        .clone()
        .unwrap_or_else(|| project.scene.clone());
    let model = crate::slicing::build_scene_model(state, user, project_id, &scene, None)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "modèle des plateaux tranchés");
            ApiError::internal()
        })?;
    let config = crate::slicing::config_from_snapshot(&newest.resolved_settings).map_err(|e| {
        tracing::error!(error = %e, "config figée des plateaux tranchés");
        ApiError::internal()
    })?;
    let (gcodes, stats) = latest
        .into_iter()
        .map(|(plate, (_, gcode))| {
            (
                PlateGcode {
                    plate_index: plate.max(0) as u32,
                    gcode_path: gcode.file_path.into(),
                },
                gcode.stats,
            )
        })
        .unzip();
    Ok(SlicedBundle {
        model,
        config,
        gcodes,
        stats,
        project_name: project.name,
    })
}

async fn gcode_3mf_bytes(bundle: &SlicedBundle, plate: Option<u32>) -> ApiResult<Vec<u8>> {
    if let Some(p) = plate {
        if !bundle.gcodes.iter().any(|g| g.plate_index == p) {
            return Err(validation(
                "plateau non tranché",
                serde_json::json!({ "plate_index": p }),
            ));
        }
    }
    crate::engine::gcode_3mf::write_gcode_3mf(&bundle.model, &bundle.config, &bundle.gcodes, plate)
        .await
        .map_err(|error| {
            if error.is_invalid_input() {
                return validation(&error.to_string(), serde_json::json!({}));
            }
            tracing::error!(error = %error, "écriture du .gcode.3mf dans engine-worker");
            ApiError::internal()
        })
}

fn parse_project_id(raw: &str) -> ApiResult<ProjectId> {
    uuid::Uuid::parse_str(raw)
        .map(ProjectId)
        .map_err(|_| ApiError::not_found("Projet"))
}

/// `POST /api/projects/{id}/export/sliced` — .gcode.3mf (pièce jointe).
pub async fn export_sliced(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(project_raw): Path<String>,
    Json(body): Json<SlicedRequest>,
) -> ApiResult<Response> {
    let project_id = parse_project_id(&project_raw)?;
    let bundle = sliced_bundle(&state, user.id, project_id, &body.job_ids).await?;
    let bytes = gcode_3mf_bytes(&bundle, body.plate_index).await?;
    let base = super::printers::safe_slug(&bundle.project_name);
    let filename = match body.plate_index {
        Some(p) => format!("{base}_plateau-{}.gcode.3mf", p + 1),
        None => format!("{base}.gcode.3mf"),
    };
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "model/3mf".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        bytes,
    )
        .into_response())
}

/// `use_3mf` du preset imprimante figé (option Orca, défaut false).
fn printer_uses_3mf(config: &engine::api::DynamicPrintConfig) -> bool {
    matches!(
        config.0.get("use_3mf"),
        Some(engine::api::ConfigValue::Bool(true))
    )
}

/// `POST /api/printers/{id}/upload-sliced` — envoi de tous les plateaux.
pub async fn upload_sliced(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(printer_raw): Path<String>,
    Json(body): Json<UploadSlicedRequest>,
) -> ApiResult<Json<UploadSlicedResponse>> {
    let printer_id = super::printers::parse_printer_id(&printer_raw)?;
    let project_id = parse_project_id(&body.project_id)?;
    let bundle = sliced_bundle(&state, user.id, project_id, &body.job_ids).await?;
    let use_3mf = printer_uses_3mf(&bundle.config);
    let file_count = if use_3mf { 1 } else { bundle.gcodes.len() };
    if body.start_now && file_count > 1 {
        return Err(validation(
            "démarrage immédiat impossible avec plusieurs fichiers",
            serde_json::json!({ "files": file_count }),
        ));
    }
    // Fichiers prêts avant toute connexion (aucun envoi partiel sur erreur locale).
    // Noms d'Orca (`send_gcode_legacy`) : nom d'export calculé par libslic3r
    // au tranchage (`filename_format`), `.gcode.3mf` si l'imprimante prend le
    // 3MF (extension remplacée, comme `default_output_file.replace_extension`).
    let mut gcode_files: Vec<(String, Vec<u8>)> = Vec::with_capacity(bundle.gcodes.len());
    for (plate, stats) in bundle.gcodes.iter().zip(&bundle.stats) {
        let bytes = state.files.read(&plate.gcode_path).await.map_err(|e| {
            tracing::error!(error = %e, "lecture du G-code (envoi de tous les plateaux)");
            ApiError::internal()
        })?;
        let name = crate::gcode_filename::for_gcode(stats, &bytes, &bundle.project_name);
        gcode_files.push((name, bytes));
    }
    let mut uploads: Vec<(String, Vec<u8>)> = Vec::with_capacity(file_count);
    if use_3mf {
        let bytes = gcode_3mf_bytes(&bundle, None).await?;
        let first = gcode_files
            .first()
            .map(|(n, _)| n.as_str())
            .unwrap_or("Untitled.gcode");
        let stem = first.strip_suffix(".gcode").unwrap_or(first);
        uploads.push((format!("{stem}.gcode.3mf"), bytes));
    } else {
        let mut names: Vec<String> = gcode_files.iter().map(|(n, _)| n.clone()).collect();
        crate::gcode_filename::dedupe(&mut names);
        uploads.extend(
            names
                .into_iter()
                .zip(gcode_files.into_iter().map(|(_, b)| b)),
        );
    }
    let printer = state.storage.printers().get(user.id, printer_id).await?; // 404, SC-008
    if let Some(host) = super::printers::host_client(&state, &printer).await? {
        let mut files = Vec::with_capacity(uploads.len());
        for (name, bytes) in uploads {
            let up = host.upload(&name, bytes, body.start_now, None).await?;
            files.push(PrinterUploadResponse {
                path: up.path,
                print_started: up.print_started,
            });
        }
        return Ok(Json(UploadSlicedResponse {
            format: if use_3mf { "gcode_3mf" } else { "gcode" },
            files,
        }));
    }
    let (_printer, client) = super::printers::load_client(&state, user.id, printer_id).await?;
    let mut files = Vec::with_capacity(uploads.len());
    for (name, bytes) in uploads {
        let result = client.upload(&name, bytes, body.start_now).await?;
        files.push(PrinterUploadResponse::from(result));
    }
    Ok(Json(UploadSlicedResponse {
        format: if use_3mf { "gcode_3mf" } else { "gcode" },
        files,
    }))
}
