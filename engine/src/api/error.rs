//! Erreur moteur structurée (contrat slicer-engine-trait.md, garantie 2).

use serde::{Deserialize, Serialize};

/// Code stable d'erreur moteur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineErrorCode {
    /// Format de fichier non reconnu ou fichier corrompu.
    InvalidModel,
    /// Valeur de configuration invalide (type, borne, enum).
    InvalidConfig,
    /// Objet hors du volume d'impression.
    OutOfBuildVolume,
    /// Le process moteur (FFI worker ou CLI) a échoué ou crashé.
    EngineCrashed,
    /// Annulé par l'utilisateur (kill coopératif).
    Cancelled,
    /// Fonction non disponible dans cette implémentation (ex. STEP en CLI).
    Unsupported,
    /// Erreur d'entrées/sorties (fichiers de travail).
    Io,
    /// Erreur interne inattendue.
    Internal,
}

/// Erreur remontée par toute opération du trait `SlicerEngine`.
#[derive(Debug, Clone, thiserror::Error, Serialize, Deserialize)]
#[error("{code:?}: {message}")]
pub struct EngineError {
    pub code: EngineErrorCode,
    /// Message d'origine du moteur, restitué tel quel (FR-032).
    pub message: String,
    /// Objet ou paramètre concerné, si identifiable.
    pub subject: Option<String>,
}

impl EngineError {
    pub fn new(code: EngineErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            subject: None,
        }
    }

    pub fn with_subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }
}

impl From<std::io::Error> for EngineError {
    fn from(e: std::io::Error) -> Self {
        Self::new(EngineErrorCode::Io, e.to_string())
    }
}

pub type EngineResult<T> = Result<T, EngineError>;
