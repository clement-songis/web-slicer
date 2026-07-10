//! Import des profils système OrcaSlicer (`resources/profiles`) — T036, R5.
//!
//! Structure : chaque vendeur possède un index `<Vendor>.json` référençant des
//! presets de 4 types (`machine_model`, `machine`, `filament`, `process`) via
//! des listes `*_list`. Les presets héritent par le champ `inherits` ;
//! `instantiation:"true"` marque un preset concret (visible), sinon abstrait.
//!
//! Miroir fidèle de `audit/extract_presets_inventory.py` (source de vérité des
//! comptages, SC-002) : on parcourt les index triés, on saute `blacklist`, et
//! pour chaque item de liste existant on charge le preset. Les fichiers absents
//! ou illisibles sont collectés en avertissements (jamais fatals), à l'identique
//! de l'inventaire d'audit.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

/// Type d'un preset système (les 4 catégories des index vendeurs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PresetKind {
    MachineModel,
    Machine,
    Filament,
    Process,
}

impl PresetKind {
    /// Les 4 types dans l'ordre des listes d'index.
    pub const ALL: [PresetKind; 4] = [
        PresetKind::MachineModel,
        PresetKind::Machine,
        PresetKind::Filament,
        PresetKind::Process,
    ];

    /// Clé de la liste correspondante dans l'index vendeur.
    fn list_key(self) -> &'static str {
        match self {
            PresetKind::MachineModel => "machine_model_list",
            PresetKind::Machine => "machine_list",
            PresetKind::Filament => "filament_list",
            PresetKind::Process => "process_list",
        }
    }

    /// Identifiant stable (aligné sur le champ `type` des profils).
    pub fn as_str(self) -> &'static str {
        match self {
            PresetKind::MachineModel => "machine_model",
            PresetKind::Machine => "machine",
            PresetKind::Filament => "filament",
            PresetKind::Process => "process",
        }
    }
}

/// Champs structurels (méta) d'un preset — tout le reste est config.
/// Identique au `META_FIELDS` de l'extracteur d'audit.
const META_FIELDS: &[&str] = &[
    "type",
    "name",
    "inherits",
    "from",
    "setting_id",
    "instantiation",
    "version",
    "url",
    "description",
    "filament_id",
    "printer_model",
    "printer_variant",
    "compatible_printers",
    "compatible_printers_condition",
    "sub_path",
    "family",
    "model_id",
    "machine_tech",
    "bed_model",
    "bed_texture",
    "default_materials",
    "default_bed_type",
    "hotend_model",
    "nozzle_diameter",
];

/// Preset importé depuis un profil vendeur.
#[derive(Debug, Clone)]
pub struct ImportedPreset {
    pub vendor: String,
    pub kind: PresetKind,
    pub name: String,
    pub sub_path: String,
    pub inherits: Option<String>,
    pub from: Option<String>,
    pub setting_id: Option<String>,
    pub filament_id: Option<String>,
    /// `true` si le preset est concret (`instantiation:"true"`).
    pub instantiation: bool,
    pub compatible_printers: Vec<String>,
    /// Surcharges de configuration (méta exclus), au format JSON Orca.
    pub values: Map<String, Value>,
}

/// Résultat d'un import : presets chargés + avertissements non fatals.
#[derive(Debug, Default)]
pub struct ImportOutcome {
    pub presets: Vec<ImportedPreset>,
    pub warnings: Vec<String>,
}

impl ImportOutcome {
    /// Nombre de presets d'un type donné.
    pub fn count(&self, kind: PresetKind) -> usize {
        self.presets.iter().filter(|p| p.kind == kind).count()
    }

    /// Vendeurs distincts rencontrés.
    pub fn vendor_count(&self) -> usize {
        let mut vendors: Vec<&str> = self.presets.iter().map(|p| p.vendor.as_str()).collect();
        vendors.sort_unstable();
        vendors.dedup();
        vendors.len()
    }
}

/// Erreur d'import (seul le répertoire manquant est fatal).
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("répertoire de profils introuvable : {0}")]
    MissingDir(PathBuf),
    #[error("erreur d'entrée/sortie : {0}")]
    Io(#[from] std::io::Error),
}

/// Importe tous les profils système sous `profiles_dir`
/// (`…/resources/profiles`).
pub fn import_profiles(profiles_dir: &Path) -> Result<ImportOutcome, ImportError> {
    if !profiles_dir.is_dir() {
        return Err(ImportError::MissingDir(profiles_dir.to_path_buf()));
    }

    // Index vendeurs : `*.json` à la racine, triés (ordre déterministe).
    let mut index_files: Vec<PathBuf> = std::fs::read_dir(profiles_dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    index_files.sort();

    let mut outcome = ImportOutcome::default();
    for index_path in index_files {
        let vendor = index_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        if vendor == "blacklist" {
            continue;
        }
        let index = match read_json(&index_path) {
            Ok(v) => v,
            Err(e) => {
                outcome
                    .warnings
                    .push(format!("index {vendor} illisible : {e}"));
                continue;
            }
        };
        for kind in PresetKind::ALL {
            let Some(items) = index.get(kind.list_key()).and_then(Value::as_array) else {
                continue;
            };
            for item in items {
                let sub_path = item.get("sub_path").and_then(Value::as_str).unwrap_or("");
                let preset_path = profiles_dir.join(&vendor).join(sub_path);
                if !preset_path.exists() {
                    outcome
                        .warnings
                        .push(format!("{vendor}/{sub_path} : référencé mais absent"));
                    continue;
                }
                match read_json(&preset_path) {
                    Ok(Value::Object(data)) => {
                        outcome
                            .presets
                            .push(build_preset(&vendor, kind, sub_path, data));
                    }
                    Ok(_) => outcome
                        .warnings
                        .push(format!("{vendor}/{sub_path} : objet JSON attendu")),
                    Err(e) => outcome
                        .warnings
                        .push(format!("{vendor}/{sub_path} illisible : {e}")),
                }
            }
        }
    }
    Ok(outcome)
}

fn build_preset(
    vendor: &str,
    kind: PresetKind,
    sub_path: &str,
    data: Map<String, Value>,
) -> ImportedPreset {
    let str_field = |k: &str| data.get(k).and_then(Value::as_str).map(String::from);
    let name = str_field("name").unwrap_or_default();
    // `instantiation` peut être une chaîne ("true"/"false") ou un booléen.
    let instantiation = match data.get("instantiation") {
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => s.eq_ignore_ascii_case("true"),
        _ => false,
    };
    let compatible_printers = data
        .get("compatible_printers")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();
    let values: Map<String, Value> = data
        .iter()
        .filter(|(k, _)| !META_FIELDS.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    ImportedPreset {
        vendor: vendor.to_string(),
        kind,
        name,
        sub_path: sub_path.to_string(),
        inherits: str_field("inherits"),
        from: str_field("from"),
        setting_id: str_field("setting_id"),
        filament_id: str_field("filament_id"),
        instantiation,
        compatible_printers,
        values,
    }
}

/// Lit un JSON en tolérant un BOM UTF-8 et l'UTF-8 invalide (comme
/// `utf-8-sig` + `errors="replace"` de l'extracteur Python).
fn read_json(path: &Path) -> Result<Value, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&bytes);
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    serde_json::from_str(text).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profiles_dir() -> PathBuf {
        PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../vendor/OrcaSlicer/resources/profiles"
        ))
    }

    fn outcome() -> ImportOutcome {
        import_profiles(&profiles_dir()).expect("profils importables")
    }

    /// SC-002 : comptages exacts par type, alignés sur audit/presets_inventory.json.
    #[test]
    fn exact_counts_per_type() {
        let out = outcome();
        assert!(
            out.warnings.is_empty(),
            "aucun avertissement : {:?}",
            out.warnings
        );
        assert_eq!(out.count(PresetKind::MachineModel), 384, "machine_model");
        assert_eq!(out.count(PresetKind::Machine), 1158, "machine");
        assert_eq!(out.count(PresetKind::Filament), 7012, "filament");
        assert_eq!(out.count(PresetKind::Process), 3341, "process");
        assert_eq!(out.presets.len(), 11_895, "total");
        assert_eq!(out.vendor_count(), 65, "65 vendeurs (blacklist exclu)");
    }

    /// Comptages des presets concrets (instantiation=true), audit summary.
    #[test]
    fn instantiated_counts_match_audit() {
        let out = outcome();
        let inst = |k: PresetKind| {
            out.presets
                .iter()
                .filter(|p| p.kind == k && p.instantiation)
                .count()
        };
        assert_eq!(inst(PresetKind::Filament), 5913);
        assert_eq!(inst(PresetKind::Machine), 1001);
        assert_eq!(inst(PresetKind::Process), 2881);
        // les modèles d'imprimante ne sont jamais instanciables
        assert_eq!(inst(PresetKind::MachineModel), 0);
    }

    /// Héritage et compatibilité capturés sur un preset BBL connu.
    #[test]
    fn captures_inheritance_and_metadata() {
        let out = outcome();
        let leaf = out
            .presets
            .iter()
            .find(|p| p.vendor == "BBL" && p.name == "0.20mm Standard @BBL A1")
            .expect("preset BBL 0.20 Standard présent");
        assert_eq!(leaf.kind, PresetKind::Process);
        assert!(leaf.instantiation, "preset concret");
        assert!(leaf.inherits.is_some(), "hérite d'un parent");
        assert!(
            !leaf.compatible_printers.is_empty(),
            "compatibilités listées"
        );
        // les champs méta ne polluent pas les surcharges de config
        assert!(!leaf.values.contains_key("inherits"));
        assert!(!leaf.values.contains_key("name"));
    }

    #[test]
    fn abstract_presets_are_marked_non_instantiable() {
        let out = outcome();
        let common = out
            .presets
            .iter()
            .find(|p| p.name == "fdm_process_common")
            .expect("preset abstrait fdm_process_common présent");
        assert!(!common.instantiation, "preset abstrait");
    }

    #[test]
    fn missing_dir_is_an_error() {
        let err = import_profiles(Path::new("/n/existe/pas")).unwrap_err();
        assert!(matches!(err, ImportError::MissingDir(_)));
    }
}
