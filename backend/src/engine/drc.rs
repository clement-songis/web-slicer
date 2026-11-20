//! Export Draco (menu Fichier › Export all objects as one DRC / as DRCs,
//! menu objet › Export as one DRC / as DRCs) exécuté par `engine-worker`
//! (sous-commande `export-drc`, `Slic3r::store_drc` d'Orca).
//!
//! Même transport que `mesh-op` : requête JSON bornée dans un répertoire
//! temporaire, un fichier `.drc` par maillage relu dans ce même répertoire.

use std::path::{Path, PathBuf};
use std::time::Duration;

use engine::api::TriangleMesh;
use serde::{Deserialize, Serialize};

use super::scene::{map_worker_error, result_json, write_json_bounded, SceneWorkerError};
use super::worker::{run_worker_at, DEFAULT_TIMEOUT};

/// Taille maximale d'un fichier Draco relu (512 Mio, comme les WSMh).
const MAX_DRC_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Serialize)]
struct DrcExportRequest<'a> {
    meshes: &'a [TriangleMesh],
    output_dir: PathBuf,
    /// Quantification Draco (`drc_bits` d'Orca, 0 = aucune).
    bits: i32,
}

#[derive(Debug, Deserialize)]
struct DrcExportResponse {
    files: Vec<PathBuf>,
}

/// Quantification Draco d'Orca : 0 = aucune (défaut `DRC_BITS_DEFAULT`).
pub const DRC_BITS_DEFAULT: i32 = 0;
/// Bornes de la préférence « Quality level for Draco export » (`DRC_BITS_MIN`/`MAX`).
pub const DRC_BITS_MIN: i32 = 8;
pub const DRC_BITS_MAX: i32 = 30;

/// Normalise une quantification demandée comme `PreferencesDialog::create_item_draco`
/// d'Orca : au-delà de 30 → 30, 1 à 7 → 8, 0 conservé (aucune quantification).
/// `None` si la valeur est négative (le champ d'Orca n'accepte que des chiffres).
pub fn normalize_drc_bits(bits: i64) -> Option<i32> {
    match bits {
        b if b < 0 => None,
        0 => Some(0),
        b if b > i64::from(DRC_BITS_MAX) => Some(DRC_BITS_MAX),
        b if b < i64::from(DRC_BITS_MIN) => Some(DRC_BITS_MIN),
        b => Some(b as i32),
    }
}

/// Encode chaque maillage en Draco via le binaire de production.
pub async fn export_drc(
    meshes: &[TriangleMesh],
    bits: i32,
) -> Result<Vec<Vec<u8>>, SceneWorkerError> {
    export_drc_at(
        &super::worker::worker_binary(),
        meshes,
        bits,
        DEFAULT_TIMEOUT,
    )
    .await
}

/// Variante à binaire explicite. Un maillage vide est refusé sans lancer de
/// process ; chaque fichier produit doit être dans le répertoire de travail.
pub async fn export_drc_at(
    binary: &Path,
    meshes: &[TriangleMesh],
    bits: i32,
    timeout: Duration,
) -> Result<Vec<Vec<u8>>, SceneWorkerError> {
    if meshes.is_empty() || meshes.iter().any(TriangleMesh::is_empty) {
        return Err(SceneWorkerError::InvalidInput(
            "aucun maillage à exporter".into(),
        ));
    }
    if bits != 0 && !(DRC_BITS_MIN..=DRC_BITS_MAX).contains(&bits) {
        return Err(SceneWorkerError::InvalidInput(format!(
            "quantification Draco hors bornes : {bits}"
        )));
    }
    let dir = crate::workdir::tempdir()?;
    let request_path = dir.path().join("export-drc.json");
    write_json_bounded(
        &request_path,
        &DrcExportRequest {
            meshes,
            output_dir: dir.path().to_path_buf(),
            bits,
        },
    )?;
    let stdout = run_worker_at(
        binary,
        &["export-drc", &request_path.to_string_lossy()],
        timeout,
    )
    .await
    .map_err(map_worker_error)?;
    let response: DrcExportResponse = serde_json::from_str(result_json(&stdout)?)
        .map_err(|e| SceneWorkerError::Protocol(e.to_string()))?;
    if response.files.len() != meshes.len() {
        return Err(SceneWorkerError::Protocol(
            "nombre de fichiers Draco incohérent".into(),
        ));
    }
    let mut out = Vec::with_capacity(response.files.len());
    for file in response.files {
        if file.parent() != Some(dir.path()) {
            return Err(SceneWorkerError::Protocol(
                "fichier Draco hors du répertoire de travail".into(),
            ));
        }
        let len = tokio::fs::metadata(&file).await?.len();
        if len > MAX_DRC_BYTES {
            return Err(SceneWorkerError::MeshTooLarge);
        }
        let bytes = tokio::fs::read(&file).await?;
        if !bytes.starts_with(b"DRACO") {
            return Err(SceneWorkerError::InvalidMesh("en-tête DRACO absent".into()));
        }
        out.push(bytes);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn empty_exports_are_refused_without_spawning() {
        let missing = Path::new("/nonexistent/engine-worker");
        for meshes in [vec![], vec![TriangleMesh::default()]] {
            let err = export_drc_at(missing, &meshes, 0, Duration::from_secs(1))
                .await
                .unwrap_err();
            assert!(err.is_invalid_input(), "{err}");
        }
    }

    #[tokio::test]
    async fn out_of_range_bits_are_refused_without_spawning() {
        let missing = Path::new("/nonexistent/engine-worker");
        let mesh = TriangleMesh {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            indices: vec![[0, 1, 2]],
        };
        for bits in [-1, 1, 7, 31] {
            let err = export_drc_at(
                missing,
                std::slice::from_ref(&mesh),
                bits,
                Duration::from_secs(1),
            )
            .await
            .unwrap_err();
            assert!(err.is_invalid_input(), "{bits}: {err}");
        }
    }

    #[test]
    fn bits_are_normalized_like_orca_preferences() {
        // PreferencesDialog::create_item_draco : > 30 → 30, 1..7 → 8, 0 gardé.
        assert_eq!(normalize_drc_bits(0), Some(0));
        assert_eq!(normalize_drc_bits(1), Some(DRC_BITS_MIN));
        assert_eq!(normalize_drc_bits(7), Some(DRC_BITS_MIN));
        assert_eq!(normalize_drc_bits(8), Some(8));
        assert_eq!(normalize_drc_bits(16), Some(16));
        assert_eq!(normalize_drc_bits(30), Some(30));
        assert_eq!(normalize_drc_bits(31), Some(DRC_BITS_MAX));
        assert_eq!(normalize_drc_bits(1_000), Some(DRC_BITS_MAX));
        assert_eq!(normalize_drc_bits(-1), None);
    }
}
