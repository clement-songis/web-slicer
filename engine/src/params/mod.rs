//! Registre des paramètres — miroir exhaustif de `PrintConfigDef`
//! (source de vérité : audit/parameters.json, Annexe A de la spec).
//!
//! `registry.rs` est GÉNÉRÉ par `scripts/codegen.sh` et committé ; toute
//! divergence avec l'audit fait échouer `registry_synced_with_audit`.

pub mod legacy;
mod legacy_tables;
pub mod orca_values;
mod registry;

pub use legacy::handle_legacy;
pub use legacy_tables::SOURCE_SHA1 as LEGACY_SOURCE_SHA1;
pub use registry::{REGISTRY, SOURCE_SHA1};

/// Type de valeur d'un paramètre (miroir des `ConfigOptionType` co*).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    Float,
    Floats,
    Int,
    Ints,
    Bool,
    Bools,
    String,
    Strings,
    Percent,
    Percents,
    FloatOrPercent,
    FloatsOrPercents,
    Enum,
    Enums,
    Point,
    Points,
    PointsGroups,
}

/// Origine du paramètre dans PrintConfig.cpp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamGroup {
    /// init_common_params — réglages communs FFF/SLA.
    Common,
    /// init_fff_params — cœur de la parité UI.
    Fff,
    /// init_sla_params — parité de données uniquement (FR-004).
    Sla,
    /// CLI*ConfigDef — actions serveur, jamais montrés comme réglages.
    Cli,
    /// Placeholders G-code, états de slicing (moteur de templates).
    Other,
}

/// Visibilité dans l'UI (FR-005).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mode {
    Simple,
    Advanced,
    Expert,
    Develop,
}

/// Définition complète d'un paramètre (une ligne de l'Annexe A).
#[derive(Debug)]
pub struct ParamDef {
    pub key: &'static str,
    pub kind: ParamKind,
    pub nullable: bool,
    pub group: ParamGroup,
    pub mode: Mode,
    pub category: Option<&'static str>,
    pub label: Option<&'static str>,
    pub tooltip: Option<&'static str>,
    pub sidetext: Option<&'static str>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub enum_values: &'static [&'static str],
    pub enum_labels: &'static [&'static str],
    /// Valeur par défaut sérialisée en JSON (None si non résolue côté audit).
    pub default_json: Option<&'static str>,
    pub readonly: bool,
}

/// Recherche un paramètre par clé (le registre est trié par clé).
pub fn get(key: &str) -> Option<&'static ParamDef> {
    REGISTRY
        .binary_search_by(|d| d.key.cmp(key))
        .ok()
        .map(|i| &REGISTRY[i])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le registre committé correspond exactement à audit/parameters.json.
    #[test]
    fn registry_synced_with_audit() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../audit/parameters.json");
        let bytes = std::fs::read(path).expect("audit/parameters.json lisible");
        let mut hasher = sha1_smol::Sha1::new();
        hasher.update(&bytes);
        assert_eq!(
            hasher.digest().to_string(),
            SOURCE_SHA1,
            "registry.rs périmé : relancer scripts/codegen.sh"
        );
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let count = json["parameters"].as_object().unwrap().len();
        assert_eq!(REGISTRY.len(), count, "comptage exact des paramètres");
    }

    #[test]
    fn registre_trie_et_indexe() {
        assert!(REGISTRY.windows(2).all(|w| w[0].key < w[1].key));
        assert!(get("layer_height").is_some());
        assert!(get("clé_inexistante").is_none());
    }

    #[test]
    fn spot_check_layer_height() {
        let p = get("layer_height").unwrap();
        assert_eq!(p.kind, ParamKind::Float);
        assert_eq!(p.group, ParamGroup::Common);
        assert_eq!(p.category, Some("Quality"));
        assert_eq!(p.min, Some(0.0));
        assert_eq!(p.default_json, Some("0.2"));
    }

    #[test]
    fn spot_check_sparse_infill_pattern() {
        let p = get("sparse_infill_pattern").unwrap();
        assert_eq!(p.kind, ParamKind::Enum);
        assert!(p.enum_values.contains(&"rectilinear"));
        assert!(p.enum_values.contains(&"zigzag"));
        assert_eq!(p.enum_values.len(), p.enum_labels.len());
        // défaut C++ ipCrossHatch résolu vers la valeur config
        assert_eq!(p.default_json, Some("\"crosshatch\""));
    }

    #[test]
    fn spot_check_host_type() {
        let p = get("host_type").unwrap();
        assert_eq!(
            p.enum_values.len(),
            16,
            "16 hôtes d'impression (v1: moonraker)"
        );
        assert!(p.enum_values.contains(&"moonraker"));
    }

    #[test]
    fn spot_check_machine_max_axes_generees() {
        // les 12 clés de la boucle AxisDefault (non littérales dans le C++)
        for axis in ["x", "y", "z", "e"] {
            for kind in ["speed", "acceleration", "jerk"] {
                let key = format!("machine_max_{kind}_{axis}");
                let p = get(&key).unwrap_or_else(|| panic!("{key} absente"));
                assert_eq!(p.kind, ParamKind::Floats);
                assert_eq!(p.mode, Mode::Simple);
            }
        }
        assert_eq!(
            get("machine_max_speed_x").unwrap().default_json,
            Some("[500, 200]")
        );
    }

    #[test]
    fn spot_check_nullable_filament_override() {
        let p = get("filament_retraction_length").unwrap();
        assert!(p.nullable);
        assert_eq!(p.kind, ParamKind::Floats);
    }

    #[test]
    fn groups_exhaustive() {
        let fff_common_sla = REGISTRY
            .iter()
            .filter(|p| {
                matches!(
                    p.group,
                    ParamGroup::Fff | ParamGroup::Common | ParamGroup::Sla
                )
            })
            .count();
        // parité exacte avec le runtime C++ (vérifiée par dump-config --keys)
        assert_eq!(fff_common_sla, 751);
    }
}
