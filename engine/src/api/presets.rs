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
    /// Autoriser le nester à pivoter les instances autour de Z
    /// (`Slic3r::arrangement::ArrangeParams::allow_rotations`) : rotation de
    /// la boîte englobante minimale puis essais à 0/45/90/135°. Absent du JSON
    /// → `false`, comportement historique de l'arrangement UI.
    #[serde(default)]
    pub allow_rotations: bool,
    /// Sémantique exacte de l'arrangement du CLI Orca (OrcaSlicer.cpp, branche
    /// `need_arrange`) : alignement final `CENTER` du nester
    /// (`ArrangeParams::do_final_align`, défaut libslic3r) et zones exclues
    /// gonflées de 1 mm (`update_unselected_items_inflation` /
    /// `preprocess_exclude_areas(…, scale_(1))`). Absent du JSON → `false`,
    /// comportement historique de l'arrangement UI (positions validées par le
    /// nester, sans alignement final).
    #[serde(default)]
    pub orca_cli_mode: bool,
    /// Config d'impression résolue : si présente, arrangement exact du CLI Orca
    /// piloté par la config (brim, dégagements extrudeur, `print_sequence`…),
    /// y compris l'arrangement séquentiel « objet par objet ».
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub print_config: Option<crate::api::DynamicPrintConfig>,
    /// Force l'arrangement séquentiel (`ArrangeParams::is_seq_print`) même si
    /// `print_sequence` de la config n'est pas « by object » (réglage de plateau).
    #[serde(default)]
    pub is_seq_print: bool,
}

impl Default for ArrangeParams {
    fn default() -> Self {
        Self {
            clearance: 5.0,
            use_machine_clearance: true,
            allow_rotations: false,
            orca_cli_mode: false,
            print_config: None,
            is_seq_print: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ArrangeParams;

    #[test]
    fn allow_rotations_defaults_to_false_for_existing_payloads() {
        let params: ArrangeParams =
            serde_json::from_str(r#"{"clearance":2.0,"use_machine_clearance":false}"#).unwrap();
        assert!(!params.allow_rotations);
        assert!(!ArrangeParams::default().allow_rotations);
        let params: ArrangeParams = serde_json::from_str(
            r#"{"clearance":0.0,"use_machine_clearance":false,"allow_rotations":true}"#,
        )
        .unwrap();
        assert!(params.allow_rotations);
        assert!(!params.orca_cli_mode);
    }
}
