//! Seed des presets système (T038). Transformation **pure** des presets
//! importés par le moteur (`engine::presets`, aucune E/S ici) en entités du
//! domaine, puis re-seed via le trait `Storage`. La lecture des profils sur
//! disque est faite en amont (composition root / handler admin) et le résultat
//! passé ici — le domaine reste sans dépendance filesystem.

use engine::presets::{ImportedPreset, PresetKind as EngineKind};
use serde_json::Value;

use super::{Preset, PresetId, PresetKind, PresetOrigin, Storage, StorageResult};

fn map_kind(kind: EngineKind) -> PresetKind {
    match kind {
        EngineKind::MachineModel => PresetKind::MachineModel,
        EngineKind::Machine => PresetKind::Machine,
        EngineKind::Filament => PresetKind::Filament,
        EngineKind::Process => PresetKind::Process,
    }
}

/// Convertit un preset importé en entité système du domaine (id neuf).
pub fn to_system_preset(p: &ImportedPreset) -> Preset {
    let compatible_printers = if p.compatible_printers.is_empty() {
        None
    } else {
        Some(Value::Array(
            p.compatible_printers
                .iter()
                .map(|s| Value::String(s.clone()))
                .collect(),
        ))
    };
    Preset {
        id: PresetId::new(),
        kind: map_kind(p.kind),
        name: p.name.clone(),
        origin: PresetOrigin::System,
        user_id: None,
        vendor: (!p.vendor.is_empty()).then(|| p.vendor.clone()),
        inherits: p.inherits.clone(),
        instantiation: p.instantiation,
        setting_id: p.setting_id.clone(),
        filament_id: p.filament_id.clone(),
        compatible_printers,
        values: Value::Object(p.values.clone()),
    }
}

/// Re-seed l'ensemble des presets système depuis un lot importé. Idempotent
/// (remplace tous les presets système) et **sans toucher aux presets
/// utilisateur** (garanti par `PresetRepo::reseed_system`). Renvoie le nombre
/// de presets système écrits.
pub async fn reseed_system_presets(
    storage: &dyn Storage,
    imported: &[ImportedPreset],
) -> StorageResult<u64> {
    let presets = imported.iter().map(to_system_preset).collect();
    storage.presets().reseed_system(presets).await
}
