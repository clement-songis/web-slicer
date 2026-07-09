//! Presets : résolution d'héritage et (T036+) import des profils système.

mod resolve;

pub use resolve::{resolve_preset_chain, validate_config};
