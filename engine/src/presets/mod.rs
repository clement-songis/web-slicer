//! Presets : import des profils système et résolution d'héritage.

pub mod import;
mod resolve;

pub use import::{import_profiles, ImportError, ImportOutcome, ImportedPreset, PresetKind};
pub use resolve::{
    canonical_orca_json, config_value_to_orca_json, resolve_preset_chain, validate_config,
    PresetIndex,
};
