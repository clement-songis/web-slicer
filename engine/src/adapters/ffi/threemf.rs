//! Projets 3MF via libslic3r (T014) : lecture scène + config embarquée,
//! écriture compatible OrcaSlicer (FR-044).

use std::collections::BTreeMap;
use std::path::Path;

use crate::api::{
    ConfigValue, DynamicPrintConfig, EngineError, EngineErrorCode, EngineResult, Model,
};
use crate::params::{self, orca_values};

use super::bridge::ffi;
use super::model::{ffi_guard, to_object, to_raw_objects};

pub fn read_project_3mf(path: &Path) -> EngineResult<(Model, DynamicPrintConfig)> {
    let _guard = ffi_guard();
    let raw = ffi::read_project_3mf_raw(&path.to_string_lossy())
        .map_err(|e| EngineError::new(EngineErrorCode::InvalidModel, e.to_string()))?;
    let model = Model {
        objects: raw.objects.into_iter().map(to_object).collect(),
    };
    let config = config_from_orca_json(&raw.config_json)?;
    Ok((model, config))
}

pub fn write_project_3mf(
    model: &Model,
    config: &DynamicPrintConfig,
    out: &Path,
) -> EngineResult<()> {
    let _guard = ffi_guard();
    let raw_objects = to_raw_objects(model);
    let json = config_to_orca_json(config);
    ffi::write_project_3mf_raw(&raw_objects, &json, &out.to_string_lossy())
        .map_err(|e| EngineError::new(EngineErrorCode::Io, e.to_string()))
}

/// JSON {clé → valeur sérialisée Orca} → config typée par le registre.
/// Les clés inconnues sont ignorées (même comportement que handle_legacy).
fn config_from_orca_json(json: &str) -> EngineResult<DynamicPrintConfig> {
    let map: BTreeMap<String, String> = serde_json::from_str(json)
        .map_err(|e| EngineError::new(EngineErrorCode::Internal, e.to_string()))?;
    let mut config = DynamicPrintConfig::new();
    for (key, text) in map {
        let Some(def) = params::get(&key) else {
            continue;
        };
        if let Some(value) = orca_values::parse_orca_value(def, &text) {
            config.0.insert(key, value);
        }
    }
    Ok(config)
}

fn config_to_orca_json(config: &DynamicPrintConfig) -> String {
    let map: BTreeMap<&str, String> = config
        .0
        .iter()
        .map(|(k, v)| (k.as_str(), orca_values::serialize_orca_value(v)))
        .collect();
    serde_json::to_string(&map).expect("map de chaînes toujours sérialisable")
}

#[allow(dead_code)]
fn as_float(v: &ConfigValue) -> Option<f64> {
    match v {
        ConfigValue::Float(x) => Some(*x),
        ConfigValue::Int(i) => Some(*i as f64),
        _ => None,
    }
}
