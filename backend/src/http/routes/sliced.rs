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
    /// « Téléverser et mettre en file » (PrusaConnect `to_queue`), comme
    /// `UploadToPrinterRequest::queue`.
    #[serde(default)]
    pub queue: bool,
    /// Nom choisi dans la boîte d'envoi (lot 42, Orca PrintHostSendDialog) :
    /// nom du fichier unique (.gcode.3mf ou plateau unique), ou base commune
    /// des G-code par plateau (suffixes `_2`, `_3`… comme les noms proposés).
    #[serde(default)]
    pub filename: Option<String>,
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
    /// Réglages de chaque plateau (index = plateau), lot 44.
    plate_settings: Vec<engine::api::DynamicPrintConfig>,
    /// Vignette de chaque plateau (index = plateau), lot 49 (BBL-b) :
    /// Metadata/plate_<n>.png (+ _small) du .gcode.3mf.
    thumbnails: Vec<Vec<u8>>,
    /// Origine monde de chaque plateau (lot 43), lot 49 (BBL-b) : boîtes de
    /// plate_<n>.json relatives au plateau.
    plate_origins: Vec<(f64, f64)>,
    /// `model_id` vendeur de l'imprimante (BL-P001…), lot 49 (BBL-b) :
    /// slice_info.config.
    printer_model_id: String,
    /// `gcodes.stats` de chaque plateau (même ordre que `gcodes`) : nom Orca.
    stats: Vec<serde_json::Value>,
    /// G-code stocké de chaque plateau (même ordre que `gcodes`).
    gcode_ids: Vec<crate::domain::GcodeId>,
    project_name: String,
}

/// Jobs validés d'un envoi / export groupé : le plus récent par plateau.
pub(crate) struct SlicedJobs {
    project: crate::domain::Project,
    latest: BTreeMap<i64, (SlicingJob, crate::domain::Gcode)>,
}

impl SlicedJobs {
    /// Job le plus récent (config figée de l'envoi).
    fn newest(&self) -> &SlicingJob {
        self.latest
            .values()
            .map(|(job, _)| job)
            .max_by_key(|job| job.created_at)
            .expect("au moins un job")
    }

    /// `use_3mf` de l'imprimante figée dans le job le plus récent.
    pub(crate) fn uses_3mf(&self) -> ApiResult<bool> {
        let config = crate::slicing::config_from_snapshot(&self.newest().resolved_settings)
            .map_err(|e| {
                tracing::error!(error = %e, "config figée des plateaux tranchés");
                ApiError::internal()
            })?;
        Ok(printer_uses_3mf(&config))
    }

    /// Noms d'Orca proposés par plateau (`filename_format` évalué au
    /// tranchage) ; le G-code n'est relu que s'il n'a pas de nom calculé.
    pub(crate) async fn default_names(&self, state: &AppState) -> ApiResult<Vec<String>> {
        let mut names = Vec::with_capacity(self.latest.len());
        for (_, gcode) in self.latest.values() {
            let bytes = if gcode.stats.get("output_filename").is_some() {
                Vec::new()
            } else {
                read_gcode(state, &gcode.file_path).await?
            };
            names.push(crate::gcode_filename::for_gcode(
                &gcode.stats,
                &bytes,
                &self.project.name,
            ));
        }
        Ok(names)
    }
}

async fn read_gcode(state: &AppState, path: &str) -> ApiResult<Vec<u8>> {
    state
        .files
        .read(std::path::Path::new(path))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "lecture du G-code (envoi de tous les plateaux)");
            ApiError::internal()
        })
}

/// Noms réellement envoyés (« Send all ») à partir des noms proposés par
/// plateau : un seul `.gcode.3mf` (base du premier plateau) si l'imprimante
/// prend le 3MF, sinon un G-code par plateau dédoublonné (`_2`…). `custom` =
/// nom choisi dans la boîte d'envoi (validé, extension imposée).
pub(crate) fn planned_upload_names(
    defaults: &[String],
    use_3mf: bool,
    custom: Option<&str>,
) -> ApiResult<Vec<String>> {
    // Chaîne vide = nom proposé ; blancs seuls = 422 « nom requis ».
    let custom = custom.filter(|c| !c.is_empty());
    let checked = |raw: &str, ext: &str| {
        crate::gcode_filename::upload_name(raw, ext)
            .map_err(|why| super::printers::upload_name_error(&why, raw))
    };
    if use_3mf {
        let name = match custom {
            Some(raw) => checked(raw, ".gcode.3mf")?,
            None => {
                let first = defaults
                    .first()
                    .map(String::as_str)
                    .unwrap_or("Untitled.gcode");
                let stem = first.strip_suffix(".gcode").unwrap_or(first);
                format!("{stem}.gcode.3mf")
            }
        };
        return Ok(vec![name]);
    }
    let mut names: Vec<String> = match custom {
        Some(raw) => {
            let name = checked(raw, ".gcode")?;
            vec![name; defaults.len().max(1)]
        }
        None => defaults.to_vec(),
    };
    crate::gcode_filename::dedupe(&mut names);
    Ok(names)
}

pub(crate) async fn sliced_jobs(
    state: &AppState,
    user: UserId,
    project_id: ProjectId,
    job_ids: &[String],
) -> ApiResult<SlicedJobs> {
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
    Ok(SlicedJobs { project, latest })
}

impl SlicedJobs {
    /// Lot 47c (audit 45 H2D-a) : plateau(x) exporté(s) / envoyé(s) dont le
    /// G-code sort de la zone d'une buse → 409 (GUI d'Orca : Print / Export
    /// désactivés). `plate` = seul plateau exporté (0-based), sinon tous.
    fn ensure_printable(&self, plate: Option<u32>) -> ApiResult<()> {
        for (index, (_, gcode)) in &self.latest {
            if plate.is_some_and(|p| i64::from(p) != *index) {
                continue;
            }
            super::gcodes::ensure_gcode_printable(gcode)?;
        }
        Ok(())
    }
}

async fn sliced_bundle(
    state: &AppState,
    user: UserId,
    project_id: ProjectId,
    job_ids: &[String],
    plate: Option<u32>,
) -> ApiResult<SlicedBundle> {
    let jobs = sliced_jobs(state, user, project_id, job_ids).await?;
    jobs.ensure_printable(plate)?;
    let newest = jobs.newest();
    let scene = newest
        .scene
        .clone()
        .unwrap_or_else(|| jobs.project.scene.clone());
    let mut model = crate::slicing::build_scene_model(state, user, project_id, &scene, None)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "modèle des plateaux tranchés");
            ApiError::internal()
        })?;
    let mut config =
        crate::slicing::config_from_snapshot(&newest.resolved_settings).map_err(|e| {
            tracing::error!(error = %e, "config figée des plateaux tranchés");
            ApiError::internal()
        })?;
    // Lot 44 (X-d) : matrice de purge réellement purgée (n² × buses).
    super::flush::complete_flush_matrix(&mut config).await?;
    // Lot 49 (audit 47 BBL-b) : vignettes par plateau (capture du compte sinon
    // rendu serveur), avant le décalage monde — comme l'export de projet.
    let mut plates_3mf = super::export::scene_project_plates(&scene);
    super::export::attach_plate_thumbnails(
        state,
        user,
        project_id,
        &model,
        &config,
        &mut plates_3mf,
    )
    .await;
    let thumbnails: Vec<Vec<u8>> = plates_3mf.iter().map(|p| p.thumbnail_png.clone()).collect();
    let plate_origins =
        super::export::plate_world_origins(&config, super::export::scene_plate_count(&scene));
    let printer_model_id = vendor_printer_model_id(&config);
    // Positions monde d'Orca dans le 3MF (plateau k décalé), lot 43.
    super::export::to_orca_world(
        &mut model,
        &config,
        super::export::scene_plate_count(&scene),
    );
    let SlicedJobs { project, latest } = jobs;
    let gcode_ids = latest.values().map(|(_, gcode)| gcode.id).collect();
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
    // Lot 44 (B-1) : type de plaque et réglages de chaque plateau, comme les
    // PartPlate d'Orca dans model_settings.config du .gcode.3mf.
    let plate_settings = plates_3mf.into_iter().map(|plate| plate.settings).collect();
    Ok(SlicedBundle {
        model,
        config,
        gcodes,
        plate_settings,
        thumbnails,
        plate_origins,
        printer_model_id,
        stats,
        gcode_ids,
        project_name: project.name,
    })
}

/// Lot 49 (audit 47 BBL-b) : `model_id` vendeur du modèle d'imprimante actif,
/// comme `Preset::get_printer_type` d'Orca (`printer_model` du preset → fiche
/// machine_model du profil vendeur, ex. `BL-P001` pour l'X1 Carbon). Les
/// fiches sont des méta hors presets (import) : relues du répertoire des
/// profils vendeur, en cache (données statiques).
fn vendor_printer_model_id(config: &engine::api::DynamicPrintConfig) -> String {
    use std::collections::HashMap;
    use std::sync::OnceLock;
    let model = match config.get("printer_model") {
        Some(engine::api::ConfigValue::String(name)) => name.clone(),
        _ => return String::new(),
    };
    static CACHE: OnceLock<HashMap<String, String>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| {
        let mut map = HashMap::new();
        let dir = crate::http::state::default_profiles_dir();
        let Ok(vendors) = std::fs::read_dir(&dir) else {
            return map;
        };
        for vendor in vendors.flatten() {
            let Ok(files) = std::fs::read_dir(vendor.path().join("machine")) else {
                continue;
            };
            for file in files.flatten() {
                let path = file.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let Ok(bytes) = std::fs::read(&path) else {
                    continue;
                };
                let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
                    continue;
                };
                if json.get("type").and_then(|t| t.as_str()) != Some("machine_model") {
                    continue;
                }
                if let (Some(name), Some(id)) = (
                    json.get("name").and_then(|n| n.as_str()),
                    json.get("model_id").and_then(|i| i.as_str()),
                ) {
                    map.insert(name.to_string(), id.to_string());
                }
            }
        }
        map
    });
    cache.get(&model).cloned().unwrap_or_default()
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
    crate::engine::gcode_3mf::write_gcode_3mf(
        &bundle.model,
        &bundle.config,
        &bundle.gcodes,
        &bundle.plate_settings,
        plate,
        &bundle.thumbnails,
        &bundle.plate_origins,
        &bundle.printer_model_id,
    )
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
    let bundle =
        sliced_bundle(&state, user.id, project_id, &body.job_ids, body.plate_index).await?;
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
    let bundle = sliced_bundle(&state, user.id, project_id, &body.job_ids, None).await?;
    let use_3mf = printer_uses_3mf(&bundle.config);
    let file_count = if use_3mf { 1 } else { bundle.gcodes.len() };
    // Nom choisi validé avant tout travail (422 lisible).
    if let Some(raw) = body.filename.as_deref() {
        planned_upload_names(&[String::new()], use_3mf, Some(raw))?;
    }
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
    let defaults: Vec<String> = gcode_files.iter().map(|(n, _)| n.clone()).collect();
    let gcode_configs: Vec<_> = gcode_files
        .iter()
        .map(|(_, bytes)| crate::gcode_filename::config_block(&String::from_utf8_lossy(bytes)))
        .collect();
    let names = planned_upload_names(&defaults, use_3mf, body.filename.as_deref())?;
    // (nom, octets, G-code du plateau) ; le .gcode.3mf groupé n'a pas de G-code unique.
    let mut uploads: Vec<(String, Vec<u8>, Option<crate::domain::GcodeId>)> =
        Vec::with_capacity(file_count);
    if use_3mf {
        let bytes = gcode_3mf_bytes(&bundle, None).await?;
        uploads.push((names[0].clone(), bytes, None));
    } else {
        uploads.extend(
            names
                .into_iter()
                .zip(gcode_files.into_iter().map(|(_, b)| b))
                .zip(bundle.gcode_ids.iter().copied())
                .map(|((name, bytes), id)| (name, bytes, Some(id))),
        );
    }
    let printer = state.storage.printers().get(user.id, printer_id).await?; // 404, SC-008
                                                                            // Lot 46 (audit 44 DEV1-a) : G-code d'une autre imprimante refusé (409).
    for config in &gcode_configs {
        super::printers::ensure_printer_matches_gcode(&state, &printer, config).await?;
    }
    let action = super::printers::post_upload_action(&printer, body.start_now, body.queue)?;
    let target = super::printers::UploadTarget::connect(&state, user.id, &printer).await?;
    let mut files = Vec::with_capacity(uploads.len());
    for (name, bytes, gcode_id) in uploads {
        let up = target
            .send(
                &state,
                user.id,
                &printer,
                super::printers::UploadJob {
                    filename: name,
                    bytes,
                    action,
                    storage: None,
                    gcode_id,
                    project_id: Some(project_id),
                },
            )
            .await?;
        files.push(up);
    }
    Ok(Json(UploadSlicedResponse {
        format: if use_3mf { "gcode_3mf" } else { "gcode" },
        files,
    }))
}

#[cfg(test)]
mod upload_name_tests {
    use super::planned_upload_names;

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn default_names_are_kept_and_deduplicated() {
        let d = names(&["P_PLA_1h.gcode", "P_PLA_1h.gcode", "P_PLA_2h.gcode"]);
        assert_eq!(
            planned_upload_names(&d, false, None).unwrap(),
            names(&["P_PLA_1h.gcode", "P_PLA_1h_2.gcode", "P_PLA_2h.gcode"])
        );
        assert_eq!(
            planned_upload_names(&d, true, None).unwrap(),
            names(&["P_PLA_1h.gcode.3mf"])
        );
        // Chaîne vide = nom proposé.
        assert_eq!(
            planned_upload_names(&d[..1], false, Some("")).unwrap(),
            names(&["P_PLA_1h.gcode"])
        );
    }

    #[test]
    fn chosen_name_is_the_common_base_with_forced_extension() {
        let d = names(&["a.gcode", "b.gcode"]);
        assert_eq!(
            planned_upload_names(&d, false, Some("Lot")).unwrap(),
            names(&["Lot.gcode", "Lot_2.gcode"])
        );
        assert_eq!(
            planned_upload_names(&d, true, Some("Lot.gcode")).unwrap(),
            names(&["Lot.gcode.3mf"])
        );
        let err = planned_upload_names(&d, false, Some("a|b")).unwrap_err();
        assert_eq!(err.status, axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(err.body.details.unwrap()["reason"], "chars");
        assert!(planned_upload_names(&d, false, Some("  ")).is_err());
    }
}
