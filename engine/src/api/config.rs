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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DynamicPrintConfig(pub BTreeMap<String, ConfigValue>);

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
    fn serde_roundtrip_json() {
        let mut cfg = DynamicPrintConfig::new();
        cfg.set("layer_height", ConfigValue::Float(0.2)).unwrap();
        cfg.set("enable_support", ConfigValue::Bool(true)).unwrap();
        let json = serde_json::to_string(&cfg).unwrap();
        let back: DynamicPrintConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.get("layer_height"), cfg.get("layer_height"));
    }
}
