//! Erreur du domaine de stockage — indépendante du SGBD (constitution III).
//! Les adaptateurs traduisent leurs erreurs SQL vers ces variantes.

/// Erreur renvoyée par les opérations du trait `Storage`.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// Ressource absente (ou hors du périmètre de l'utilisateur : l'isolation
    /// se traduit par « introuvable », jamais « interdit », pour ne pas
    /// divulguer l'existence d'une ressource d'autrui — SC-008).
    #[error("ressource introuvable")]
    NotFound,

    /// Violation d'unicité (email, nom de projet, nom de preset…).
    #[error("conflit d'unicité : {0}")]
    Conflict(String),

    /// Verrou optimiste : version fournie ≠ version stockée (409).
    #[error("conflit de version (attendu {expected}, reçu {actual})")]
    VersionConflict { expected: i64, actual: i64 },

    /// Règle métier violée (ex. supprimer le dernier admin).
    #[error("opération refusée : {0}")]
    Forbidden(String),

    /// Erreur du backend de stockage (SQL, connexion…).
    #[error("erreur de stockage : {0}")]
    Backend(String),
}

pub type StorageResult<T> = Result<T, StorageError>;
