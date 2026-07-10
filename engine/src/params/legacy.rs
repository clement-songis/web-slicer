//! Conversion des clés/valeurs héritées à l'import (miroir fidèle de
//! `PrintConfigDef::handle_legacy`, PrintConfig.cpp).
//!
//! Deux étages :
//!   1. renommages simples et clés ignorées → tables **générées** depuis
//!      `audit/legacy_keys.json` (`legacy_tables.rs`) ;
//!   2. transformations conditionnées par la valeur → portées à la main ici
//!      (leurs conditions ne figurent pas dans l'audit, qui n'en garde que
//!      l'assignement + un SHA-1 pour la détection de dérive).
//!
//! Comportement final identique à Orca : après conversion, une clé de
//! l'ensemble ignoré **ou** absente du registre est abandonnée (l'import la
//! laisse tomber, comme le fait le chargement desktop).

use std::collections::BTreeSet;

use super::legacy_tables::{IGNORED, RENAMES};

/// Convertit une paire (clé, valeur) héritée.
///
/// - `Some((key, value))` : clé (éventuellement renommée) et valeur
///   (éventuellement transformée) à conserver ;
/// - `None` : clé obsolète, ignorée ou inconnue du registre → à abandonner.
pub fn handle_legacy(key: &str, value: &str) -> Option<(String, String)> {
    let mut key = key.to_string();
    let mut value = value.to_string();

    apply_value_rules(&mut key, &mut value);

    // Renommages inconditionnels (aucun recouvrement avec les règles ci-dessus).
    if !key.is_empty() {
        if let Ok(i) = RENAMES.binary_search_by(|(old, _)| (*old).cmp(key.as_str())) {
            key = RENAMES[i].1.to_string();
        }
    }

    // Clé effacée, ignorée, ou absente du registre → abandonnée.
    if key.is_empty() || IGNORED.binary_search(&key.as_str()).is_ok() || super::get(&key).is_none()
    {
        return None;
    }
    Some((key, value))
}

/// Porte les branches de `handle_legacy` conditionnées par la valeur. Vider
/// `key` signale l'abandon (opt_key = "" côté C++).
fn apply_value_rules(key: &mut String, value: &mut String) {
    match key.as_str() {
        "curr_bed_type" => {
            if value == "SuperTack Plate" {
                *value = "Supertack Plate".into();
            }
        }
        // Sélecteurs de filament par fonction : renommage + défaut « 1 » → « 0 ».
        "infill_extruder" | "sparse_infill_filament" => {
            set_filament_id(key, value, "sparse_infill_filament_id")
        }
        "solid_infill_extruder" | "solid_infill_filament" => {
            set_filament_id(key, value, "internal_solid_filament_id")
        }
        "top_solid_infill_filament" => set_filament_id(key, value, "top_surface_filament_id"),
        "bottom_solid_infill_filament" => set_filament_id(key, value, "bottom_surface_filament_id"),
        "perimeter_extruder" | "wall_filament" | "wall_filament_id" => {
            set_filament_id(key, value, "outer_wall_filament_id")
        }
        "inner_wall_filament" => set_filament_id(key, value, "inner_wall_filament_id"),
        "outer_wall_filament" => set_filament_id(key, value, "outer_wall_filament_id"),
        // Vitesses/hauteur jadis exprimées en % : désormais absolues → effacées.
        "initial_layer_print_height"
        | "initial_layer_speed"
        | "internal_solid_infill_speed"
        | "top_surface_speed"
        | "support_interface_speed"
        | "outer_wall_speed"
        | "support_object_xy_distance" => {
            if value.contains('%') {
                key.clear();
            }
        }
        "timelapse_type" => {
            if value == "2" {
                *value = "0".into();
            }
        }
        "support_type" => match value.as_str() {
            "normal" => *value = "normal(manual)".into(),
            "tree" => *value = "tree(manual)".into(),
            "hybrid(auto)" => *value = "tree(auto)".into(),
            _ => {}
        },
        "support_base_pattern" => {
            if value == "none" {
                *value = "hollow".into();
            }
        }
        "different_settings_to_system" => rewrite_different_settings(value),
        "overhang_fan_threshold" => {
            if value == "5%" {
                *value = "10%".into();
            }
        }
        "wall_infill_order" => {
            *key = "wall_sequence".into();
            let mapped = match value.as_str() {
                "inner wall/outer wall/infill" | "infill/inner wall/outer wall" => {
                    Some("inner wall/outer wall")
                }
                "outer wall/inner wall/infill" | "infill/outer wall/inner wall" => {
                    Some("outer wall/inner wall")
                }
                "inner-outer-inner wall/infill" => Some("inner-outer-inner wall"),
                _ => None,
            };
            if let Some(v) = mapped {
                *value = v.into();
            }
        }
        "nozzle_volume_type"
        | "default_nozzle_volume_type"
        | "printer_extruder_variant"
        | "print_extruder_variant"
        | "filament_extruder_variant"
        | "extruder_variant_list" => {
            *value = value
                .replace("Normal", "Standard")
                .replace("Big Traffic", "High Flow");
        }
        "extruder_type" => *value = value.replace("DirectDrive", "Direct Drive"),
        "enable_power_loss_recovery" => {
            if value == "1" || value.eq_ignore_ascii_case("true") {
                *value = "enable".into();
            } else if value == "0" || value.eq_ignore_ascii_case("false") {
                *value = "disable".into();
            }
        }
        "ensure_vertical_shell_thickness" => {
            if value == "1" {
                *value = "ensure_all".into();
            } else if value == "0" {
                *value = "ensure_moderate".into();
            }
        }
        "rotate_solid_infill_direction" => {
            *key = "solid_infill_rotate_template".into();
            if value == "1" {
                *value = "0,90".into();
            } else if value == "0" {
                *value = "0".into();
            }
        }
        "top_one_wall_type" => {
            if value != "none" {
                *key = "only_one_wall_top".into();
                *value = "1".into();
            }
        }
        "ironing_angle" => {
            if value.starts_with('-') {
                *value = "0".into();
            }
        }
        "draft_shield" => {
            if value == "limited" {
                *value = "disabled".into();
            }
        }
        "sparse_infill_pattern"
        | "top_surface_pattern"
        | "bottom_surface_pattern"
        | "internal_solid_infill_pattern"
        | "ironing_pattern"
        | "support_ironing_pattern" => {
            if value == "zig-zag" {
                *value = "rectilinear".into();
            }
        }
        "filament_map_mode" => {
            if value == "Auto" {
                *value = "Auto For Flush".into();
            }
        }
        "filament_type" => rewrite_filament_type(value),
        "prime_tower_rib_wall" => {
            if value == "1" {
                *key = "wipe_tower_wall_type".into();
                *value = "rib".into();
            } else {
                key.clear();
            }
        }
        "wall_direction" if value == "auto" => *value = "ccw".into(),
        _ => {}
    }
}

fn set_filament_id(key: &mut String, value: &mut String, new_key: &str) {
    *key = new_key.into();
    if value == "1" {
        *value = "0".into();
    }
}

/// `different_settings_to_system` : la valeur liste des clés `;`-séparées ; on
/// renomme chacune via `handle_legacy` et on répercute dans la chaîne d'origine.
fn rewrite_different_settings(value: &mut String) {
    let dequoted = value.replace('"', "");
    // std::set côté C++ : dédupliqué et trié (ordre déterministe des remplacements).
    let keys: BTreeSet<&str> = dequoted.split(';').filter(|s| !s.is_empty()).collect();
    for split_key in keys {
        let new_key = handle_legacy(split_key, "")
            .map(|(k, _)| k)
            .unwrap_or_default();
        if new_key != split_key {
            *value = value.replace(split_key, &new_key);
        }
    }
}

/// `filament_type` : normalise l'ancien libellé `ASA-Aero` → `ASA-AERO`, en
/// reconstruisant la liste guillemetée seulement si une valeur a changé.
fn rewrite_filament_type(value: &mut String) {
    let mut rebuild = false;
    let mut tokens = Vec::new();
    for raw in value.split(';') {
        let mut tok = raw;
        if tok.len() >= 2 && tok.starts_with('"') && tok.ends_with('"') {
            tok = &tok[1..tok.len() - 1];
        }
        if tok == "ASA-Aero" {
            rebuild = true;
            tokens.push("ASA-AERO".to_string());
        } else {
            tokens.push(tok.to_string());
        }
    }
    if rebuild {
        *value = tokens
            .iter()
            .map(|t| format!("\"{t}\""))
            .collect::<Vec<_>>()
            .join(";");
    }
}

#[cfg(test)]
mod tests {
    use super::super::legacy_tables::SOURCE_SHA1;
    use super::*;

    fn conv(key: &str, value: &str) -> Option<(String, String)> {
        handle_legacy(key, value)
    }

    /// Les tables générées correspondent exactement à audit/legacy_keys.json.
    #[test]
    fn legacy_tables_synced_with_audit() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../audit/legacy_keys.json");
        let bytes = std::fs::read(path).expect("audit/legacy_keys.json lisible");
        let mut hasher = sha1_smol::Sha1::new();
        hasher.update(&bytes);
        assert_eq!(
            hasher.digest().to_string(),
            SOURCE_SHA1,
            "legacy_tables.rs périmé : relancer audit/generate_legacy_rs.py"
        );
        assert_eq!(RENAMES.len(), 28, "28 renommages (audit)");
        assert_eq!(IGNORED.len(), 44, "44 clés ignorées (audit)");
    }

    #[test]
    fn tables_are_sorted_for_binary_search() {
        assert!(RENAMES.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(IGNORED.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn simple_renames_apply() {
        assert_eq!(
            conv("enable_wipe_tower", "1"),
            Some(("enable_prime_tower".into(), "1".into()))
        );
        assert_eq!(
            conv("support_material_angle", "45"),
            Some(("support_angle".into(), "45".into()))
        );
        assert_eq!(
            conv("tool_change_gcode", "G1"),
            Some(("change_filament_gcode".into(), "G1".into()))
        );
    }

    #[test]
    fn filament_selectors_rename_and_map_default() {
        assert_eq!(
            conv("sparse_infill_filament", "1"),
            Some(("sparse_infill_filament_id".into(), "0".into()))
        );
        assert_eq!(
            conv("perimeter_extruder", "3"),
            Some(("outer_wall_filament_id".into(), "3".into())),
            "valeur explicite > 1 conservée"
        );
    }

    #[test]
    fn value_conditioned_transforms() {
        assert_eq!(
            conv("support_type", "normal"),
            Some(("support_type".into(), "normal(manual)".into()))
        );
        assert_eq!(
            conv("extruder_type", "DirectDrive"),
            Some(("extruder_type".into(), "Direct Drive".into()))
        );
        assert_eq!(
            conv("enable_power_loss_recovery", "true"),
            Some(("enable_power_loss_recovery".into(), "enable".into()))
        );
        // Motif "zig-zag" migré, sinon inchangé.
        assert_eq!(
            conv("sparse_infill_pattern", "zig-zag"),
            Some(("sparse_infill_pattern".into(), "rectilinear".into()))
        );
        assert_eq!(
            conv("sparse_infill_pattern", "grid"),
            Some(("sparse_infill_pattern".into(), "grid".into()))
        );
    }

    #[test]
    fn conditional_rename_prime_tower_rib_wall() {
        assert_eq!(
            conv("prime_tower_rib_wall", "1"),
            Some(("wipe_tower_wall_type".into(), "rib".into()))
        );
        assert_eq!(
            conv("prime_tower_rib_wall", "0"),
            None,
            "effacée hors « 1 »"
        );
    }

    #[test]
    fn percent_speeds_are_dropped() {
        assert_eq!(conv("outer_wall_speed", "80%"), None);
    }

    #[test]
    fn ignored_and_unknown_keys_are_dropped() {
        assert_eq!(conv("acceleration", "1000"), None, "clé ignorée");
        assert_eq!(conv("clé_totalement_inconnue", "x"), None, "hors registre");
    }

    #[test]
    fn known_current_key_passes_through() {
        assert_eq!(
            conv("layer_height", "0.2"),
            Some(("layer_height".into(), "0.2".into()))
        );
    }

    #[test]
    fn different_settings_rewrites_embedded_keys() {
        // « support_material_angle » renommée dans la liste ; « acceleration »
        // (ignorée) remplacée par vide.
        let (key, value) = conv(
            "different_settings_to_system",
            "support_material_angle;layer_height",
        )
        .unwrap();
        assert_eq!(key, "different_settings_to_system");
        assert!(
            value.contains("support_angle"),
            "renommage répercuté : {value}"
        );
        assert!(!value.contains("support_material_angle"));
    }
}
