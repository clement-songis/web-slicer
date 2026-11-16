//! Maillages d'export (STL / Draco) calculés par libslic3r comme
//! `Plater::export_stl` d'Orca : volumes négatifs soustraits par booléens,
//! instances fusionnées (`engine-worker export-meshes`, aucune FFI ici).

use std::path::Path;
use std::time::Duration;

use engine::api::{Model, TriangleMesh};

use super::scene::{result_json, write_json_bounded, SceneWorkerError};
use super::worker::{run_worker_at, DEFAULT_TIMEOUT};

#[derive(serde::Serialize)]
struct Request<'a> {
    model: &'a Model,
    output_dir: &'a Path,
}

#[derive(serde::Deserialize)]
struct Response {
    files: Vec<std::path::PathBuf>,
}

/// Un maillage monde par objet du modèle (même ordre).
pub async fn export_meshes(model: &Model) -> Result<Vec<TriangleMesh>, SceneWorkerError> {
    export_meshes_at(&super::worker::worker_binary(), model, DEFAULT_TIMEOUT).await
}

pub async fn export_meshes_at(
    binary: &Path,
    model: &Model,
    timeout: Duration,
) -> Result<Vec<TriangleMesh>, SceneWorkerError> {
    if model.objects.is_empty() {
        return Err(SceneWorkerError::InvalidInput(
            "aucun objet à exporter".into(),
        ));
    }
    let dir = crate::workdir::tempdir()?;
    let request_path = dir.path().join("export-meshes.json");
    write_json_bounded(
        &request_path,
        &Request {
            model,
            output_dir: dir.path(),
        },
    )?;
    let stdout = run_worker_at(
        binary,
        &["export-meshes", &request_path.to_string_lossy()],
        timeout,
    )
    .await
    .map_err(super::scene::map_worker_error)?;
    let response: Response = serde_json::from_str(result_json(&stdout)?)
        .map_err(|e| SceneWorkerError::Protocol(e.to_string()))?;
    let mut out = Vec::with_capacity(response.files.len());
    for file in response.files {
        if !file.starts_with(dir.path()) {
            return Err(SceneWorkerError::Protocol(
                "fichier hors du répertoire de travail".into(),
            ));
        }
        let bytes = tokio::fs::read(&file).await?;
        let display = engine::api::DisplayMesh::decode(&bytes)
            .map_err(|e| SceneWorkerError::Protocol(e.to_string()))?;
        out.push(TriangleMesh {
            vertices: display
                .positions
                .chunks_exact(3)
                .map(|c| [c[0], c[1], c[2]])
                .collect(),
            indices: display
                .indices
                .chunks_exact(3)
                .map(|c| [c[0], c[1], c[2]])
                .collect(),
        });
    }
    Ok(out)
}
