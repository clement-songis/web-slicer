//! Modèle tel que lu par libslic3r (`engine-worker load-model-raw`) : maillage
//! recentré en float32 par Orca (`ModelVolume::center_geometry_after_creation`)
//! et matrice de volume, SANS réintégrer le décalage dans les sommets. Le
//! maillage d'affichage (WSMh) cuit ce décalage en float32 : sur un modèle à
//! sommets non entiers, les points diffèrent d'Orca d'~1 µm (trou lamé,
//! params-review cb2). Utilisé par le harness et par le tranchage de production.

use std::path::Path;
use std::time::Duration;

use engine::api::{Model, TriangleMesh, VolumeRole};

use crate::domain::ModelFormat;

const TIMEOUT: Duration = Duration::from_secs(170);

/// FNV-1a 64 bits du contenu (clé de cache, sans dépendance).
fn content_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// Cache disque à côté du fichier source (comme `<gcode>.wspg`) :
/// `<fichier>.raw-<hash>.json`. Le nom porte le hash du contenu : un fichier
/// remplacé (« Replace 3D file ») invalide le cache de lui-même.
pub fn cache_path(source: &Path, bytes: &[u8]) -> std::path::PathBuf {
    let mut name = source.as_os_str().to_owned();
    name.push(format!(".raw-{:016x}.json", content_hash(bytes)));
    std::path::PathBuf::from(name)
}

/// Modèle brut libslic3r du fichier `source` ; `None` si le format n'est pas
/// lu par le moteur ou si la lecture échoue (l'appelant garde son repli).
/// Résultat mis en cache sur disque (un seul appel worker par contenu).
pub async fn load(source: &Path, format: ModelFormat) -> Option<Model> {
    let fmt = crate::models::convert::format_arg(format)?;
    let bytes = tokio::fs::read(source).await.ok()?;
    let cache = cache_path(source, &bytes);
    if let Ok(cached) = tokio::fs::read(&cache).await {
        if let Ok(model) = serde_json::from_slice::<Model>(&cached) {
            return Some(model);
        }
    }
    let model = load_uncached(source, fmt).await?;
    if let Ok(json) = serde_json::to_vec(&model) {
        let tmp = cache.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
        if tokio::fs::write(&tmp, json).await.is_ok()
            && tokio::fs::rename(&tmp, &cache).await.is_err()
        {
            let _ = tokio::fs::remove_file(&tmp).await;
        }
    }
    Some(model)
}

async fn load_uncached(source: &Path, fmt: &str) -> Option<Model> {
    let work = crate::workdir::tempdir().ok()?;
    let out = work.path().join("model-raw.json");
    super::run_worker(
        &["load-model-raw", source.to_str()?, fmt, out.to_str()?],
        TIMEOUT,
    )
    .await
    .ok()?;
    serde_json::from_slice(&tokio::fs::read(&out).await.ok()?).ok()
}

/// Maillage d'un objet importé tel que l'éditeur d'Orca le tranche : volume
/// unique, sommets recentrés par libslic3r (après `center_around_origin`, le
/// décalage du volume est ramené à zéro et seuls ces sommets restent). Les
/// indices sont ceux du maillage servi à l'éditeur (même ordre de lecture),
/// donc compatibles avec la peinture.
pub async fn centered_part_mesh(source: &Path, format: ModelFormat) -> Option<TriangleMesh> {
    let model = load(source, format).await?;
    let [object] = model.objects.as_slice() else {
        return None;
    };
    let [volume] = object.volumes.as_slice() else {
        return None;
    };
    (volume.role == VolumeRole::ModelPart).then(|| volume.mesh.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_key_follows_content() {
        let p = Path::new("/d/models/a.stl");
        assert_eq!(cache_path(p, b"abc"), cache_path(p, b"abc"));
        assert_ne!(cache_path(p, b"abc"), cache_path(p, b"abd"));
        assert!(cache_path(p, b"abc")
            .to_string_lossy()
            .starts_with("/d/models/a.stl.raw-"));
    }

    #[tokio::test]
    async fn cached_model_is_served_without_worker() {
        let dir = tempfile::tempdir().unwrap();
        let stl = dir.path().join("m.stl");
        std::fs::write(&stl, b"solid x\nendsolid x\n").unwrap();
        let mut model = Model::default();
        model.add_object(
            "cache",
            TriangleMesh {
                vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                indices: vec![[0, 1, 2]],
            },
        );
        let bytes = std::fs::read(&stl).unwrap();
        std::fs::write(
            cache_path(&stl, &bytes),
            serde_json::to_vec(&model).unwrap(),
        )
        .unwrap();
        // Le cache répond avant tout appel au worker (le fichier STL est vide :
        // le worker ne pourrait pas produire ce modèle).
        let loaded = load(&stl, ModelFormat::Stl).await;
        assert_eq!(loaded.unwrap().objects[0].name, "cache");
        // Contenu modifié : clé différente → l'entrée de cache n'est pas servie.
        std::fs::write(&stl, b"solid y\nendsolid y\n").unwrap();
        let miss = load(&stl, ModelFormat::Stl).await;
        assert!(miss.is_none_or(|m| m.objects.first().map(|o| o.name.as_str()) != Some("cache")));
    }
}
