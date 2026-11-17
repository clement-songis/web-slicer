//! Projet 3MF avec ses plateaux écrit par libslic3r (« Save project » d'Orca :
//! `store_bbs_3mf`, SplitModel | ShareMesh) via `engine-worker export-project-3mf`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use engine::api::{DynamicPrintConfig, Model};
use engine::threemf::ProjectPlate;

use super::scene::{result_json, write_json_bounded, SceneWorkerError};
use super::worker::{run_worker_at, DEFAULT_TIMEOUT};

#[derive(serde::Serialize)]
struct Request<'a> {
    model: &'a Model,
    config: &'a DynamicPrintConfig,
    plates: &'a [ProjectPlate],
    #[serde(skip_serializing_if = "Option::is_none")]
    aux_dir: Option<&'a Path>,
    output_path: PathBuf,
}

#[derive(serde::Deserialize)]
struct Response {
    path: PathBuf,
}

/// Octets du projet 3MF.
pub async fn write_project_3mf(
    model: &Model,
    config: &DynamicPrintConfig,
    plates: &[ProjectPlate],
) -> Result<Vec<u8>, SceneWorkerError> {
    write_project_3mf_at(
        &super::worker::worker_binary(),
        model,
        config,
        plates,
        None,
        DEFAULT_TIMEOUT,
    )
    .await
}

/// Avec les fichiers auxiliaires du dossier `aux_dir` (Auxiliaries/ du 3MF).
pub async fn write_project_3mf_with_aux(
    model: &Model,
    config: &DynamicPrintConfig,
    plates: &[ProjectPlate],
    aux_dir: &Path,
) -> Result<Vec<u8>, SceneWorkerError> {
    write_project_3mf_at(
        &super::worker::worker_binary(),
        model,
        config,
        plates,
        Some(aux_dir),
        DEFAULT_TIMEOUT,
    )
    .await
}

/// Fichiers auxiliaires d'un projet 3MF extraits par libslic3r dans `out_dir`.
pub async fn read_project_auxiliaries(
    bytes: &[u8],
    out_dir: &Path,
) -> Result<Vec<String>, SceneWorkerError> {
    let dir = crate::workdir::tempdir()?;
    let input = dir.path().join("project.3mf");
    tokio::fs::write(&input, bytes).await?;
    let stdout = run_worker_at(
        &super::worker::worker_binary(),
        &[
            "read-project-aux",
            &input.to_string_lossy(),
            &out_dir.to_string_lossy(),
        ],
        DEFAULT_TIMEOUT,
    )
    .await
    .map_err(super::scene::map_worker_error)?;
    #[derive(serde::Deserialize)]
    struct Files {
        files: Vec<String>,
    }
    let files: Files = serde_json::from_str(result_json(&stdout)?)
        .map_err(|e| SceneWorkerError::Protocol(e.to_string()))?;
    Ok(files.files)
}

pub async fn write_project_3mf_at(
    binary: &Path,
    model: &Model,
    config: &DynamicPrintConfig,
    plates: &[ProjectPlate],
    aux_dir: Option<&Path>,
    timeout: Duration,
) -> Result<Vec<u8>, SceneWorkerError> {
    let dir = crate::workdir::tempdir()?;
    let output_path = dir.path().join("project.3mf");
    let request_path = dir.path().join("export-project-3mf.json");
    write_json_bounded(
        &request_path,
        &Request {
            model,
            config,
            plates,
            aux_dir,
            output_path: output_path.clone(),
        },
    )?;
    let stdout = run_worker_at(
        binary,
        &["export-project-3mf", &request_path.to_string_lossy()],
        timeout,
    )
    .await
    .map_err(super::scene::map_worker_error)?;
    let response: Response = serde_json::from_str(result_json(&stdout)?)
        .map_err(|e| SceneWorkerError::Protocol(e.to_string()))?;
    if response.path != output_path {
        return Err(SceneWorkerError::Protocol(
            "fichier hors du répertoire de travail".into(),
        ));
    }
    Ok(tokio::fs::read(&output_path).await?)
}
