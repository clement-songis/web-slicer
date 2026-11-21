//! Réglages modifiés du projet (lot 40, audit D1).
//!
//! Orca garde les modifications non enregistrées d'un preset dans le projet :
//! le preset est affiché « (modified) », `Save project` écrit la config
//! **éditée** (`PresetBundle::full_config`) dans `project_settings.config`,
//! et la réouverture sélectionne le preset nommé puis lui réapplique les
//! valeurs du projet (`PresetCollection::load_external_preset`,
//! `LoadAndSelect::Always` → preset « dirty »).
//!
//! Ici ces modifications sont des **surcharges** portées par le document
//! `active_presets` du projet, clé `overrides` (même forme que le brouillon
//! de tranchage `SliceDraft`). La config effective est
//! presets résolus + surcharges (`slice::resolve_active_config`), puis les
//! réglages projet / plateau.

use engine::api::DynamicPrintConfig;
use serde_json::{json, Map, Value};

use crate::http::dto::SliceDraft;
use crate::http::error::{ApiError, ApiResult};

/// Clé des surcharges dans `active_presets`.
pub(crate) const OVERRIDES_KEY: &str = "overrides";

/// Clés jamais reprises en surcharge depuis un 3MF : identité du preset
/// (héritage, compatibilités, noms — portés par le preset sélectionné) et
/// connexion à l'imprimante, qu'Orca ignore aussi à l'import
/// (`load_external_preset` : « ignore print connection info from project »).
const NOT_OVERRIDABLE: &[&str] = &[
    "inherits",
    "compatible_printers",
    "compatible_printers_condition",
    "compatible_prints",
    "compatible_prints_condition",
    "print_settings_id",
    "printer_settings_id",
    "printer_model",
    "printer_variant",
    "upward_compatible_machine",
    "print_host",
    "print_host_webui",
    "printhost_apikey",
    "printhost_cafile",
    "printhost_user",
    "printhost_password",
    "printhost_port",
];

fn invalid(message: String) -> ApiError {
    ApiError::validation(message, json!({ "field": "active_presets.overrides" }))
}

/// Surcharges d'un document `active_presets` (absentes ou `null` → `None`).
/// Forme illisible → 422 `field = active_presets.overrides`.
pub(crate) fn overrides_of(active: &Value) -> ApiResult<Option<SliceDraft>> {
    match active.get(OVERRIDES_KEY) {
        None | Some(Value::Null) => Ok(None),
        Some(raw) => serde_json::from_value::<SliceDraft>(raw.clone())
            .map(Some)
            .map_err(|e| invalid(format!("active_presets.overrides illisible : {e}"))),
    }
}

/// Le même document sans ses surcharges (brouillon de l'éditeur fourni : il
/// les remplace au lieu de s'y ajouter).
pub(crate) fn without_overrides(active: &Value) -> Value {
    let mut out = active.clone();
    if let Value::Object(map) = &mut out {
        map.remove(OVERRIDES_KEY);
    }
    out
}

/// Surcharges sans entrées vides (`None` si plus rien) : objets vides
/// retirés, filaments de fin sans modification tronqués.
pub(crate) fn compact(draft: SliceDraft) -> Option<SliceDraft> {
    let keep = |m: Option<Map<String, Value>>| m.filter(|m| !m.is_empty());
    let mut filaments: Vec<Option<Map<String, Value>>> = draft
        .filaments
        .unwrap_or_default()
        .into_iter()
        .map(keep)
        .collect();
    while filaments.last().is_some_and(Option::is_none) {
        filaments.pop();
    }
    let out = SliceDraft {
        process: keep(draft.process),
        printer: keep(draft.printer),
        filaments: (!filaments.is_empty()).then_some(filaments),
    };
    (out != SliceDraft::default()).then_some(out)
}

/// Normalise `active_presets` avant enregistrement (`PUT` / `PATCH`) :
/// surcharges vérifiées comme un brouillon de tranchage (`apply_draft` :
/// valeur illisible ou `null` sur option non nullable → 422) puis compactées
/// (`overrides` vide → clé retirée). Un document non objet est laissé tel
/// quel (tolérance historique).
pub(crate) fn normalize_active_presets(active: Value) -> ApiResult<Value> {
    let Value::Object(mut map) = active else {
        return Ok(active);
    };
    let Some(raw) = map.remove(OVERRIDES_KEY) else {
        return Ok(Value::Object(map));
    };
    if raw.is_null() {
        return Ok(Value::Object(map));
    }
    let draft: SliceDraft = serde_json::from_value(raw)
        .map_err(|e| invalid(format!("active_presets.overrides illisible : {e}")))?;
    super::slice::apply_draft(&mut DynamicPrintConfig::new(), &draft)?;
    if let Some(draft) = compact(draft) {
        map.insert(
            OVERRIDES_KEY.into(),
            serde_json::to_value(draft).map_err(|_| ApiError::internal())?,
        );
    }
    Ok(Value::Object(map))
}

/// Forme comparable d'une valeur : nombres en flottants (`1` = `1.0`),
/// listes de chaînes jointes par « , » et, hors texte multiligne (G-code),
/// espaces autour des virgules retirés (« 48x48/PNG, 300x300/PNG » lu en
/// vecteur puis rejoint = « 48x48/PNG,300x300/PNG » d'Orca).
fn canonical(value: &engine::api::ConfigValue) -> Value {
    fn floats(value: Value) -> Value {
        match value {
            Value::Number(n) => n.as_f64().map_or(Value::Number(n), Value::from),
            Value::Array(items) => Value::Array(items.into_iter().map(floats).collect()),
            other => other,
        }
    }
    let text = |s: String| {
        if s.contains('\n') {
            Value::String(s)
        } else {
            Value::String(s.split(',').map(str::trim).collect::<Vec<_>>().join(","))
        }
    };
    match value {
        engine::api::ConfigValue::String(s) => text(s.clone()),
        engine::api::ConfigValue::Strings(v) => text(v.join(",")),
        other => floats(super::presets::config_value_to_json(other)),
    }
}

/// Valeur « vide » (chaîne vide, vecteur vide ou de chaînes vides / `nil`) :
/// ce que le lecteur 3MF crée pour une clé que le projet ne value pas.
fn is_blank(value: &engine::api::ConfigValue) -> bool {
    match value {
        engine::api::ConfigValue::String(s) => s.is_empty(),
        engine::api::ConfigValue::Strings(v) => v.iter().all(|s| s.is_empty() || s == "nil"),
        other => super::presets::config_value_to_json(other)
            .as_array()
            .is_some_and(|items| items.iter().all(Value::is_null)),
    }
}

/// Surcharges d'un projet 3MF par rapport aux presets retenus à l'ouverture
/// (`presets` : config résolue sans surcharge), comme Orca qui sélectionne
/// le preset nommé puis lui applique les valeurs du projet : chaque option
/// process (`Preset::print_options()`) et, si l'imprimante est posée,
/// machine (`Preset::printer_options()` = options imprimante + limites
/// machine + options d'extrudeur) dont la valeur diffère de celle des presets
/// (défaut du registre à défaut). Une clé que ni les presets ni le registre
/// ne valuent n'est retenue que si le projet la déclare modifiée
/// (`different_settings_to_system`, `process_listed` / `printer_listed`) :
/// sinon sa valeur est le défaut d'Orca, que le moteur applique déjà.
pub(crate) fn imported_overrides(
    project: &DynamicPrintConfig,
    presets: &DynamicPrintConfig,
    with_printer: bool,
    process_listed: &[String],
    printer_listed: &[String],
) -> Option<SliceDraft> {
    let pick = |keys: &[&str], listed: &[String]| -> Option<Map<String, Value>> {
        modified_values(project, presets, keys, listed, &[])
    };
    compact(SliceDraft {
        process: pick(&ORCA_PRINT_OPTIONS, process_listed),
        printer: if with_printer {
            pick(&ORCA_PRINTER_OPTIONS, printer_listed)
        } else {
            None
        },
        filaments: None,
    })
}

/// Valeurs de `project` (options `keys`, hors `NOT_OVERRIDABLE` et
/// `excluded`) qui diffèrent de `presets` (défaut du registre à défaut) ;
/// une clé que ni le preset ni le registre ne valuent n'est retenue que si le
/// projet la déclare modifiée (`listed`). None si rien ne diffère.
pub(crate) fn modified_values(
    project: &DynamicPrintConfig,
    presets: &DynamicPrintConfig,
    keys: &[&str],
    listed: &[String],
    excluded: &[&str],
) -> Option<Map<String, Value>> {
    let mut base = presets.clone();
    super::presets::fill_registry_defaults(&mut base);
    let mut out = Map::new();
    for key in keys {
        if NOT_OVERRIDABLE.contains(key) || excluded.contains(key) {
            continue;
        }
        let Some(value) = project.0.get(*key) else {
            continue;
        };
        let modified = match base.0.get(*key) {
            Some(current) => canonical(current) != canonical(value),
            None => !is_blank(value) && listed.iter().any(|k| k == key),
        };
        if modified {
            out.insert(
                (*key).to_string(),
                super::presets::config_value_to_json(value),
            );
        }
    }
    (!out.is_empty()).then_some(out)
}

/// `different_settings_to_system` du `project_settings.config` exporté
/// (`PresetBundle::full_fff_config` : [process, filaments…, imprimante],
/// chaque entrée = clés modifiées séparées par « ; ») : clés surchargées de
/// chaque preset ajoutées aux entrées déjà portées par la config (projet
/// importé). Sans surcharge, la config est laissée intacte.
pub(crate) fn apply_different_settings(
    config: &mut DynamicPrintConfig,
    overrides: Option<&SliceDraft>,
) {
    let Some(overrides) = overrides else {
        return;
    };
    let filaments = match config.0.get("filament_settings_id") {
        Some(engine::api::ConfigValue::Strings(v)) => v.len(),
        _ => 0,
    }
    .max(overrides.filaments.as_ref().map_or(0, Vec::len));
    let mut slots: Vec<Vec<String>> = vec![Vec::new(); filaments + 2];
    if let Some(engine::api::ConfigValue::Strings(carried)) =
        config.0.get("different_settings_to_system")
    {
        if carried.len() > slots.len() {
            slots.resize(carried.len(), Vec::new());
        }
        for (slot, entry) in slots.iter_mut().zip(carried) {
            slot.extend(
                entry
                    .split(';')
                    .map(|k| k.trim().trim_matches('"'))
                    .filter(|k| !k.is_empty())
                    .map(str::to_string),
            );
        }
    }
    let last = slots.len() - 1;
    let mut add = |index: usize, keys: Option<&Map<String, Value>>| {
        if let Some(keys) = keys {
            slots[index].extend(keys.keys().cloned());
        }
    };
    add(0, overrides.process.as_ref());
    for (i, keys) in overrides.filaments.iter().flatten().enumerate() {
        add(1 + i, keys.as_ref());
    }
    add(last, overrides.printer.as_ref());
    let values = slots
        .into_iter()
        .map(|mut keys| {
            keys.sort();
            keys.dedup();
            keys.join(";")
        })
        .collect();
    config.0.insert(
        "different_settings_to_system".into(),
        engine::api::ConfigValue::Strings(values),
    );
}

/// `Preset::print_options()` d'Orca 2.4.1 (libslic3r/Preset.cpp,
/// `s_Preset_print_options`), extrait de `$ORCA_SRC` (lot 40).
pub(crate) const ORCA_PRINT_OPTIONS: [&str; 355] = [
    "layer_height",
    "initial_layer_print_height",
    "wall_loops",
    "alternate_extra_wall",
    "slice_closing_radius",
    "spiral_mode",
    "spiral_mode_smooth",
    "spiral_mode_max_xy_smoothing",
    "spiral_starting_flow_ratio",
    "spiral_finishing_flow_ratio",
    "slicing_mode",
    "top_shell_layers",
    "top_shell_thickness",
    "top_surface_density",
    "bottom_surface_density",
    "bottom_shell_layers",
    "bottom_shell_thickness",
    "extra_perimeters_on_overhangs",
    "ensure_vertical_shell_thickness",
    "reduce_crossing_wall",
    "detect_thin_wall",
    "detect_overhang_wall",
    "overhang_reverse",
    "overhang_reverse_threshold",
    "overhang_reverse_internal_only",
    "wall_direction",
    "seam_position",
    "staggered_inner_seams",
    "wall_sequence",
    "is_infill_first",
    "sparse_infill_density",
    "fill_multiline",
    "gyroid_optimized",
    "sparse_infill_pattern",
    "lateral_lattice_angle_1",
    "lateral_lattice_angle_2",
    "infill_overhang_angle",
    "lightning_overhang_angle",
    "lightning_prune_angle",
    "lightning_straightening_angle",
    "top_surface_pattern",
    "bottom_surface_pattern",
    "infill_direction",
    "solid_infill_direction",
    "counterbore_hole_bridging",
    "infill_shift_step",
    "sparse_infill_rotate_template",
    "solid_infill_rotate_template",
    "symmetric_infill_y_axis",
    "skeleton_infill_density",
    "infill_lock_depth",
    "skin_infill_depth",
    "skin_infill_density",
    "align_infill_direction_to_model",
    "extra_solid_infills",
    "minimum_sparse_infill_area",
    "reduce_infill_retraction",
    "internal_solid_infill_pattern",
    "gap_fill_target",
    "ironing_type",
    "ironing_pattern",
    "ironing_flow",
    "ironing_speed",
    "ironing_spacing",
    "ironing_angle",
    "ironing_angle_fixed",
    "ironing_inset",
    "support_ironing",
    "support_ironing_pattern",
    "support_ironing_flow",
    "support_ironing_spacing",
    "max_travel_detour_distance",
    "fuzzy_skin",
    "fuzzy_skin_thickness",
    "fuzzy_skin_point_distance",
    "fuzzy_skin_first_layer",
    "fuzzy_skin_noise_type",
    "fuzzy_skin_mode",
    "fuzzy_skin_scale",
    "fuzzy_skin_octaves",
    "fuzzy_skin_persistence",
    "fuzzy_skin_ripples_per_layer",
    "fuzzy_skin_ripple_offset",
    "fuzzy_skin_layers_between_ripple_offset",
    "max_volumetric_extrusion_rate_slope",
    "max_volumetric_extrusion_rate_slope_segment_length",
    "extrusion_rate_smoothing_external_perimeter_only",
    "inner_wall_speed",
    "outer_wall_speed",
    "sparse_infill_speed",
    "internal_solid_infill_speed",
    "top_surface_speed",
    "support_speed",
    "support_object_xy_distance",
    "support_object_first_layer_gap",
    "support_interface_speed",
    "bridge_speed",
    "internal_bridge_speed",
    "gap_infill_speed",
    "travel_speed",
    "travel_speed_z",
    "initial_layer_speed",
    "outer_wall_acceleration",
    "initial_layer_acceleration",
    "top_surface_acceleration",
    "default_acceleration",
    "skirt_type",
    "skirt_loops",
    "skirt_speed",
    "min_skirt_length",
    "skirt_distance",
    "skirt_start_angle",
    "skirt_height",
    "single_loop_draft_shield",
    "draft_shield",
    "brim_width",
    "brim_object_gap",
    "brim_flow_ratio",
    "brim_use_efc_outline",
    "combine_brims",
    "brim_type",
    "brim_ears_max_angle",
    "brim_ears_detection_length",
    "enable_support",
    "support_type",
    "support_threshold_angle",
    "support_threshold_overlap",
    "enforce_support_layers",
    "raft_layers",
    "raft_first_layer_density",
    "raft_first_layer_expansion",
    "raft_contact_distance",
    "raft_expansion",
    "support_base_pattern",
    "support_base_pattern_spacing",
    "support_expansion",
    "support_style",
    "print_extruder_id",
    "print_extruder_variant",
    "independent_support_layer_height",
    "support_angle",
    "support_interface_top_layers",
    "support_interface_bottom_layers",
    "support_interface_pattern",
    "support_interface_spacing",
    "support_interface_loop_pattern",
    "support_top_z_distance",
    "support_on_build_plate_only",
    "support_critical_regions_only",
    "bridge_no_support",
    "thick_bridges",
    "thick_internal_bridges",
    "dont_filter_internal_bridges",
    "enable_extra_bridge_layer",
    "max_bridge_length",
    "print_sequence",
    "print_order",
    "support_remove_small_overhang",
    "filename_format",
    "outer_wall_filament_id",
    "inner_wall_filament_id",
    "support_bottom_z_distance",
    "sparse_infill_filament_id",
    "internal_solid_filament_id",
    "top_surface_filament_id",
    "bottom_surface_filament_id",
    "support_filament",
    "support_interface_filament",
    "support_interface_not_for_body",
    "ooze_prevention",
    "standby_temperature_delta",
    "preheat_time",
    "preheat_steps",
    "interface_shells",
    "line_width",
    "initial_layer_line_width",
    "inner_wall_line_width",
    "outer_wall_line_width",
    "sparse_infill_line_width",
    "internal_solid_infill_line_width",
    "skin_infill_line_width",
    "skeleton_infill_line_width",
    "top_surface_line_width",
    "support_line_width",
    "infill_wall_overlap",
    "top_bottom_infill_wall_overlap",
    "bridge_flow",
    "bridge_line_width",
    "internal_bridge_flow",
    "elefant_foot_compensation",
    "elefant_foot_compensation_layers",
    "elefant_foot_layers_density",
    "xy_contour_compensation",
    "xy_hole_compensation",
    "resolution",
    "enable_prime_tower",
    "prime_tower_enable_framework",
    "prime_tower_width",
    "prime_tower_brim_width",
    "prime_tower_skip_points",
    "prime_volume",
    "prime_tower_infill_gap",
    "prime_tower_flat_ironing",
    "enable_tower_interface_features",
    "enable_tower_interface_cooldown_during_tower",
    "wipe_tower_no_sparse_layers",
    "compatible_printers",
    "compatible_printers_condition",
    "inherits",
    "flush_into_infill",
    "flush_into_objects",
    "flush_into_support",
    "tree_support_branch_angle",
    "tree_support_angle_slow",
    "tree_support_wall_count",
    "tree_support_top_rate",
    "tree_support_branch_distance",
    "tree_support_tip_diameter",
    "tree_support_branch_diameter",
    "tree_support_branch_diameter_angle",
    "detect_narrow_internal_solid_infill",
    "gcode_add_line_number",
    "enable_arc_fitting",
    "precise_z_height",
    "infill_combination",
    "infill_combination_max_layer_height",
    "support_bottom_interface_spacing",
    "enable_overhang_speed",
    "slowdown_for_curled_perimeters",
    "overhang_1_4_speed",
    "overhang_2_4_speed",
    "overhang_3_4_speed",
    "overhang_4_4_speed",
    "initial_layer_infill_speed",
    "only_one_wall_top",
    "timelapse_type",
    "wall_generator",
    "wall_transition_length",
    "wall_transition_filter_deviation",
    "wall_transition_angle",
    "wall_distribution_count",
    "min_feature_size",
    "min_bead_width",
    "post_process",
    "process_change_extrusion_role_gcode",
    "min_length_factor",
    "wall_maximum_resolution",
    "wall_maximum_deviation",
    "small_perimeter_speed",
    "small_perimeter_threshold",
    "bridge_angle",
    "internal_bridge_angle",
    "relative_bridge_angle",
    "filter_out_gap_fill",
    "travel_acceleration",
    "inner_wall_acceleration",
    "min_width_top_surface",
    "default_jerk",
    "outer_wall_jerk",
    "inner_wall_jerk",
    "infill_jerk",
    "top_surface_jerk",
    "initial_layer_jerk",
    "travel_jerk",
    "default_junction_deviation",
    "top_solid_infill_flow_ratio",
    "bottom_solid_infill_flow_ratio",
    "only_one_wall_first_layer",
    "print_flow_ratio",
    "seam_gap",
    "set_other_flow_ratios",
    "first_layer_flow_ratio",
    "outer_wall_flow_ratio",
    "inner_wall_flow_ratio",
    "overhang_flow_ratio",
    "sparse_infill_flow_ratio",
    "internal_solid_infill_flow_ratio",
    "gap_fill_flow_ratio",
    "support_flow_ratio",
    "support_interface_flow_ratio",
    "role_based_wipe_speed",
    "wipe_speed",
    "accel_to_decel_enable",
    "accel_to_decel_factor",
    "wipe_on_loops",
    "wipe_before_external_loop",
    "bridge_density",
    "internal_bridge_density",
    "precise_outer_wall",
    "bridge_acceleration",
    "sparse_infill_acceleration",
    "internal_solid_infill_acceleration",
    "tree_support_auto_brim",
    "tree_support_brim_width",
    "gcode_comments",
    "gcode_label_objects",
    "initial_layer_travel_speed",
    "initial_layer_travel_acceleration",
    "initial_layer_travel_jerk",
    "exclude_object",
    "slow_down_layers",
    "infill_anchor",
    "infill_anchor_max",
    "initial_layer_min_bead_width",
    "make_overhang_printable",
    "make_overhang_printable_angle",
    "make_overhang_printable_hole_size",
    "notes",
    "wipe_tower_cone_angle",
    "wipe_tower_extra_spacing",
    "wipe_tower_max_purge_speed",
    "wipe_tower_wall_type",
    "wipe_tower_extra_rib_length",
    "wipe_tower_rib_width",
    "wipe_tower_fillet_wall",
    "wipe_tower_filament",
    "wiping_volumes_extruders",
    "wipe_tower_bridging",
    "wipe_tower_extra_flow",
    "single_extruder_multi_material_priming",
    "wipe_tower_rotation_angle",
    "tree_support_branch_distance_organic",
    "tree_support_branch_diameter_organic",
    "tree_support_branch_angle_organic",
    "hole_to_polyhole",
    "hole_to_polyhole_threshold",
    "hole_to_polyhole_twisted",
    "mmu_segmented_region_max_width",
    "mmu_segmented_region_interlocking_depth",
    "small_area_infill_flow_compensation",
    "small_area_infill_flow_compensation_model",
    "enable_wrapping_detection",
    "seam_slope_type",
    "seam_slope_conditional",
    "scarf_angle_threshold",
    "scarf_joint_speed",
    "scarf_joint_flow_ratio",
    "seam_slope_start_height",
    "seam_slope_entire_loop",
    "seam_slope_min_length",
    "seam_slope_steps",
    "seam_slope_inner_walls",
    "scarf_overhang_threshold",
    "interlocking_beam",
    "interlocking_orientation",
    "interlocking_beam_layer_count",
    "interlocking_depth",
    "interlocking_boundary_avoidance",
    "interlocking_beam_width",
    "calib_flowrate_topinfill_special_order",
    "zaa_enabled",
    "zaa_minimize_perimeter_height",
    "zaa_dont_alternate_fill_direction",
    "zaa_min_z",
    "ironing_expansion",
];

/// `Preset::printer_options()` d'Orca 2.4.1 : `s_Preset_printer_options` +
/// `s_Preset_machine_limits_options` + `print_config_def.extruder_option_keys()`
/// (PrintConfig.cpp), doublons retirés, extrait de `$ORCA_SRC` (lot 40).
pub(crate) const ORCA_PRINTER_OPTIONS: [&str; 160] = [
    "printer_technology",
    "printable_area",
    "extruder_printable_area",
    "support_parallel_printheads",
    "parallel_printheads_count",
    "parallel_printheads_bed_exclude_areas",
    "bed_exclude_area",
    "bed_custom_texture",
    "bed_custom_model",
    "gcode_flavor",
    "fan_kickstart",
    "part_cooling_fan_min_pwm",
    "fan_speedup_time",
    "fan_speedup_overhangs",
    "single_extruder_multi_material",
    "manual_filament_change",
    "file_start_gcode",
    "machine_start_gcode",
    "machine_end_gcode",
    "before_layer_change_gcode",
    "printing_by_object_gcode",
    "layer_change_gcode",
    "time_lapse_gcode",
    "wrapping_detection_gcode",
    "change_filament_gcode",
    "change_extrusion_role_gcode",
    "printer_model",
    "printer_variant",
    "printer_extruder_id",
    "printer_extruder_variant",
    "extruder_variant_list",
    "default_nozzle_volume_type",
    "printable_height",
    "extruder_printable_height",
    "extruder_clearance_radius",
    "extruder_clearance_height_to_lid",
    "extruder_clearance_height_to_rod",
    "nozzle_height",
    "master_extruder_id",
    "default_print_profile",
    "inherits",
    "silent_mode",
    "scan_first_layer",
    "enable_power_loss_recovery",
    "wrapping_detection_layers",
    "wrapping_exclude_area",
    "machine_load_filament_time",
    "machine_unload_filament_time",
    "machine_tool_change_time",
    "time_cost",
    "machine_pause_gcode",
    "template_custom_gcode",
    "nozzle_type",
    "nozzle_hrc",
    "auxiliary_fan",
    "nozzle_volume",
    "upward_compatible_machine",
    "z_hop_types",
    "travel_slope",
    "retract_lift_enforce",
    "support_chamber_temp_control",
    "support_air_filtration",
    "printer_structure",
    "best_object_pos",
    "head_wrap_detect_zone",
    "host_type",
    "print_host",
    "printhost_apikey",
    "flashforge_serial_number",
    "bbl_use_printhost",
    "printer_agent",
    "print_host_webui",
    "printhost_cafile",
    "printhost_port",
    "printhost_authorization_type",
    "printhost_user",
    "printhost_password",
    "printhost_ssl_ignore_revoke",
    "thumbnails",
    "thumbnails_format",
    "use_relative_e_distances",
    "extruder_type",
    "use_firmware_retraction",
    "printer_notes",
    "grab_length",
    "support_object_skip_flush",
    "physical_extruder_map",
    "cooling_tube_retraction",
    "cooling_tube_length",
    "high_current_on_filament_swap",
    "parking_pos_retraction",
    "extra_loading_move",
    "wipe_tower_type",
    "purge_in_prime_tower",
    "enable_filament_ramming",
    "tool_change_on_wipe_tower",
    "z_offset",
    "disable_m73",
    "preferred_orientation",
    "emit_machine_limits_to_gcode",
    "pellet_modded_printer",
    "support_multi_bed_types",
    "use_3mf",
    "default_bed_type",
    "bed_mesh_min",
    "bed_mesh_max",
    "bed_mesh_probe_distance",
    "adaptive_bed_mesh_margin",
    "enable_long_retraction_when_cut",
    "long_retractions_when_cut",
    "retraction_distances_when_cut",
    "bed_temperature_formula",
    "nozzle_flush_dataset",
    "machine_max_acceleration_extruding",
    "machine_max_acceleration_retracting",
    "machine_max_acceleration_travel",
    "machine_max_acceleration_x",
    "machine_max_acceleration_y",
    "machine_max_acceleration_z",
    "machine_max_acceleration_e",
    "machine_max_speed_x",
    "machine_max_speed_y",
    "machine_max_speed_z",
    "machine_max_speed_e",
    "machine_min_extruding_rate",
    "machine_min_travel_rate",
    "machine_max_jerk_x",
    "machine_max_jerk_y",
    "machine_max_jerk_z",
    "machine_max_jerk_e",
    "machine_max_junction_deviation",
    "resonance_avoidance",
    "min_resonance_avoidance_speed",
    "max_resonance_avoidance_speed",
    "input_shaping_emit",
    "input_shaping_type",
    "input_shaping_freq_x",
    "input_shaping_freq_y",
    "input_shaping_damp_x",
    "input_shaping_damp_y",
    "nozzle_diameter",
    "min_layer_height",
    "max_layer_height",
    "extruder_offset",
    "retraction_length",
    "z_hop",
    "retract_lift_above",
    "retract_lift_below",
    "retraction_speed",
    "deretraction_speed",
    "retract_before_wipe",
    "retract_restart_extra",
    "retraction_minimum_travel",
    "wipe",
    "wipe_distance",
    "retract_when_changing_layer",
    "retract_length_toolchange",
    "retract_restart_extra_toolchange",
    "extruder_colour",
    "default_filament_profile",
];

#[cfg(test)]
mod tests {
    use super::*;
    use engine::api::ConfigValue;

    #[test]
    fn compact_drops_empty_entries() {
        let draft: SliceDraft = serde_json::from_value(json!({
            "process": {},
            "printer": { "z_offset": "0.1" },
            "filaments": [null, {}, null]
        }))
        .unwrap();
        let out = compact(draft).unwrap();
        assert!(out.process.is_none());
        assert!(out.filaments.is_none());
        assert_eq!(out.printer.unwrap()["z_offset"], "0.1");
        assert!(compact(SliceDraft::default()).is_none());
    }

    #[test]
    fn normalize_keeps_valid_overrides_and_rejects_unreadable_values() {
        let active = json!({
            "process": "p",
            "overrides": { "process": { "wall_loops": "5" }, "printer": {} }
        });
        let out = normalize_active_presets(active).unwrap();
        assert_eq!(
            out["overrides"],
            json!({ "process": { "wall_loops": "5" } })
        );
        assert_eq!(out["process"], "p");

        let empty = normalize_active_presets(json!({ "process": "p", "overrides": {} })).unwrap();
        assert!(empty.get("overrides").is_none());

        let bad = json!({ "overrides": { "process": { "wall_loops": "beaucoup" } } });
        assert!(normalize_active_presets(bad).is_err());
        let shape = json!({ "overrides": { "process": 3 } });
        assert!(normalize_active_presets(shape).is_err());
    }

    #[test]
    fn imported_overrides_keep_only_project_differences_by_preset_kind() {
        let mut presets = DynamicPrintConfig::new();
        presets.0.insert("wall_loops".into(), ConfigValue::Int(3));
        presets
            .0
            .insert("layer_height".into(), ConfigValue::Float(0.2));
        presets
            .0
            .insert("nozzle_diameter".into(), ConfigValue::Floats(vec![0.4]));
        let mut project = presets.clone();
        project.0.insert("wall_loops".into(), ConfigValue::Int(5));
        project
            .0
            .insert("nozzle_diameter".into(), ConfigValue::Floats(vec![0.6]));
        project.0.insert(
            "print_settings_id".into(),
            ConfigValue::String("autre".into()),
        );
        project.0.insert(
            "filament_type".into(),
            ConfigValue::Strings(vec!["PETG".into()]),
        );
        let out = imported_overrides(&project, &presets, true, &[], &[]).unwrap();
        assert_eq!(
            out.process,
            Some(json!({ "wall_loops": 5 }).as_object().unwrap().clone())
        );
        assert_eq!(
            out.printer,
            Some(
                json!({ "nozzle_diameter": [0.6] })
                    .as_object()
                    .unwrap()
                    .clone()
            )
        );
        assert!(out.filaments.is_none());
        let without_printer = imported_overrides(&project, &presets, false, &[], &[]).unwrap();
        assert!(without_printer.printer.is_none());
        assert!(imported_overrides(&presets, &presets, true, &[], &[]).is_none());
    }

    /// Représentations équivalentes (entier / flottant, liste rejointe avec
    /// ou sans espace) : pas de surcharge. Clé sans valeur côté presets ni
    /// registre : surcharge seulement si le projet la déclare modifiée.
    #[test]
    fn imported_overrides_ignore_equivalent_forms_and_unlisted_unknown_defaults() {
        let mut presets = DynamicPrintConfig::new();
        presets
            .0
            .insert("print_flow_ratio".into(), ConfigValue::Int(1));
        presets.0.insert(
            "thumbnails".into(),
            ConfigValue::String("48x48/PNG, 300x300/PNG".into()),
        );
        let mut project = DynamicPrintConfig::new();
        project
            .0
            .insert("print_flow_ratio".into(), ConfigValue::Float(1.0));
        project.0.insert(
            "thumbnails".into(),
            ConfigValue::String("48x48/PNG,300x300/PNG".into()),
        );
        project.0.insert(
            "bridge_acceleration".into(),
            ConfigValue::FloatOrPercent {
                value: 60.0,
                percent: true,
            },
        );
        project.0.insert(
            "bed_custom_model".into(),
            ConfigValue::String(String::new()),
        );
        // `bridge_acceleration` : sans défaut connu → seulement si listée.
        assert!(imported_overrides(&project, &presets, true, &[], &[]).is_none());
        let listed = imported_overrides(
            &project,
            &presets,
            true,
            &["bridge_acceleration".to_string()],
            &["bed_custom_model".to_string()],
        )
        .unwrap();
        assert_eq!(
            listed.process,
            Some(
                json!({ "bridge_acceleration": "60%" })
                    .as_object()
                    .unwrap()
                    .clone()
            )
        );
        assert!(listed.printer.is_none(), "valeur vide : rien de modifié");
    }

    #[test]
    fn different_settings_list_override_keys_per_preset_slot() {
        let mut config = DynamicPrintConfig::new();
        config.0.insert(
            "filament_settings_id".into(),
            ConfigValue::Strings(vec!["A".into(), "B".into()]),
        );
        config.0.insert(
            "different_settings_to_system".into(),
            ConfigValue::Strings(vec!["sparse_infill_density".into(), "".into()]),
        );
        let overrides: SliceDraft = serde_json::from_value(json!({
            "process": { "wall_loops": "5" },
            "filaments": [null, { "nozzle_temperature": ["250"] }],
            "printer": { "z_offset": "0.1" }
        }))
        .unwrap();
        apply_different_settings(&mut config, Some(&overrides));
        assert_eq!(
            config.0["different_settings_to_system"],
            ConfigValue::Strings(vec![
                "sparse_infill_density;wall_loops".into(),
                "".into(),
                "nozzle_temperature".into(),
                "z_offset".into(),
            ])
        );
    }

    #[test]
    fn option_lists_match_orca() {
        assert_eq!(ORCA_PRINT_OPTIONS.len(), 355);
        assert!(ORCA_PRINT_OPTIONS.contains(&"wall_loops"));
        assert!(ORCA_PRINTER_OPTIONS.contains(&"thumbnails"));
        assert!(ORCA_PRINTER_OPTIONS.contains(&"nozzle_diameter"));
        assert!(ORCA_PRINTER_OPTIONS.contains(&"machine_max_jerk_x"));
    }
}
