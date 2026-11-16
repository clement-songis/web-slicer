//! Répertoires de travail temporaires du serveur.
//!
//! `tempfile::tempdir()` crée sous `std::env::temp_dir()` (`TMPDIR`). Si ce
//! dossier disparaît pendant la vie du processus (shell `nix develop` ou
//! session systemd qui nettoie son `TMPDIR` à la sortie), chaque tranchage
//! échouerait en « No such file or directory ». On recrée donc la racine à la
//! volée avant chaque répertoire de travail.

use std::io;

/// Répertoire de travail isolé, supprimé au `Drop`, résistant à la
/// disparition de la racine temporaire.
pub fn tempdir() -> io::Result<tempfile::TempDir> {
    tempdir_under(&std::env::temp_dir())
}

/// Comme [`tempdir`] sous une racine explicite (recréée si elle a disparu).
fn tempdir_under(root: &std::path::Path) -> io::Result<tempfile::TempDir> {
    match tempfile::tempdir_in(root) {
        Ok(dir) => Ok(dir),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            std::fs::create_dir_all(root)?;
            tempfile::tempdir_in(root)
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn recreates_a_vanished_temp_root() {
        let root = tempfile::tempdir().unwrap();
        let gone = root.path().join("vanished");
        // Racine explicite : ne jamais muter TMPDIR du process de test (les
        // autres tests et leurs engine-worker l'héritent en parallèle ; leurs
        // répertoires de travail disparaissaient avec cette racine).
        let dir = super::tempdir_under(&gone).expect("la racine disparue est recréée");
        assert!(dir.path().starts_with(&gone));
        assert!(dir.path().is_dir());
    }
}
