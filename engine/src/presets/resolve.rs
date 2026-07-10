//! Résolution d'une chaîne d'héritage de presets (R5, FR-020) — fonction
//! pure, partagée par les adaptateurs FFI et CLI (T017), enrichie en T037.
//!
//! La chaîne arrive ordonnée de la racine vers la feuille ; chaque niveau ne
//! contient que ses surcharges (data-model.md). Les valeurs sont au format
//! JSON Orca : chaîne (`"0.2"`) ou tableau de chaînes (`["0.4","0.4"]`,
//! une entrée par extrudeur/filament).

use std::collections::HashMap;

use crate::api::{
    ConfigWarning, DynamicPrintConfig, EngineError, EngineErrorCode, EngineResult, RawPreset,
};
use crate::params::{self, orca_values};

use super::import::{ImportedPreset, PresetKind};

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

/// Index des presets système permettant de reconstruire une chaîne d'héritage
/// par nom, puis de la résoudre en valeurs effectives (R5, FR-020).
///
/// L'héritage (`inherits`) est résolu **dans le même vendeur et le même type** :
/// des noms comme `fdm_machine_common` sont partagés entre vendeurs, mais un
/// preset n'hérite que d'un parent de son propre vendeur.
pub struct PresetIndex<'a> {
    presets: &'a [ImportedPreset],
    by_key: HashMap<(String, PresetKind, String), usize>,
}

impl<'a> PresetIndex<'a> {
    /// Construit l'index à partir d'un lot de presets importés (T036).
    pub fn new(presets: &'a [ImportedPreset]) -> Self {
        let mut by_key = HashMap::with_capacity(presets.len());
        for (i, p) in presets.iter().enumerate() {
            by_key.insert((p.vendor.clone(), p.kind, p.name.clone()), i);
        }
        Self { presets, by_key }
    }

    fn lookup(&self, vendor: &str, kind: PresetKind, name: &str) -> Option<&'a ImportedPreset> {
        self.by_key
            .get(&(vendor.to_string(), kind, name.to_string()))
            .map(|&i| &self.presets[i])
    }

    /// Chaîne d'héritage **racine → feuille** d'un preset nommé. Erreur si le
    /// preset ou un parent est introuvable, ou si un cycle est détecté.
    pub fn chain(
        &self,
        vendor: &str,
        kind: PresetKind,
        name: &str,
    ) -> EngineResult<Vec<&'a ImportedPreset>> {
        let mut chain = Vec::new();
        let mut seen = Vec::new();
        let mut current = Some(name.to_string());
        while let Some(cur) = current {
            if seen.contains(&cur) {
                return Err(EngineError::new(
                    EngineErrorCode::InvalidConfig,
                    format!("cycle d'héritage sur « {cur} »"),
                ));
            }
            let preset = self.lookup(vendor, kind, &cur).ok_or_else(|| {
                EngineError::new(
                    EngineErrorCode::InvalidConfig,
                    format!("preset « {cur} » introuvable ({vendor}/{})", kind.as_str()),
                )
            })?;
            seen.push(cur);
            current = preset.inherits.clone();
            chain.push(preset);
        }
        chain.reverse(); // feuille→racine devient racine→feuille
        Ok(chain)
    }

    /// Valeurs effectives d'un preset système nommé (chaîne aplatie).
    pub fn resolve(
        &self,
        vendor: &str,
        kind: PresetKind,
        name: &str,
    ) -> EngineResult<DynamicPrintConfig> {
        let chain = self.chain(vendor, kind, name)?;
        let raws: Vec<RawPreset> = chain.iter().map(|p| to_raw(p)).collect();
        resolve_preset_chain(&raws)
    }

    /// Résout un preset **dérivé** par l'utilisateur : la chaîne du parent plus
    /// les seules surcharges stockées. Un changement du parent se propage donc
    /// automatiquement à la prochaine résolution (US3-AS4).
    pub fn resolve_derived(
        &self,
        vendor: &str,
        kind: PresetKind,
        parent: &str,
        overrides: &serde_json::Map<String, serde_json::Value>,
    ) -> EngineResult<DynamicPrintConfig> {
        let mut raws: Vec<RawPreset> = self
            .chain(vendor, kind, parent)?
            .iter()
            .map(|p| to_raw(p))
            .collect();
        raws.push(RawPreset {
            name: format!("{parent} (dérivé)"),
            inherits: Some(parent.to_string()),
            values: overrides.clone(),
        });
        resolve_preset_chain(&raws)
    }
}

/// Vue `RawPreset` d'un preset importé (surcharges seules, méta exclus).
fn to_raw(p: &ImportedPreset) -> RawPreset {
    RawPreset {
        name: p.name.clone(),
        inherits: p.inherits.clone(),
        values: p.values.clone(),
    }
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

    // --- Résolution par index sur les profils importés (T037) ----------------

    use super::super::import::{import_profiles, ImportedPreset, PresetKind};

    fn imported() -> Vec<ImportedPreset> {
        let dir = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../vendor/OrcaSlicer/resources/profiles"
        ));
        import_profiles(&dir).expect("import").presets
    }

    #[test]
    fn chain_is_root_to_leaf() {
        let presets = imported();
        let index = PresetIndex::new(&presets);
        let chain = index
            .chain("BBL", PresetKind::Machine, "Bambu Lab A1 0.4 nozzle")
            .unwrap();
        let names: Vec<&str> = chain.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "fdm_machine_common",
                "fdm_bbl_3dp_001_common",
                "Bambu Lab A1 0.4 nozzle"
            ]
        );
    }

    /// « Bambu Lab A1 0.4 nozzle » → valeurs effectives connues, chacune héritée
    /// d'un niveau différent (racine, milieu, feuille).
    #[test]
    fn resolves_bambu_a1_effective_values() {
        let presets = imported();
        let index = PresetIndex::new(&presets);
        let cfg = index
            .resolve("BBL", PresetKind::Machine, "Bambu Lab A1 0.4 nozzle")
            .unwrap();
        // feuille
        assert_eq!(
            cfg.get("printable_height"),
            Some(&ConfigValue::Float(256.0))
        );
        // racine (fdm_machine_common) propagée jusqu'à la feuille
        assert_eq!(
            cfg.get("gcode_flavor"),
            Some(&ConfigValue::String("marlin".into()))
        );
        // niveau intermédiaire (fdm_bbl_3dp_001_common)
        assert!(cfg.get("retraction_length").is_some(), "héritée du parent");
    }

    /// Preset dérivé utilisateur : ne stocke que ses surcharges, hérite du reste.
    #[test]
    fn derived_preset_stores_overrides_only() {
        let presets = imported();
        let index = PresetIndex::new(&presets);
        let overrides = serde_json::json!({ "printable_height": "200" })
            .as_object()
            .unwrap()
            .clone();
        let cfg = index
            .resolve_derived(
                "BBL",
                PresetKind::Machine,
                "Bambu Lab A1 0.4 nozzle",
                &overrides,
            )
            .unwrap();
        // surcharge appliquée
        assert_eq!(
            cfg.get("printable_height"),
            Some(&ConfigValue::Float(200.0))
        );
        // reste hérité du parent (non stocké dans le dérivé)
        assert_eq!(
            cfg.get("gcode_flavor"),
            Some(&ConfigValue::String("marlin".into()))
        );
    }

    /// US3-AS4 : une modification du parent se propage à la re-résolution du
    /// dérivé (aucune valeur figée dans le preset dérivé).
    #[test]
    fn parent_change_propagates_to_derived() {
        // Chaîne minimale : parent porte `wall_loops`, dérivé ne surcharge que
        // `layer_height`. On re-résout après avoir changé le parent.
        let derived = preset("dérivé", &[("layer_height", serde_json::json!("0.1"))]);

        let parent_v1 = preset("parent", &[("wall_loops", serde_json::json!("2"))]);
        let cfg1 = resolve_preset_chain(&[parent_v1, derived.clone()]).unwrap();
        assert_eq!(cfg1.get("wall_loops"), Some(&ConfigValue::Int(2)));

        let parent_v2 = preset("parent", &[("wall_loops", serde_json::json!("4"))]);
        let cfg2 = resolve_preset_chain(&[parent_v2, derived]).unwrap();
        assert_eq!(
            cfg2.get("wall_loops"),
            Some(&ConfigValue::Int(4)),
            "changement du parent propagé"
        );
    }

    #[test]
    fn unknown_preset_and_cycles_are_errors() {
        let presets = imported();
        let index = PresetIndex::new(&presets);
        assert!(index.resolve("BBL", PresetKind::Machine, "nope").is_err());

        // Cycle synthétique A → B → A.
        let cyclic = vec![
            ImportedPreset {
                vendor: "V".into(),
                kind: PresetKind::Process,
                name: "a".into(),
                sub_path: String::new(),
                inherits: Some("b".into()),
                from: None,
                setting_id: None,
                filament_id: None,
                instantiation: false,
                compatible_printers: vec![],
                values: serde_json::Map::new(),
            },
            ImportedPreset {
                vendor: "V".into(),
                kind: PresetKind::Process,
                name: "b".into(),
                sub_path: String::new(),
                inherits: Some("a".into()),
                from: None,
                setting_id: None,
                filament_id: None,
                instantiation: false,
                compatible_printers: vec![],
                values: serde_json::Map::new(),
            },
        ];
        let cyclic_index = PresetIndex::new(&cyclic);
        assert!(cyclic_index.chain("V", PresetKind::Process, "a").is_err());
    }
}
