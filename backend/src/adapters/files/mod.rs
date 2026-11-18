//! Adaptateur fichiers : espaces de stockage par utilisateur
//! `data/users/<uid>/{models,gcodes,thumbnails}` (data-model.md, FR-053).
//!
//! Sécurité : les chemins sont construits **uniquement** à partir d'IDs typés
//! (UUID) et d'extensions validées — aucune donnée utilisateur libre n'entre
//! dans un chemin, la traversée (`../`) est donc impossible par construction ;
//! la validation d'extension ferme la porte défensivement.

use std::path::{Path, PathBuf};

use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::domain::{GcodeId, ModelId, UserId};

/// Erreur de l'adaptateur fichiers.
#[derive(Debug, thiserror::Error)]
pub enum FilesError {
    #[error("erreur d'entrée/sortie : {0}")]
    Io(#[from] std::io::Error),
    /// Composant de chemin dangereux (séparateur, `..`, vide).
    #[error("nom de fichier non sûr : {0}")]
    UnsafeName(String),
}

pub type FilesResult<T> = Result<T, FilesError>;

/// Sous-espace de fichiers d'un utilisateur.
#[derive(Debug, Clone, Copy)]
pub enum Space {
    Models,
    Gcodes,
    Thumbnails,
    /// Maillages d'affichage « WSMh » produits par la conversion moteur (T123).
    Meshes,
}

impl Space {
    fn dir(self) -> &'static str {
        match self {
            Space::Models => "models",
            Space::Gcodes => "gcodes",
            Space::Thumbnails => "thumbnails",
            Space::Meshes => "meshes",
        }
    }
}

/// Magasin de fichiers ancré sur un répertoire racine (`data/`).
#[derive(Debug, Clone)]
pub struct FileStore {
    root: PathBuf,
}

impl FileStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Racine d'un utilisateur : `<root>/users/<uid>`.
    fn user_root(&self, user: UserId) -> PathBuf {
        self.root.join("users").join(user.to_string())
    }

    fn space_dir(&self, user: UserId, space: Space) -> PathBuf {
        self.user_root(user).join(space.dir())
    }

    /// Écrit un modèle `<uid>/models/<model_id>.<ext>` de façon atomique.
    pub async fn write_model(
        &self,
        user: UserId,
        model: ModelId,
        ext: &str,
        bytes: &[u8],
    ) -> FilesResult<PathBuf> {
        let name = format!("{}.{}", model, safe_ext(ext)?);
        self.write_atomic(user, Space::Models, &name, bytes).await
    }

    /// Écrit un maillage d'affichage `<uid>/meshes/<model_id>.wsm` (WSMh, T123).
    pub async fn write_mesh(
        &self,
        user: UserId,
        model: ModelId,
        bytes: &[u8],
    ) -> FilesResult<PathBuf> {
        let name = format!("{model}.wsm");
        self.write_atomic(user, Space::Meshes, &name, bytes).await
    }

    /// Écrit un G-code `<uid>/gcodes/<gcode_id>.gcode`.
    pub async fn write_gcode(
        &self,
        user: UserId,
        gcode: GcodeId,
        bytes: &[u8],
    ) -> FilesResult<PathBuf> {
        let name = format!("{gcode}.gcode");
        self.write_atomic(user, Space::Gcodes, &name, bytes).await
    }

    /// Écrit une vignette `<uid>/thumbnails/<gcode_id>.png`.
    pub async fn write_thumbnail(
        &self,
        user: UserId,
        gcode: GcodeId,
        bytes: &[u8],
    ) -> FilesResult<PathBuf> {
        let name = format!("{gcode}.png");
        self.write_atomic(user, Space::Thumbnails, &name, bytes)
            .await
    }

    /// Vignette de projet rendue côté serveur : `<uid>/thumbnails/project-<id>.png`.
    pub fn project_thumbnail_path(&self, user: UserId, project: impl std::fmt::Display) -> PathBuf {
        self.space_dir(user, Space::Thumbnails)
            .join(format!("project-{project}.png"))
    }

    /// Vignette fournie par l'interface pour le plateau `plate` (0-based) d'un
    /// projet, écrite dans le 3MF exporté (Metadata/plate_<n>.png).
    pub fn plate_thumbnail_path(
        &self,
        user: UserId,
        project: impl std::fmt::Display,
        plate: u32,
    ) -> PathBuf {
        self.space_dir(user, Space::Thumbnails)
            .join(format!("project-{project}-plate-{plate}.png"))
    }

    /// Écrit la vignette de plateau fournie par l'interface (atomique).
    pub async fn write_plate_thumbnail(
        &self,
        user: UserId,
        project: impl std::fmt::Display,
        plate: u32,
        bytes: &[u8],
    ) -> FilesResult<PathBuf> {
        let name = format!("project-{project}-plate-{plate}.png");
        self.write_atomic(user, Space::Thumbnails, &name, bytes)
            .await
    }

    /// Écrit la vignette de projet (atomique).
    pub async fn write_project_thumbnail(
        &self,
        user: UserId,
        project: impl std::fmt::Display,
        bytes: &[u8],
    ) -> FilesResult<PathBuf> {
        let name = format!("project-{project}.png");
        self.write_atomic(user, Space::Thumbnails, &name, bytes)
            .await
    }

    /// Écriture atomique : fichier temporaire dans le même dossier puis rename.
    async fn write_atomic(
        &self,
        user: UserId,
        space: Space,
        name: &str,
        bytes: &[u8],
    ) -> FilesResult<PathBuf> {
        ensure_component(name)?;
        let dir = self.space_dir(user, space);
        fs::create_dir_all(&dir).await?;
        let final_path = dir.join(name);
        let tmp = dir.join(format!(".{}.tmp", uuid::Uuid::new_v4()));

        let mut file = fs::File::create(&tmp).await?;
        file.write_all(bytes).await?;
        file.sync_all().await?;
        drop(file);
        fs::rename(&tmp, &final_path).await?;
        Ok(final_path)
    }

    /// Dossier des fichiers auxiliaires d'un projet (Orca : Auxiliaries/ du 3MF) :
    /// `<uid>/auxiliaries/<projet>/<dossier Orca>/<fichier>`.
    pub fn auxiliaries_dir(&self, user: UserId, project: impl std::fmt::Display) -> PathBuf {
        self.user_root(user)
            .join("auxiliaries")
            .join(project.to_string())
    }

    /// Valide un chemin auxiliaire relatif « <dossier>/<fichier> » : dossiers
    /// d'Orca (AuxiliaryDataViewModel), composants sûrs, pas de remontée.
    pub fn auxiliary_path(
        &self,
        user: UserId,
        project: impl std::fmt::Display,
        rel: &str,
    ) -> FilesResult<PathBuf> {
        let mut parts = rel.split('/');
        let (Some(folder), Some(name), None) = (parts.next(), parts.next(), parts.next()) else {
            return Err(FilesError::UnsafeName(rel.to_string()));
        };
        if !AUXILIARY_FOLDERS.contains(&folder) || name.starts_with('.') {
            return Err(FilesError::UnsafeName(rel.to_string()));
        }
        ensure_component(name)?;
        Ok(self.auxiliaries_dir(user, project).join(folder).join(name))
    }

    /// Fichiers auxiliaires d'un projet (chemins relatifs triés, tailles).
    pub async fn list_auxiliaries(
        &self,
        user: UserId,
        project: impl std::fmt::Display,
    ) -> FilesResult<Vec<(String, u64)>> {
        let root = self.auxiliaries_dir(user, project);
        let mut out = Vec::new();
        for folder in AUXILIARY_FOLDERS {
            let mut dir = match fs::read_dir(root.join(folder)).await {
                Ok(d) => d,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e.into()),
            };
            while let Some(entry) = dir.next_entry().await? {
                let meta = entry.metadata().await?;
                let name = entry.file_name().to_string_lossy().into_owned();
                if meta.is_file() && !name.starts_with('.') {
                    out.push((format!("{folder}/{name}"), meta.len()));
                }
            }
        }
        out.sort();
        Ok(out)
    }

    /// Écrit un fichier auxiliaire (atomique).
    pub async fn write_auxiliary(&self, path: &Path, bytes: &[u8]) -> FilesResult<()> {
        let dir = path
            .parent()
            .ok_or_else(|| FilesError::UnsafeName(path.display().to_string()))?;
        fs::create_dir_all(dir).await?;
        let tmp = dir.join(format!(".{}.tmp", uuid::Uuid::new_v4()));
        fs::write(&tmp, bytes).await?;
        fs::rename(&tmp, path).await?;
        Ok(())
    }

    /// Lit un fichier déjà stocké (chemin issu du magasin).
    pub async fn read(&self, path: &Path) -> FilesResult<Vec<u8>> {
        Ok(fs::read(path).await?)
    }

    /// Purge totale de l'espace d'un utilisateur (suppression de compte, FR-053).
    /// Idempotent : ne renvoie pas d'erreur si l'espace n'existe pas.
    pub async fn purge_user(&self, user: UserId) -> FilesResult<()> {
        let dir = self.user_root(user);
        match fs::remove_dir_all(&dir).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

/// Dossiers de fichiers auxiliaires d'Orca (AuxiliaryDataViewModel).
pub const AUXILIARY_FOLDERS: [&str; 4] = [
    "Model Pictures",
    "Bill of Materials",
    "Assembly Guide",
    "Others",
];

/// Valide qu'une extension ne contient que des caractères sûrs.
fn safe_ext(ext: &str) -> FilesResult<&str> {
    if !ext.is_empty() && ext.chars().all(|c| c.is_ascii_alphanumeric()) {
        Ok(ext)
    } else {
        Err(FilesError::UnsafeName(ext.to_string()))
    }
}

/// Rejette tout composant contenant un séparateur, `..`, ou vide.
fn ensure_component(name: &str) -> FilesResult<()> {
    let bad = name.is_empty()
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0');
    if bad {
        return Err(FilesError::UnsafeName(name.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, FileStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = FileStore::new(dir.path());
        (dir, store)
    }

    #[tokio::test]
    async fn writes_and_reads_back_a_model() {
        let (_dir, store) = store();
        let user = UserId::new();
        let model = ModelId::new();
        let path = store
            .write_model(user, model, "stl", b"solid cube")
            .await
            .unwrap();
        assert!(path.ends_with(format!("{model}.stl")));
        assert!(path.to_string_lossy().contains(&user.to_string()));
        assert_eq!(store.read(&path).await.unwrap(), b"solid cube");
    }

    #[tokio::test]
    async fn users_are_isolated_in_separate_dirs() {
        let (_dir, store) = store();
        let a = UserId::new();
        let b = UserId::new();
        let pa = store.write_gcode(a, GcodeId::new(), b"a").await.unwrap();
        let pb = store.write_gcode(b, GcodeId::new(), b"b").await.unwrap();
        assert_ne!(
            pa.parent(),
            pb.parent(),
            "espaces disjoints par utilisateur"
        );
    }

    #[tokio::test]
    async fn purge_removes_everything_for_the_user() {
        let (_dir, store) = store();
        let user = UserId::new();
        let path = store
            .write_model(user, ModelId::new(), "stl", b"x")
            .await
            .unwrap();
        assert!(path.exists());
        store.purge_user(user).await.unwrap();
        assert!(!path.exists());
        // Idempotent.
        store.purge_user(user).await.unwrap();
    }

    #[tokio::test]
    async fn path_traversal_is_impossible() {
        let (_dir, store) = store();
        let user = UserId::new();
        // Une extension malveillante est rejetée avant tout accès disque.
        let err = store
            .write_model(user, ModelId::new(), "../../etc/passwd", b"x")
            .await
            .unwrap_err();
        assert!(matches!(err, FilesError::UnsafeName(_)), "{err:?}");
    }
}
