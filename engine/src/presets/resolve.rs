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
/// Chaque paire clé/valeur passe par `handle_legacy` avant placement — miroir
/// de `ConfigBase::set_deserialize_nothrow` (Config.cpp:604 : « Both opt_key
/// and value may be modified by handle_legacy() ») — puis les clés inconnues
/// du registre sont ignorées.
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
            let text = json_value_to_orca_text(value, def);
            // handle_legacy (Config.cpp:604) : renommage éventuel de la clé,
            // transformation de la valeur (ex. enable_arc_fitting « 1 » →
            // « enabled », devenu coEnum dans main) ou abandon (clé obsolète).
            let Some((key, text)) = params::handle_legacy(key, &text) else {
                continue;
            };
            // Après renommage, handle_legacy garantit une clé du registre ;
            // la définition finale pilote le parse.
            let def = params::get(&key).unwrap_or(def);
            if let Some(parsed) = orca_values::parse_orca_value(def, &text) {
                config.0.insert(key, parsed);
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

/// Lot 46 (audit 44 PR1-a) : valeur JSON d'un profil (éventuellement typée par
/// l'interface : `25`, `true`, `[0.8]`) ramenée à la forme d'un fichier de
/// preset Orca (`ConfigBase::save_to_json` : chaîne pour un scalaire,
/// tableau de chaînes pour un vecteur). Une valeur déjà textuelle (chaîne ou
/// tableau de chaînes) est conservée telle quelle si elle est valide pour le
/// registre ; `None` si la clé est inconnue ou la valeur invalide.
pub fn canonical_orca_json(key: &str, value: &serde_json::Value) -> Option<serde_json::Value> {
    use serde_json::Value;
    let def = params::get(key)?;
    let textual = match value {
        Value::String(_) => true,
        Value::Array(items) => items.iter().all(Value::is_string),
        _ => false,
    };
    let prepared = if textual {
        value.clone()
    } else {
        let scalar = |v: &Value| -> Value {
            match v {
                Value::Bool(b) => Value::String(if *b { "1" } else { "0" }.into()),
                Value::Number(n) => Value::String(n.to_string()),
                Value::Null => Value::String("nil".into()),
                Value::String(s) => Value::String(s.clone()),
                Value::Object(o) => match (o.get("value"), o.get("percent")) {
                    // FloatOrPercent typé : { value, percent }.
                    (Some(Value::Number(n)), Some(Value::Bool(true))) => {
                        Value::String(format!("{n}%"))
                    }
                    (Some(Value::Number(n)), _) => Value::String(n.to_string()),
                    _ => Value::String(v.to_string()),
                },
                other => Value::String(other.to_string()),
            }
        };
        match value {
            Value::Array(items) => Value::Array(items.iter().map(scalar).collect()),
            other => scalar(other),
        }
    };
    let text = json_value_to_orca_text(&prepared, def);
    let parsed = orca_values::parse_orca_value(def, &text)?;
    if textual {
        return Some(prepared);
    }
    Some(config_value_to_orca_json(def, &parsed))
}

/// `ConfigValue` typé → forme JSON d'un fichier de preset Orca (scalaire :
/// chaîne `opt->serialize()` ; vecteur : un élément sérialisé par entrée,
/// `vserialize()`).
pub fn config_value_to_orca_json(
    def: &params::ParamDef,
    value: &crate::api::ConfigValue,
) -> serde_json::Value {
    use crate::api::ConfigValue as V;
    use serde_json::Value;
    let one = |v: V| Value::String(orca_values::serialize_orca_value_for(def, &v));
    match value {
        V::Bools(items) => Value::Array(items.iter().map(|b| one(V::Bools(vec![*b]))).collect()),
        V::BoolsNullable(items) => Value::Array(
            items
                .iter()
                .map(|b| one(V::BoolsNullable(vec![*b])))
                .collect(),
        ),
        V::Ints(items) => Value::Array(items.iter().map(|i| one(V::Ints(vec![*i]))).collect()),
        V::Floats(items) => Value::Array(items.iter().map(|f| one(V::Floats(vec![*f]))).collect()),
        V::FloatsNullable(items) => Value::Array(
            items
                .iter()
                .map(|f| one(V::FloatsNullable(vec![*f])))
                .collect(),
        ),
        V::Points(items) => Value::Array(
            items
                .iter()
                .map(|p| one(V::Points(vec![p.clone()])))
                .collect(),
        ),
        // Vecteurs de chaînes / d'enums / groupes de points : éléments bruts.
        V::Strings(items) if def.kind != params::ParamKind::String => {
            if items.is_empty() && def.nullable && def.kind == params::ParamKind::Enums {
                Value::Array(vec![Value::String("nil".into())])
            } else {
                Value::Array(items.iter().cloned().map(Value::String).collect())
            }
        }
        other => Value::String(orca_values::serialize_orca_value_for(def, other)),
    }
}

/// Les profils Orca sérialisent les vecteurs numériques en CSV et les vecteurs
/// de chaînes avec `;`. Le séparateur dépend donc du type du registre.
fn json_value_to_orca_text(value: &serde_json::Value, def: &params::ParamDef) -> String {
    let separator = match def.kind {
        params::ParamKind::Strings | params::ParamKind::Enums => ";",
        // Groupes de points (extruder_printable_area) : un groupe par élément,
        // séparés par « # » comme dans la sérialisation Orca.
        params::ParamKind::PointsGroups => "#",
        _ => ",",
    };
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(items) => items
            .iter()
            .map(|v| match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .map(|text| match def.kind {
                // Protège les `;` et guillemets contenus dans les valeurs, en
                // particulier les commentaires G-code qui commencent par `;`.
                params::ParamKind::Strings | params::ParamKind::Enums => {
                    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
                }
                _ => text,
            })
            .collect::<Vec<_>>()
            .join(separator),
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
    fn orca_json_string_arrays_use_semicolons() {
        let chain = [preset(
            "process",
            &[("compatible_printers", serde_json::json!(["A", "B"]))],
        )];
        let cfg = resolve_preset_chain(&chain).unwrap();
        assert_eq!(
            cfg.get("compatible_printers"),
            Some(&ConfigValue::Strings(vec!["A".into(), "B".into()]))
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
        let profiles = std::env::var("ORCA_SRC")
            .map(|src| PathBuf::from(src).join("resources/profiles/BBL/process"))
            .ok()
            .filter(|dir| dir.is_dir())
            .unwrap_or_else(|| {
                PathBuf::from(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../vendor/OrcaSlicer/resources/profiles/BBL/process"
                ))
            });
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
                renamed_from: vec![],
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
                renamed_from: vec![],
                compatible_printers: vec![],
                values: serde_json::Map::new(),
            },
        ];
        let cyclic_index = PresetIndex::new(&cyclic);
        assert!(cyclic_index.chain("V", PresetKind::Process, "a").is_err());
    }
}
