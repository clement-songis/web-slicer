//! Implémentations du trait `Storage` (contrat storage-trait.md). Sélection
//! runtime par `DATABASE_URL` (sqlite:// par défaut, postgres:// via feature).

pub mod sqlite;

#[cfg(feature = "postgres")]
pub mod postgres;

use std::collections::HashSet;

use serde_json::Value;
use uuid::Uuid;

/// Extrait les UUID d'un document `active_presets`. Les références peuvent être
/// directement sous `printer`/`process` ou dans le tableau `filaments` ; le
/// parcours récursif évite aussi de laisser une référence morte si le document
/// évolue sans que l'adaptateur de stockage soit modifié.
pub(crate) fn collect_uuid_strings(value: &Value, ids: &mut HashSet<Uuid>) {
    match value {
        Value::String(s) => {
            if let Ok(id) = Uuid::parse_str(s) {
                ids.insert(id);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_uuid_strings(value, ids);
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                collect_uuid_strings(value, ids);
            }
        }
        _ => {}
    }
}

pub(crate) fn json_references_id(value: &Value, id: Uuid) -> bool {
    let mut ids = HashSet::new();
    collect_uuid_strings(value, &mut ids);
    ids.contains(&id)
}
