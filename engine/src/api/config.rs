//! `DynamicPrintConfig` — miroir du conteneur de valeurs de libslic3r,
//! validé par le registre (`params::REGISTRY`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::params::{self, ParamKind};

use super::error::{EngineError, EngineErrorCode, EngineResult};

/// Valeur typée d'un paramètre (miroir des `ConfigOption*`).
///
/// Les valeurs vectorielles (une par extrudeur/filament) utilisent `Vec` ;
/// `None` dans un vecteur nullable = « hérite de l'imprimante » (nil).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConfigValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    /// FloatOrPercent : `{ "value": 0.4, "percent": false }`.
    FloatOrPercent {
        value: f64,
        percent: bool,
    },
    String(String),
    Point(Vec<f64>),
    Bools(Vec<bool>),
    Ints(Vec<i64>),
    Floats(Vec<f64>),
    FloatsNullable(Vec<Option<f64>>),
    /// `ConfigOptionBoolsNullable` contenant au moins un `nil` (`None`) ;
    /// sans `nil`, la forme `Bools` reste utilisée.
    BoolsNullable(Vec<Option<bool>>),
    /// Vecteurs d'enums : nil (`ConfigOptionEnumsGenericNullable`) est porté
    /// par le jeton `"nil"`, jamais par un vecteur raccourci.
    Strings(Vec<String>),
    Points(Vec<Vec<f64>>),
}

/// Avertissement de validation non bloquant (FR-032, US2-AS3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigWarning {
    pub key: String,
    pub message: String,
    /// Valeur corrigée appliquée, le cas échéant (clamp aux bornes).
    pub corrected: Option<ConfigValue>,
}

/// Ensemble clé → valeur, seules les clés du registre sont admises.
#[derive(Debug, Clone, Default, Serialize)]
pub struct DynamicPrintConfig(pub BTreeMap<String, ConfigValue>);

/// La forme JSON des tableaux est volontairement non ambiguë seulement si on
/// connaît la clé Orca. Une désérialisation `untagged` de `ConfigValue` pouvait
/// transformer un vecteur machine en `Point` avant son arrivée au worker.
impl<'de> Deserialize<'de> for DynamicPrintConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = BTreeMap::<String, serde_json::Value>::deserialize(deserializer)?;
        let mut config = Self::new();
        for (key, value) in raw {
            let Some(def) = params::get(&key) else {
                continue;
            };
            let parsed = config_value_from_json(def, &value).ok_or_else(|| {
                serde::de::Error::custom(format!("valeur invalide pour le paramètre {key}"))
            })?;
            config.0.insert(key, parsed);
        }
        Ok(config)
    }
}

fn config_value_from_json(
    def: &params::ParamDef,
    value: &serde_json::Value,
) -> Option<ConfigValue> {
    use serde_json::Value as V;
    use ConfigValue as C;
    use ParamKind as K;

    let number = |value: &V| -> Option<f64> {
        match value {
            V::Number(n) => n.as_f64(),
            V::String(s) => s.trim().trim_end_matches('%').parse().ok(),
            _ => None,
        }
    };
    let integer = |value: &V| -> Option<i64> {
        match value {
            V::Number(n) => n.as_i64(),
            V::String(s) => s.trim().parse().ok(),
            _ => None,
        }
    };
    let boolean = |value: &V| -> Option<bool> {
        match value {
            V::Bool(b) => Some(*b),
            V::String(s) => match s.trim() {
                "1" | "true" => Some(true),
                "0" | "false" => Some(false),
                _ => None,
            },
            _ => None,
        }
    };
    let string = |value: &V| -> Option<String> {
        match value {
            V::String(s) => Some(s.clone()),
            V::Number(n) => Some(n.to_string()),
            V::Bool(b) => Some(b.to_string()),
            _ => None,
        }
    };

    match def.kind {
        K::Bool => boolean(value).map(C::Bool),
        K::Int => integer(value).map(C::Int),
        K::Float | K::Percent => number(value).map(C::Float),
        K::FloatOrPercent => {
            if let V::Object(object) = value {
                let value = object.get("value").and_then(&number)?;
                let percent = object.get("percent").and_then(boolean).unwrap_or(false);
                Some(C::FloatOrPercent { value, percent })
            } else if let V::String(text) = value {
                let percent = text.trim_end().ends_with('%');
                number(value).map(|value| C::FloatOrPercent { value, percent })
            } else {
                number(value).map(|value| C::FloatOrPercent {
                    value,
                    percent: false,
                })
            }
        }
        K::String | K::Enum => string(value).map(C::String),
        K::Point => value.as_array().and_then(|items| {
            items
                .iter()
                .map(number)
                .collect::<Option<Vec<_>>>()
                .map(C::Point)
        }),
        K::Bools => value.as_array().and_then(|items| {
            let is_nil = |item: &V| item.is_null() || item.as_str().map(str::trim) == Some("nil");
            if def.nullable && items.iter().any(is_nil) {
                items
                    .iter()
                    .map(|item| {
                        if is_nil(item) {
                            Some(None)
                        } else {
                            boolean(item).map(Some)
                        }
                    })
                    .collect::<Option<Vec<_>>>()
                    .map(C::BoolsNullable)
            } else {
                items
                    .iter()
                    .map(boolean)
                    .collect::<Option<Vec<_>>>()
                    .map(C::Bools)
            }
        }),
        K::Ints => value.as_array().and_then(|items| {
            items
                .iter()
                .map(integer)
                .collect::<Option<Vec<_>>>()
                .map(C::Ints)
        }),
        K::Floats | K::Percents | K::FloatsOrPercents => value.as_array().and_then(|items| {
            if def.nullable && items.iter().any(|item| item.is_null()) {
                items
                    .iter()
                    .map(|item| {
                        if item.is_null() {
                            Some(None)
                        } else {
                            number(item).map(Some)
                        }
                    })
                    .collect::<Option<Vec<_>>>()
                    .map(C::FloatsNullable)
            } else {
                items
                    .iter()
                    .map(number)
                    .collect::<Option<Vec<_>>>()
                    .map(C::Floats)
            }
        }),
        K::Strings | K::Enums => match value {
            V::String(_) if def.kind == K::Enum => string(value).map(C::String),
            // Enums Orca : `A,nil` — découpe sur `,`, `nil` conservé comme jeton.
            V::String(text) if def.kind == K::Enums => {
                let items: Vec<String> = text.split(',').map(|t| t.trim().to_string()).collect();
                if !def.nullable && items.iter().any(|t| t == "nil") {
                    return None;
                }
                Some(C::Strings(items))
            }
            V::String(text) => Some(C::Strings(vec![text.clone()])),
            V::Array(items) => items
                .iter()
                .map(|item| match item {
                    V::Null if def.kind == K::Enums && def.nullable => Some("nil".to_string()),
                    V::String(s) if def.kind == K::Enums && !def.nullable && s.trim() == "nil" => {
                        None
                    }
                    other => string(other),
                })
                .collect::<Option<Vec<_>>>()
                .map(C::Strings),
            _ => None,
        },
        K::Points | K::PointsGroups => value.as_array().and_then(|items| {
            items
                .iter()
                .map(|point| match point {
                    V::Array(coords) => coords.iter().map(number).collect::<Option<Vec<_>>>(),
                    V::String(text) => text
                        .split('x')
                        .map(|part| part.trim().parse::<f64>().ok())
                        .collect::<Option<Vec<_>>>(),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
                .map(C::Points)
        }),
    }
}

impl DynamicPrintConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: &str) -> Option<&ConfigValue> {
        self.0.get(key)
    }

    /// Insère après validation stricte (clé connue + type conforme).
    pub fn set(&mut self, key: &str, value: ConfigValue) -> EngineResult<()> {
        let def = params::get(key).ok_or_else(|| {
            EngineError::new(EngineErrorCode::InvalidConfig, "clé inconnue du registre")
                .with_subject(key)
        })?;
        if !value_matches_kind(&value, def.kind, def.nullable) {
            return Err(EngineError::new(
                EngineErrorCode::InvalidConfig,
                format!("type incompatible avec {:?}", def.kind),
            )
            .with_subject(key));
        }
        self.0.insert(key.to_string(), value);
        Ok(())
    }

    /// Valide les bornes/enums de toutes les valeurs ; corrige par clamp et
    /// retourne les avertissements (mêmes corrections qu'OrcaSlicer).
    pub fn validate(&mut self) -> Vec<ConfigWarning> {
        let mut warnings = Vec::new();
        for (key, value) in self.0.iter_mut() {
            let Some(def) = params::get(key) else {
                continue;
            };
            // bornes numériques
            if let (Some(min), Some(max)) = (def.min.or(Some(f64::MIN)), def.max.or(Some(f64::MAX)))
            {
                clamp_value(
                    value,
                    min,
                    max,
                    key,
                    def.min.is_some() || def.max.is_some(),
                    &mut warnings,
                );
            }
            // appartenance à l'enum
            if matches!(def.kind, ParamKind::Enum | ParamKind::Enums) {
                if let ConfigValue::String(s) = value {
                    if !def.enum_values.is_empty() && !def.enum_values.contains(&s.as_str()) {
                        warnings.push(ConfigWarning {
                            key: key.clone(),
                            message: format!(
                                "valeur d'enum inconnue « {s} » (attendues : {:?})",
                                def.enum_values
                            ),
                            corrected: None,
                        });
                    }
                }
            }
        }
        warnings
    }
}

fn value_matches_kind(v: &ConfigValue, kind: ParamKind, nullable: bool) -> bool {
    use ConfigValue as V;
    use ParamKind as K;
    match (kind, v) {
        (K::Bool, V::Bool(_))
        | (K::Int, V::Int(_))
        | (K::Float | K::Percent, V::Float(_) | V::Int(_))
        | (K::FloatOrPercent, V::FloatOrPercent { .. } | V::Float(_) | V::Int(_))
        | (K::String | K::Enum, V::String(_))
        | (K::Point, V::Point(_))
        | (K::Bools, V::Bools(_))
        | (K::Ints, V::Ints(_))
        | (K::Floats | K::Percents | K::FloatsOrPercents, V::Floats(_))
        | (K::Strings | K::Enums, V::Strings(_))
        | (K::Points | K::PointsGroups, V::Points(_)) => true,
        (K::Floats | K::Percents, V::FloatsNullable(_)) => nullable,
        (K::Bools, V::BoolsNullable(_)) => nullable,
        _ => false,
    }
}

fn clamp_value(
    value: &mut ConfigValue,
    min: f64,
    max: f64,
    key: &str,
    has_bounds: bool,
    warnings: &mut Vec<ConfigWarning>,
) {
    if !has_bounds {
        return;
    }
    let fix = |x: f64| -> Option<f64> {
        if x < min {
            Some(min)
        } else if x > max {
            Some(max)
        } else {
            None
        }
    };
    match value {
        ConfigValue::Float(x) => {
            if let Some(c) = fix(*x) {
                warnings.push(ConfigWarning {
                    key: key.into(),
                    message: format!("{x} hors bornes [{min}, {max}], corrigé à {c}"),
                    corrected: Some(ConfigValue::Float(c)),
                });
                *x = c;
            }
        }
        ConfigValue::Int(x) => {
            if let Some(c) = fix(*x as f64) {
                warnings.push(ConfigWarning {
                    key: key.into(),
                    message: format!("{x} hors bornes [{min}, {max}], corrigé à {c}"),
                    corrected: Some(ConfigValue::Int(c as i64)),
                });
                *x = c as i64;
            }
        }
        ConfigValue::Floats(xs) => {
            for x in xs.iter_mut() {
                if let Some(c) = fix(*x) {
                    warnings.push(ConfigWarning {
                        key: key.into(),
                        message: format!("{x} hors bornes [{min}, {max}], corrigé à {c}"),
                        corrected: None,
                    });
                    *x = c;
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_rejects_unknown_key() {
        let mut cfg = DynamicPrintConfig::new();
        let err = cfg.set("pas_une_cle", ConfigValue::Bool(true)).unwrap_err();
        assert_eq!(err.code, EngineErrorCode::InvalidConfig);
        assert_eq!(err.subject.as_deref(), Some("pas_une_cle"));
    }

    #[test]
    fn set_rejects_incompatible_type() {
        let mut cfg = DynamicPrintConfig::new();
        // layer_height est un Float, pas un Bool
        assert!(cfg.set("layer_height", ConfigValue::Bool(true)).is_err());
        assert!(cfg.set("layer_height", ConfigValue::Float(0.2)).is_ok());
    }

    #[test]
    fn validate_clampe_aux_bornes() {
        let mut cfg = DynamicPrintConfig::new();
        // layer_height : min 0 (registre)
        cfg.set("layer_height", ConfigValue::Float(-1.0)).unwrap();
        let warnings = cfg.validate();
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].key, "layer_height");
        assert_eq!(cfg.get("layer_height"), Some(&ConfigValue::Float(0.0)));
    }

    #[test]
    fn validate_signale_enum_inconnue() {
        let mut cfg = DynamicPrintConfig::new();
        cfg.set(
            "sparse_infill_pattern",
            ConfigValue::String("spirale_magique".into()),
        )
        .unwrap();
        let warnings = cfg.validate();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("spirale_magique"));
    }

    #[test]
    fn enum_valide_sans_avertissement() {
        let mut cfg = DynamicPrintConfig::new();
        cfg.set(
            "sparse_infill_pattern",
            ConfigValue::String("rectilinear".into()),
        )
        .unwrap();
        assert!(cfg.validate().is_empty());
    }

    #[test]
    fn nullable_filament_override_accepted() {
        let mut cfg = DynamicPrintConfig::new();
        cfg.set(
            "filament_retraction_length",
            ConfigValue::FloatsNullable(vec![Some(0.8), None]),
        )
        .unwrap();
        assert!(cfg.validate().is_empty());
    }

    #[test]
    fn nullable_bools_variant_is_gated_by_registry() {
        let mut cfg = DynamicPrintConfig::new();
        let value = ConfigValue::BoolsNullable(vec![Some(true), None]);
        assert!(cfg.set("filament_wipe", value.clone()).is_ok());
        assert!(cfg.set("filament_soluble", value).is_err());
    }

    #[test]
    fn serde_nullable_bools_and_enums_keep_nil_slots() {
        let raw = serde_json::json!({
            "filament_wipe": [true, null, "nil", false],
            "filament_z_hop_types": ["Normal Lift", null, "nil"],
            "filament_retract_lift_enforce": "nil,Top Only",
        });
        let config: DynamicPrintConfig = serde_json::from_value(raw).unwrap();
        let wipe = ConfigValue::BoolsNullable(vec![Some(true), None, None, Some(false)]);
        assert_eq!(config.get("filament_wipe"), Some(&wipe));
        assert_eq!(
            config.get("filament_z_hop_types"),
            Some(&ConfigValue::Strings(vec![
                "Normal Lift".into(),
                "nil".into(),
                "nil".into()
            ]))
        );
        assert_eq!(
            config.get("filament_retract_lift_enforce"),
            Some(&ConfigValue::Strings(vec!["nil".into(), "Top Only".into()]))
        );
        // Sérialisation JSON : nil → null, puis relecture identique.
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(
            json["filament_wipe"],
            serde_json::json!([true, null, null, false])
        );
        let back: DynamicPrintConfig = serde_json::from_value(json).unwrap();
        assert_eq!(back.0, config.0);
        // Variante seule (untagged) : [bool, null] → BoolsNullable.
        let alone: ConfigValue = serde_json::from_str("[true,null]").unwrap();
        assert_eq!(alone, ConfigValue::BoolsNullable(vec![Some(true), None]));
        // Non nullable : nil refusé.
        let bad = serde_json::json!({ "filament_soluble": [true, null] });
        assert!(serde_json::from_value::<DynamicPrintConfig>(bad).is_err());
        let bad = serde_json::json!({ "extruder_type": ["nil"] });
        assert!(serde_json::from_value::<DynamicPrintConfig>(bad).is_err());
    }

    #[test]
    fn serde_roundtrip_json() {
        let mut cfg = DynamicPrintConfig::new();
        cfg.set("layer_height", ConfigValue::Float(0.2)).unwrap();
        cfg.set("enable_support", ConfigValue::Bool(true)).unwrap();
        let json = serde_json::to_string(&cfg).unwrap();
        let back: DynamicPrintConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.get("layer_height"), cfg.get("layer_height"));
    }

    #[test]
    fn serde_uses_the_key_to_distinguish_float_vectors_from_points() {
        let raw = serde_json::json!({
            "machine_max_acceleration_e": [5000.0, 5000.0],
            "extruder_offset": [[0.0, 0.0]],
        });
        let config: DynamicPrintConfig = serde_json::from_value(raw).unwrap();
        assert_eq!(
            config.get("machine_max_acceleration_e"),
            Some(&ConfigValue::Floats(vec![5000.0, 5000.0]))
        );
        assert_eq!(
            config.get("extruder_offset"),
            Some(&ConfigValue::Points(vec![vec![0.0, 0.0]]))
        );
    }
}
