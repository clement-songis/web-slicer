//! Profil de hauteur de couche variable calculé par libslic3r
//! (`engine-worker layer-profile`, Slicing.cpp) — même transport que
//! `mesh_ops` : requête JSON bornée dans un répertoire temporaire, ligne `R`
//! = `LayerProfileResponse`. Aucun appel FFI dans ce process.

use std::path::Path;
use std::time::Duration;

use engine::api::layer_profile::{LayerProfileRequest, LayerProfileResponse};

use super::scene::{result_json, write_json_bounded, SceneWorkerError};
use super::worker::{run_worker_at, DEFAULT_TIMEOUT};

pub async fn run_layer_profile(req: &LayerProfileRequest) -> Result<Vec<f64>, SceneWorkerError> {
    run_layer_profile_at(&super::worker::worker_binary(), req, DEFAULT_TIMEOUT).await
}

pub async fn run_layer_profile_at(
    binary: &Path,
    req: &LayerProfileRequest,
    timeout: Duration,
) -> Result<Vec<f64>, SceneWorkerError> {
    req.op.validate().map_err(SceneWorkerError::InvalidInput)?;
    if req.model.objects.len() != 1 {
        return Err(SceneWorkerError::InvalidInput(
            "un seul objet attendu".into(),
        ));
    }
    let dir = crate::workdir::tempdir()?;
    let request_path = dir.path().join("layer-profile.json");
    write_json_bounded(&request_path, req)?;
    let stdout = run_worker_at(
        binary,
        &["layer-profile", &request_path.to_string_lossy()],
        timeout,
    )
    .await?;
    let response: LayerProfileResponse = serde_json::from_str(result_json(&stdout)?)
        .map_err(|e| SceneWorkerError::Protocol(e.to_string()))?;
    if !response.profile.len().is_multiple_of(2) || response.profile.iter().any(|v| !v.is_finite())
    {
        return Err(SceneWorkerError::Protocol("profil incohérent".into()));
    }
    Ok(response.profile)
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::api::layer_profile::LayerProfileOp;

    #[tokio::test]
    async fn invalid_requests_are_refused_without_spawning() {
        let missing = Path::new("/nonexistent/engine-worker");
        let req = LayerProfileRequest {
            model: engine::api::Model::default(),
            config: engine::api::DynamicPrintConfig::new(),
            op: LayerProfileOp::Adaptive { quality: 2.0 },
        };
        let err = run_layer_profile_at(missing, &req, Duration::from_secs(1))
            .await
            .unwrap_err();
        assert!(err.is_invalid_input(), "{err}");
        let req = LayerProfileRequest {
            op: LayerProfileOp::Adaptive { quality: 0.5 },
            ..req
        };
        let err = run_layer_profile_at(missing, &req, Duration::from_secs(1))
            .await
            .unwrap_err();
        assert!(err.is_invalid_input(), "objet manquant : {err}");
    }
}
