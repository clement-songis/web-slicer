//! Opérations de scène réelles exécutées par `engine-worker`.
//!
//! Le backend ne lie et n'appelle jamais la FFI : il sérialise une requête JSON
//! bornée dans un répertoire temporaire, lance un process isolé, puis décode la
//! ligne `R` du protocole. La réparation transporte le maillage en WSMh dans un
//! fichier borné plutôt qu'en JSON ou sur stdout (pollué par Boost.Log).
//!
//! Limites exactes du contrat actuel :
//! - requêtes JSON ≤ 512 Mio, ligne résultat ≤ 16 Mio, WSMh réparé ≤ 512 Mio,
//!   timeout de 120 s par opération ;
//! - arrange ne crée aucun plateau supplémentaire et ne tient pas compte de
//!   `max_height` ni de `use_machine_clearance` (non exposés par `arrange_raw`) ;
//!   le nester peut pivoter les instances et le bridge C++ peut introduire une
//!   faible approximation numérique des scales non uniformes ; les exclusions
//!   sont garanties dans le cas nominal, mais pas pour cette combinaison de
//!   transformations non uniformes/rotation (limite du bridge existant) ;
//! - repair = admesh (soudure, trous, normales, facettes dégénérées), sans
//!   garantie sur les auto-intersections ; le fichier source reste inchangé,
//!   seuls le WSMh et `repair_report` sont remplacés ;
//! - simplify = `its_quadric_edge_collapse` de libslic3r (outil « Simplifier »
//!   d'Orca), ratio de décimation ou niveau de détail ; même transport WSMh
//!   borné que repair, fichier source inchangé, seul le WSMh est remplacé.

use std::path::{Path, PathBuf};
use std::time::Duration;

use engine::api::mesh::{SimplifyParams, SimplifyReport};
use engine::api::{
    ArrangeParams, BuildVolume, DisplayMesh, Model, RepairedMeshErrors, TriangleMesh,
};
use serde::{Deserialize, Serialize};

use super::worker::{run_worker_at, WorkerError, DEFAULT_TIMEOUT};

/// Limite commune aux requêtes JSON de scène (512 Mio, au-delà de la limite
/// d'upload pour laisser la place à l'indexation et aux matrices).
pub const MAX_SCENE_REQUEST_BYTES: u64 = 512 * 1024 * 1024;
/// Une ligne `R` ne contient que des matrices/rapports, jamais un maillage.
pub const MAX_SCENE_RESULT_BYTES: usize = 16 * 1024 * 1024;
/// Limite du WSMh réparé relu par le backend (512 Mio).
pub const MAX_REPAIRED_MESH_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrangeWorkerRequest {
    pub model: Model,
    pub bed: BuildVolume,
    pub params: ArrangeParams,
    /// Identifiants scène, dans l'ordre objets puis instances du modèle.
    pub instance_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrangedInstance {
    /// Ordre d'impression séquentielle (1-based par plateau), si séquentiel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub print_order: Option<u32>,
    pub id: String,
    pub plate_index: u32,
    /// Matrice monde complète, colonne-major : rotation/scale conservés.
    pub matrix: [f64; 16],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrangeWorkerResponse {
    pub placements: Vec<ArrangedInstance>,
}

#[derive(Debug, Clone, Serialize)]
struct RepairWorkerRequest {
    mesh: TriangleMesh,
    output_path: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
struct RepairWorkerResponse {
    mesh_path: PathBuf,
    repairs: RepairedMeshErrors,
}

#[derive(Debug, Clone)]
pub struct RepairWorkerOutput {
    pub mesh: TriangleMesh,
    pub display_bytes: Vec<u8>,
    pub repairs: RepairedMeshErrors,
}

#[derive(Debug, Clone, Serialize)]
struct SimplifyWorkerRequest {
    mesh: TriangleMesh,
    params: SimplifyParams,
    output_path: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
struct SimplifyWorkerResponse {
    mesh_path: PathBuf,
    report: SimplifyReport,
}

#[derive(Debug, Clone)]
pub struct SimplifyWorkerOutput {
    pub mesh: TriangleMesh,
    pub display_bytes: Vec<u8>,
    pub report: SimplifyReport,
}

#[derive(Debug, thiserror::Error)]
pub enum SceneWorkerError {
    #[error("requête de scène trop volumineuse")]
    RequestTooLarge,
    #[error("résultat de scène trop volumineux")]
    ResultTooLarge,
    #[error("protocole worker invalide : {0}")]
    Protocol(String),
    #[error("maillage produit trop volumineux")]
    MeshTooLarge,
    #[error("maillage produit invalide : {0}")]
    InvalidMesh(String),
    #[error("entrée refusée par le moteur : {0}")]
    InvalidInput(String),
    #[error("E/S de la requête worker : {0}")]
    Io(#[from] std::io::Error),
    #[error("exécution du worker : {0}")]
    Worker(WorkerError),
}

impl SceneWorkerError {
    /// Vrai pour une erreur utilisateur (422), faux pour une faute d'infra (500).
    pub fn is_invalid_input(&self) -> bool {
        matches!(self, Self::InvalidInput(_))
    }
}

/// Arrangement via le binaire résolu par l'environnement de production.
pub async fn arrange(
    request: &ArrangeWorkerRequest,
) -> Result<ArrangeWorkerResponse, SceneWorkerError> {
    arrange_at(&super::worker::worker_binary(), request, DEFAULT_TIMEOUT).await
}

/// Variante à binaire explicite pour les tests de protocole.
pub async fn arrange_at(
    binary: &Path,
    request: &ArrangeWorkerRequest,
    timeout: Duration,
) -> Result<ArrangeWorkerResponse, SceneWorkerError> {
    let dir = crate::workdir::tempdir()?;
    let request_path = dir.path().join("arrange.json");
    write_json_bounded(&request_path, request)?;
    let stdout = run_worker_at(
        binary,
        &["arrange", &request_path.to_string_lossy()],
        timeout,
    )
    .await?;
    let json = result_json(&stdout)?;
    serde_json::from_str(json).map_err(|e| SceneWorkerError::Protocol(e.to_string()))
}

/// Réparation via le binaire résolu par l'environnement de production.
pub async fn repair(mesh: &TriangleMesh) -> Result<RepairWorkerOutput, SceneWorkerError> {
    repair_at(&super::worker::worker_binary(), mesh, DEFAULT_TIMEOUT).await
}

/// Variante à binaire explicite pour les tests de protocole. Le WSMh est relu
/// avant destruction du répertoire temporaire ; aucun chemin n'est exposé à
/// l'appelant HTTP.
pub async fn repair_at(
    binary: &Path,
    mesh: &TriangleMesh,
    timeout: Duration,
) -> Result<RepairWorkerOutput, SceneWorkerError> {
    let dir = crate::workdir::tempdir()?;
    let request_path = dir.path().join("repair.json");
    let output_path = dir.path().join("repaired.wsm");
    let request = RepairWorkerRequest {
        mesh: mesh.clone(),
        output_path,
    };
    write_json_bounded(&request_path, &request)?;
    let stdout = run_worker_at(
        binary,
        &["repair", &request_path.to_string_lossy()],
        timeout,
    )
    .await?;
    let json = result_json(&stdout)?;
    let response: RepairWorkerResponse =
        serde_json::from_str(json).map_err(|e| SceneWorkerError::Protocol(e.to_string()))?;
    let (mesh, display_bytes) = read_worker_mesh(dir.path(), &response.mesh_path).await?;
    Ok(RepairWorkerOutput {
        mesh,
        display_bytes,
        repairs: response.repairs,
    })
}

/// Simplification quadrique via le binaire résolu par l'environnement.
pub async fn simplify(
    mesh: &TriangleMesh,
    params: &SimplifyParams,
) -> Result<SimplifyWorkerOutput, SceneWorkerError> {
    simplify_at(
        &super::worker::worker_binary(),
        mesh,
        params,
        DEFAULT_TIMEOUT,
    )
    .await
}

/// Variante à binaire explicite. Le paramètre est validé ici (logique pure du
/// crate moteur, sans FFI) pour refuser une entrée invalide sans lancer de
/// process ; le worker le revalide de toute façon. Le rapport est recoupé avec
/// le WSMh effectivement relu.
pub async fn simplify_at(
    binary: &Path,
    mesh: &TriangleMesh,
    params: &SimplifyParams,
    timeout: Duration,
) -> Result<SimplifyWorkerOutput, SceneWorkerError> {
    params
        .target(mesh.triangle_count())
        .map_err(|e| SceneWorkerError::InvalidInput(e.to_string()))?;
    let dir = crate::workdir::tempdir()?;
    let request_path = dir.path().join("simplify.json");
    let output_path = dir.path().join("simplified.wsm");
    let request = SimplifyWorkerRequest {
        mesh: mesh.clone(),
        params: *params,
        output_path,
    };
    write_json_bounded(&request_path, &request)?;
    let stdout = run_worker_at(
        binary,
        &["simplify", &request_path.to_string_lossy()],
        timeout,
    )
    .await?;
    let json = result_json(&stdout)?;
    let response: SimplifyWorkerResponse =
        serde_json::from_str(json).map_err(|e| SceneWorkerError::Protocol(e.to_string()))?;
    let (simplified, display_bytes) = read_worker_mesh(dir.path(), &response.mesh_path).await?;
    if simplified.is_empty()
        || response.report.triangles_after as usize != simplified.triangle_count()
        || response.report.vertices_after as usize != simplified.vertices.len()
    {
        return Err(SceneWorkerError::Protocol(
            "rapport de simplification incohérent avec le maillage".into(),
        ));
    }
    Ok(SimplifyWorkerOutput {
        mesh: simplified,
        display_bytes,
        report: response.report,
    })
}

/// Relit et décode le WSMh produit par le worker. Le chemin vient du worker,
/// pas du client ; on impose néanmoins qu'il reste dans le répertoire de cette
/// invocation avant de lire le binaire.
pub(super) async fn read_worker_mesh(
    dir: &Path,
    mesh_path: &Path,
) -> Result<(TriangleMesh, Vec<u8>), SceneWorkerError> {
    if mesh_path.parent() != Some(dir) {
        return Err(SceneWorkerError::Protocol(
            "chemin du maillage hors du répertoire temporaire".into(),
        ));
    }
    let display_bytes = read_bounded(mesh_path, MAX_REPAIRED_MESH_BYTES).await?;
    let display = DisplayMesh::decode(&display_bytes)
        .map_err(|e| SceneWorkerError::InvalidMesh(e.to_string()))?;
    let mesh = display_to_triangle_mesh(display)?;
    Ok((mesh, display_bytes))
}

fn display_to_triangle_mesh(display: DisplayMesh) -> Result<TriangleMesh, SceneWorkerError> {
    if !display.indices.len().is_multiple_of(3) {
        return Err(SceneWorkerError::InvalidMesh(
            "nombre d'indices non multiple de trois".into(),
        ));
    }
    let vertices: Vec<[f32; 3]> = display
        .positions
        .chunks_exact(3)
        .map(|c| [c[0], c[1], c[2]])
        .collect();
    if display
        .indices
        .iter()
        .any(|&index| index as usize >= vertices.len())
    {
        return Err(SceneWorkerError::InvalidMesh(
            "indice de sommet hors bornes".into(),
        ));
    }
    Ok(TriangleMesh {
        vertices,
        indices: display
            .indices
            .chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect(),
    })
}

pub(super) fn write_json_bounded<T: Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), SceneWorkerError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|e| SceneWorkerError::Protocol(format!("sérialisation JSON : {e}")))?;
    if bytes.len() as u64 > MAX_SCENE_REQUEST_BYTES {
        return Err(SceneWorkerError::RequestTooLarge);
    }
    std::fs::write(path, bytes)?;
    Ok(())
}

async fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, SceneWorkerError> {
    let metadata = tokio::fs::metadata(path).await?;
    if metadata.len() > limit {
        return Err(SceneWorkerError::MeshTooLarge);
    }
    let bytes = tokio::fs::read(path).await?;
    if bytes.len() as u64 > limit {
        return Err(SceneWorkerError::MeshTooLarge);
    }
    Ok(bytes)
}

/// Extrait l'unique ligne `R <json>`, en ignorant les logs libslic3r. Les
/// erreurs `E` remontées dans un `WorkerError::Failed` sont converties par
/// [`map_worker_error`].
pub(super) fn result_json(stdout: &[u8]) -> Result<&str, SceneWorkerError> {
    if stdout.len() > MAX_SCENE_RESULT_BYTES {
        return Err(SceneWorkerError::ResultTooLarge);
    }
    let text = std::str::from_utf8(stdout)
        .map_err(|_| SceneWorkerError::Protocol("résultat non UTF-8".into()))?;
    let mut results = text.lines().filter_map(|line| line.strip_prefix("R "));
    let result = results
        .next()
        .ok_or_else(|| SceneWorkerError::Protocol("ligne R absente".into()))?;
    if result.len() > MAX_SCENE_RESULT_BYTES || results.next().is_some() {
        return Err(SceneWorkerError::Protocol(
            "résultat absent, excessif ou multiple".into(),
        ));
    }
    Ok(result)
}

/// Traduit le détail structuré `E <code> <message>` conservé par le runner.
pub fn map_worker_error(error: WorkerError) -> SceneWorkerError {
    if let WorkerError::Failed { stdout, .. } = &error {
        let text = String::from_utf8_lossy(stdout);
        if let Some(line) = text.lines().find_map(|line| line.strip_prefix("E ")) {
            let (code, message) = line.split_once(' ').unwrap_or((line, ""));
            if matches!(
                code,
                "invalid_model" | "invalid_config" | "out_of_build_volume"
            ) {
                return SceneWorkerError::InvalidInput(message.to_string());
            }
        }
    }
    SceneWorkerError::Worker(error)
}

impl From<WorkerError> for SceneWorkerError {
    fn from(error: WorkerError) -> Self {
        map_worker_error(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_parser_ignores_logs_but_rejects_multiple_results() {
        assert_eq!(
            result_json(b"log\nR {\"ok\":true}\n").unwrap(),
            "{\"ok\":true}"
        );
        assert!(result_json(b"R {}\nR {}\n").is_err());
        assert!(result_json(b"E invalid_model non\n").is_err());
    }

    #[test]
    fn display_decoder_rejects_out_of_bounds_indices() {
        let display = DisplayMesh {
            positions: vec![0.0; 9],
            normals: vec![0.0; 9],
            indices: vec![0, 1, 3],
        };
        assert!(display_to_triangle_mesh(display).is_err());
    }

    #[tokio::test]
    async fn simplify_rejects_invalid_params_without_spawning() {
        let mesh = TriangleMesh {
            vertices: vec![[0.0; 3]; 3],
            indices: vec![[0, 1, 2]; 10],
        };
        // Binaire inexistant : seul le refus préalable peut répondre.
        let missing = Path::new("/nonexistent/engine-worker");
        for params in [
            SimplifyParams::Ratio {
                decimate_ratio: 0.0,
            },
            SimplifyParams::Ratio {
                decimate_ratio: 100.0,
            },
            SimplifyParams::Detail { level: 9 },
        ] {
            let err = simplify_at(missing, &mesh, &params, Duration::from_secs(1))
                .await
                .unwrap_err();
            assert!(err.is_invalid_input(), "{err}");
        }
    }

    #[test]
    fn structured_engine_error_is_an_input_error() {
        let error = WorkerError::Failed {
            code: 1,
            stdout: b"E out_of_build_volume objet trop grand\n".to_vec(),
            stderr: String::new(),
        };
        assert!(matches!(
            map_worker_error(error),
            SceneWorkerError::InvalidInput(message) if message == "objet trop grand"
        ));
    }
}
