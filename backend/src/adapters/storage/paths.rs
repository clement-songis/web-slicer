//! Lot 46 (audit 44 OLD-a) — chemins de fichiers stockés **relatifs** au
//! répertoire de données (`DATA_DIR`), pour qu'un déplacement / une
//! restauration de ce répertoire ne casse plus les projets existants.
//!
//! En base : `users/<uid>/…` (relatif à la racine). En mémoire (entités du
//! domaine) : chemin absolu sous la racine **courante**, comme avant — le reste
//! du backend est inchangé. Rattrapage des anciens enregistrements : un chemin
//! absolu hors de la racine courante (ancien `DATA_DIR`) est relocalisé sur
//! son suffixe `users/<uuid>/…` s'il existe sous la racine courante, à la
//! lecture comme par la migration de démarrage (`relativize_paths`).
//! Sans racine configurée (tests unitaires des repos), le codec est neutre.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use crate::domain::{Gcode, Model, Project, StorageResult};

/// Colonnes (table, colonne) portant un chemin de fichier du magasin.
pub const PATH_COLUMNS: [(&str, &str); 5] = [
    ("models", "file_path"),
    ("models", "mesh_path"),
    ("gcodes", "file_path"),
    ("gcodes", "preview_path"),
    ("projects", "thumbnail_path"),
];

/// Codec de chemins partagé par les repos d'un stockage.
#[derive(Debug, Clone, Default)]
pub struct DataPaths(Arc<OnceLock<PathBuf>>);

impl DataPaths {
    /// Fixe la racine des données (une seule fois ; appels suivants ignorés).
    pub fn set_root(&self, root: impl Into<PathBuf>) {
        let _ = self.0.set(root.into());
    }

    pub fn root(&self) -> Option<&Path> {
        self.0.get().map(PathBuf::as_path)
    }

    /// Chemin en mémoire → valeur stockée (relative à la racine si possible).
    pub fn encode(&self, path: &str) -> String {
        let Some(root) = self.root() else {
            return path.to_string();
        };
        let current = self.decode(path.to_string());
        match Path::new(&current).strip_prefix(root) {
            Ok(rel) if !rel.as_os_str().is_empty() => rel.to_string_lossy().into_owned(),
            _ => path.to_string(),
        }
    }

    pub fn encode_opt(&self, path: Option<&str>) -> Option<String> {
        path.map(|p| self.encode(p))
    }

    /// Valeur stockée → chemin en mémoire (absolu sous la racine courante).
    pub fn decode(&self, stored: String) -> String {
        let Some(root) = self.root() else {
            return stored;
        };
        if stored.is_empty() {
            return stored;
        }
        let path = Path::new(&stored);
        if path.is_relative() && stored.starts_with("users/") {
            return root.join(path).to_string_lossy().into_owned();
        }
        if path.starts_with(root) || path.exists() {
            return stored;
        }
        // Ancienne racine : relocalisation sur le suffixe users/<uuid>/….
        match relocated(root, &stored) {
            Some(p) if p.exists() => p.to_string_lossy().into_owned(),
            _ => stored,
        }
    }

    pub fn decode_opt(&self, stored: Option<String>) -> Option<String> {
        stored.map(|s| self.decode(s))
    }

    pub fn model(&self, model: StorageResult<Model>) -> StorageResult<Model> {
        model.map(|mut m| {
            m.file_path = self.decode(std::mem::take(&mut m.file_path));
            m.mesh_path = self.decode_opt(m.mesh_path.take());
            m
        })
    }

    pub fn project(&self, project: StorageResult<Project>) -> StorageResult<Project> {
        project.map(|mut p| {
            p.thumbnail_path = self.decode_opt(p.thumbnail_path.take());
            p
        })
    }

    pub fn gcode(&self, gcode: StorageResult<Gcode>) -> StorageResult<Gcode> {
        gcode.map(|mut g| {
            g.file_path = self.decode(std::mem::take(&mut g.file_path));
            g.preview_path = self.decode(std::mem::take(&mut g.preview_path));
            g
        })
    }
}

/// `<ancienne racine>/users/<uuid>/reste` → `<root>/users/<uuid>/reste`
/// (dernière occurrence de `users/<uuid>`).
fn relocated(root: &Path, stored: &str) -> Option<PathBuf> {
    let parts: Vec<&str> = stored.split('/').collect();
    let at = (0..parts.len().saturating_sub(2))
        .rev()
        .find(|&i| parts[i] == "users" && uuid::Uuid::parse_str(parts[i + 1]).is_ok())?;
    let mut out = root.to_path_buf();
    for part in &parts[at..] {
        if part.is_empty() || *part == ".." || *part == "." {
            return None;
        }
        out.push(part);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_without_root() {
        let paths = DataPaths::default();
        assert_eq!(paths.encode("/a/b"), "/a/b");
        assert_eq!(paths.decode("users/x".into()), "users/x");
    }

    #[test]
    fn stores_relative_and_reads_absolute() {
        let dir = tempfile::tempdir().unwrap();
        let paths = DataPaths::default();
        paths.set_root(dir.path());
        let abs = dir
            .path()
            .join("users/5d00935a-c6b3-40e8-934d-b10950000000/models/m.stl");
        let stored = paths.encode(&abs.to_string_lossy());
        assert_eq!(
            stored,
            "users/5d00935a-c6b3-40e8-934d-b10950000000/models/m.stl"
        );
        assert_eq!(paths.decode(stored), abs.to_string_lossy());
    }

    #[test]
    fn relocates_records_of_a_moved_data_dir() {
        let dir = tempfile::tempdir().unwrap();
        let uid = "48567c38-9a5a-439e-a6f4-6005b0000000";
        let file = dir.path().join("users").join(uid).join("meshes/m.wsm");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, b"x").unwrap();
        let paths = DataPaths::default();
        paths.set_root(dir.path());
        let old = format!("/var/lib/old-root/data/users/{uid}/meshes/m.wsm");
        assert_eq!(paths.decode(old.clone()), file.to_string_lossy());
        assert_eq!(paths.encode(&old), format!("users/{uid}/meshes/m.wsm"));
        // Fichier absent sous la nouvelle racine : valeur conservée.
        let missing = format!("/var/lib/old-root/data/users/{uid}/meshes/absent.wsm");
        assert_eq!(paths.decode(missing.clone()), missing);
        assert_eq!(paths.encode(&missing), missing);
    }
}
