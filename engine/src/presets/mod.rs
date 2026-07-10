//! Presets : import des profils système et résolution d'héritage.

pub mod import;
mod resolve;

pub use import::{import_profiles, ImportError, ImportOutcome, ImportedPreset, PresetKind};
pub use resolve::{resolve_preset_chain, validate_config};
