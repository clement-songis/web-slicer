//! Découpe de maillage (split / cut / boolean) exécutée par `engine-worker`
//! (sous-commande `mesh-op`, bridge `split.cpp`).
//!
//! Même transport que repair/simplify (`super::scene`) : requête JSON bornée
//! dans un répertoire temporaire, une pièce = un WSMh relu dans ce même
//! répertoire, ligne `R` = `MeshOpResponse`. Aucun appel FFI dans ce process.

use std::path::Path;
use std::time::Duration;

use engine::api::split::{MeshOp, MeshOpRequest, MeshOpResponse};
use engine::api::TriangleMesh;

use super::scene::{read_worker_mesh, result_json, write_json_bounded, SceneWorkerError};
use super::worker::{run_worker_at, DEFAULT_TIMEOUT};

/// Pièce relue après l'opération.
#[derive(Debug, Clone)]
pub struct MeshOpOutputPart {
    pub label: String,
    pub mesh: TriangleMesh,
    /// WSMh tel qu'écrit par le worker (servi tel quel par `/mesh`).
    pub display_bytes: Vec<u8>,
}

/// Opération via le binaire résolu par l'environnement de production.
pub async fn run_mesh_op(
    op: &MeshOp,
    meshes: &[TriangleMesh],
) -> Result<Vec<MeshOpOutputPart>, SceneWorkerError> {
    run_mesh_op_at(&super::worker::worker_binary(), op, meshes, DEFAULT_TIMEOUT).await
}

/// Variante à binaire explicite. Les paramètres sont validés ici (logique
/// pure du crate moteur) : une entrée invalide est refusée sans lancer de
/// process. Les comptes du rapport sont recoupés avec les WSMh relus.
pub async fn run_mesh_op_at(
    binary: &Path,
    op: &MeshOp,
    meshes: &[TriangleMesh],
    timeout: Duration,
) -> Result<Vec<MeshOpOutputPart>, SceneWorkerError> {
    op.validate(meshes)
        .map_err(|e| SceneWorkerError::InvalidInput(e.to_string()))?;
    let dir = crate::workdir::tempdir()?;
    let request_path = dir.path().join("mesh-op.json");
    let request = MeshOpRequest {
        op: op.clone(),
        meshes: meshes.to_vec(),
        output_dir: dir.path().to_path_buf(),
    };
    write_json_bounded(&request_path, &request)?;
    let stdout = run_worker_at(
        binary,
        &["mesh-op", &request_path.to_string_lossy()],
        timeout,
    )
    .await?;
    let response: MeshOpResponse = serde_json::from_str(result_json(&stdout)?)
        .map_err(|e| SceneWorkerError::Protocol(e.to_string()))?;
    if response.parts.is_empty() {
        return Err(SceneWorkerError::Protocol("aucune pièce produite".into()));
    }
    let mut parts = Vec::with_capacity(response.parts.len());
    for part in response.parts {
        let (mesh, display_bytes) = read_worker_mesh(dir.path(), &part.mesh_path).await?;
        if mesh.is_empty()
            || part.triangles as usize != mesh.triangle_count()
            || part.vertices as usize != mesh.vertices.len()
        {
            return Err(SceneWorkerError::Protocol(
                "rapport de découpe incohérent avec le maillage".into(),
            ));
        }
        parts.push(MeshOpOutputPart {
            label: part.label,
            mesh,
            display_bytes,
        });
    }
    Ok(parts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::api::split::{BooleanOp, CutPlane};

    #[tokio::test]
    async fn invalid_requests_are_refused_without_spawning() {
        let mesh = TriangleMesh {
            vertices: vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            indices: vec![[0, 1, 2]],
        };
        let missing = Path::new("/nonexistent/engine-worker");
        let cases = [
            (MeshOp::Split, vec![]),
            (
                MeshOp::Cut {
                    plane: CutPlane::horizontal(0.5),
                    keep_upper: false,
                    keep_lower: false,
                },
                vec![mesh.clone()],
            ),
            (
                MeshOp::Boolean {
                    op: BooleanOp::Union,
                },
                vec![mesh.clone()],
            ),
        ];
        for (op, meshes) in cases {
            let err = run_mesh_op_at(missing, &op, &meshes, Duration::from_secs(1))
                .await
                .unwrap_err();
            assert!(err.is_invalid_input(), "{err}");
        }
    }
}
