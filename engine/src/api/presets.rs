//! Presets bruts et paramètres d'arrangement — entrées du trait.

use serde::{Deserialize, Serialize};

/// Preset tel que stocké (système ou utilisateur) : uniquement les clés
/// surchargées, l'héritage n'est jamais aplati en base (R5, data-model.md).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawPreset {
    pub name: String,
    /// Nom du parent (`inherits`) — la chaîne est fournie ordonnée au trait,
    /// de la racine vers la feuille.
    pub inherits: Option<String>,
    /// Valeurs au format JSON Orca (chaînes/nombres/listes bruts du profil).
    pub values: serde_json::Map<String, serde_json::Value>,
}

/// Paramètres d'arrangement automatique (FR-013).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrangeParams {
    /// Dégagement minimal entre objets, en mm.
    pub clearance: f64,
    /// Respecter les dégagements machine (extruder_clearance_*).
    pub use_machine_clearance: bool,
}

impl Default for ArrangeParams {
    fn default() -> Self {
        Self {
            clearance: 5.0,
            use_machine_clearance: true,
        }
    }
}
