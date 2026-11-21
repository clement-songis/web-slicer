//! Seed des presets système (T038). Transformation **pure** des presets
//! importés par le moteur (`engine::presets`, aucune E/S ici) en entités du
//! domaine, puis re-seed via le trait `Storage`. La lecture des profils sur
//! disque est faite en amont (composition root / handler admin) et le résultat
//! passé ici — le domaine reste sans dépendance filesystem.

use std::sync::atomic::{AtomicU64, Ordering};

use engine::presets::{ImportedPreset, PresetKind as EngineKind};
use serde_json::Value;

use super::{Preset, PresetId, PresetKind, PresetOrigin, Storage, StorageResult};

/// Génération des presets système : incrémentée par chaque `reseed_system`
/// abouti (tous adaptateurs). Les caches de presets système (listes
/// compatibles, `http::preset_cache`) la relisent avant chaque usage et se
/// reconstruisent si elle a changé. Compteur de processus : partagé par toutes
/// les bases d'un même processus (tests), une incrémentation de trop ne coûte
/// qu'une reconstruction.
static SYSTEM_PRESETS_GENERATION: AtomicU64 = AtomicU64::new(0);

/// Génération courante des presets système (voir `bump_system_generation`).
pub fn system_generation() -> u64 {
    SYSTEM_PRESETS_GENERATION.load(Ordering::Acquire)
}

/// À appeler après tout changement validé des presets système.
pub fn bump_system_generation() {
    SYSTEM_PRESETS_GENERATION.fetch_add(1, Ordering::AcqRel);
}

fn map_kind(kind: EngineKind) -> PresetKind {
    match kind {
        EngineKind::MachineModel => PresetKind::MachineModel,
        EngineKind::Machine => PresetKind::Machine,
        EngineKind::Filament => PresetKind::Filament,
        EngineKind::Process => PresetKind::Process,
    }
}

/// Convertit un preset importé en entité système du domaine. L'UUID est
/// provisoire : lors d'un reseed, les adaptateurs conservent l'UUID de la ligne
/// ayant la même identité naturelle `(kind, name, vendor)` afin de préserver les
/// références des imprimantes et des projets.
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
        renamed_from: p.renamed_from.clone(),
        compatible_printers,
        values: Value::Object(p.values.clone()),
    }
}

/// Re-seed l'ensemble des presets système depuis un lot importé. La
/// réconciliation est transactionnelle et conserve les UUID des identités déjà
/// présentes, sans toucher aux presets utilisateur. Renvoie le nombre de
/// presets du lot écrits.
pub async fn reseed_system_presets(
    storage: &dyn Storage,
    imported: &[ImportedPreset],
) -> StorageResult<u64> {
    let presets = imported.iter().map(to_system_preset).collect();
    storage.presets().reseed_system(presets).await
}
