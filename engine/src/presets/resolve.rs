//! Résolution d'une chaîne d'héritage de presets (R5, FR-020) — fonction
//! pure, partagée par les adaptateurs FFI et CLI (T017), enrichie en T037.
//!
//! La chaîne arrive ordonnée de la racine vers la feuille ; chaque niveau ne
//! contient que ses surcharges (data-model.md). Les valeurs sont au format
//! JSON Orca : chaîne (`"0.2"`) ou tableau de chaînes (`["0.4","0.4"]`,
//! une entrée par extrudeur/filament).

use crate::api::{
    ConfigWarning, DynamicPrintConfig, EngineError, EngineErrorCode, EngineResult, RawPreset,
};
use crate::params::{self, orca_values};

/// Aplati la chaîne (racine → feuille) en valeurs effectives typées.
/// Les clés inconnues du registre sont ignorées (comportement handle_legacy).
pub fn resolve_preset_chain(chain: &[RawPreset]) -> EngineResult<DynamicPrintConfig> {
    if chain.is_empty() {
        return Err(EngineError::new(
            EngineErrorCode::InvalidConfig,
            "chaîne de presets vide",
        ));
    }
    let mut config = DynamicPrintConfig::new();
    for preset in chain {
        for (key, value) in &preset.values {
            let Some(def) = params::get(key) else {
                continue; // clé legacy/vendeur inconnue : ignorée comme Orca
            };
            let text = json_value_to_orca_text(value);
            if let Some(parsed) = orca_values::parse_orca_value(def, &text) {
                config.0.insert(key.clone(), parsed);
            }
        }
    }
    Ok(config)
}

/// Contrôles de cohérence (bornes, enums) — mêmes corrections qu'OrcaSlicer.
pub fn validate_config(config: &DynamicPrintConfig) -> EngineResult<Vec<ConfigWarning>> {
    let mut copy = config.clone();
    Ok(copy.validate())
}

/// Les profils Orca sérialisent les vecteurs en tableaux JSON de chaînes ;
/// on les rejoint en CSV, le format qu'attend `parse_orca_value`.
fn json_value_to_orca_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(items) => items
            .iter()
            .map(|v| match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .collect::<Vec<_>>()
            .join(","),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ConfigValue;
    use std::path::PathBuf;

    fn preset(name: &str, pairs: &[(&str, serde_json::Value)]) -> RawPreset {
        RawPreset {
            name: name.into(),
            inherits: None,
            values: pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        }
    }

    #[test]
    fn leaf_overrides_root() {
        let chain = [
            preset(
                "base",
                &[
                    ("layer_height", serde_json::json!("0.2")),
                    ("wall_loops", serde_json::json!("2")),
                ],
            ),
            preset("fin", &[("layer_height", serde_json::json!("0.12"))]),
        ];
        let cfg = resolve_preset_chain(&chain).unwrap();
        assert_eq!(cfg.get("layer_height"), Some(&ConfigValue::Float(0.12)));
        assert_eq!(cfg.get("wall_loops"), Some(&ConfigValue::Int(2)));
    }

    #[test]
    fn orca_json_arrays_become_vectors() {
        let chain = [preset(
            "machine",
            &[("nozzle_diameter", serde_json::json!(["0.4", "0.6"]))],
        )];
        let cfg = resolve_preset_chain(&chain).unwrap();
        assert_eq!(
            cfg.get("nozzle_diameter"),
            Some(&ConfigValue::Floats(vec![0.4, 0.6]))
        );
    }

    #[test]
    fn unknown_keys_ignored_like_orca() {
        let chain = [preset(
            "legacy",
            &[
                ("layer_heigth", serde_json::json!("0.3")), // faute historique
                ("layer_height", serde_json::json!("0.2")),
            ],
        )];
        let cfg = resolve_preset_chain(&chain).unwrap();
        assert_eq!(cfg.get("layer_height"), Some(&ConfigValue::Float(0.2)));
        assert!(cfg.get("layer_heigth").is_none());
    }

    #[test]
    fn empty_chain_rejected() {
        assert!(resolve_preset_chain(&[]).is_err());
    }

    /// Chaîne réelle BBL : fdm_process_common → 0.20 Standard @BBL A1.
    #[test]
    fn real_bbl_chain() {
        let profiles = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../vendor/OrcaSlicer/resources/profiles/BBL/process"
        ));
        let load = |name: &str| -> RawPreset {
            let raw: serde_json::Map<String, serde_json::Value> =
                serde_json::from_str(&std::fs::read_to_string(profiles.join(name)).expect(name))
                    .unwrap();
            RawPreset {
                name: name.into(),
                inherits: raw
                    .get("inherits")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                values: raw,
            }
        };
        // chaîne réelle (ordre racine → feuille)
        let chain = [
            load("fdm_process_common.json"),
            load("fdm_process_single_common.json"),
            load("fdm_process_single_0.20.json"),
            load("0.20mm Standard @BBL A1.json"),
        ];
        let cfg = resolve_preset_chain(&chain).unwrap();
        // valeurs effectives connues du profil 0.20 Standard
        assert_eq!(cfg.get("layer_height"), Some(&ConfigValue::Float(0.2)));
        assert!(
            matches!(cfg.get("sparse_infill_density"), Some(ConfigValue::Float(d)) if *d > 0.0),
            "densité héritée présente : {:?}",
            cfg.get("sparse_infill_density")
        );
        // la validation de la chaîne réelle ne produit aucun avertissement
        let warnings = validate_config(&cfg).unwrap();
        assert!(
            warnings.is_empty(),
            "profil système sans avertissement : {warnings:?}"
        );
    }
}
