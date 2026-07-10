// GÉNÉRÉ par audit/generate_legacy_rs.py — NE PAS ÉDITER (constitution V).
// Source : audit/legacy_keys.json (renommages 28, ignorées 44).
// Fraîcheur verrouillée par le test `legacy_tables_synced_with_audit`.

/// SHA-1 de audit/legacy_keys.json au moment de la génération.
pub const SOURCE_SHA1: &str = "a098bfee6bf2b5b64583f9c16ac73c346dbad461";

/// Renommages inconditionnels (ancienne clé → nouvelle clé), triés par ancienne
/// clé pour la recherche dichotomique.
pub static RENAMES: &[(&str, &str)] = &[
    ("bridge_fan_speed", "overhang_fan_speed"),
    ("chamber_temperatures", "chamber_temperature"),
    (
        "compatible_printers_condition_cummulative",
        "compatible_machine_expression_group",
    ),
    (
        "compatible_prints_condition_cummulative",
        "compatible_process_expression_group",
    ),
    ("cooling", "slow_down_for_layer_cooling"),
    ("counterbole_hole_bridging", "counterbore_hole_bridging"),
    ("enable_wipe_tower", "enable_prime_tower"),
    ("extruder_clearance_max_radius", "extruder_clearance_radius"),
    ("inherits_cummulative", "inherits_group"),
    ("initial_layer_flow_ratio", "bottom_solid_infill_flow_ratio"),
    ("ironing_direction", "ironing_angle"),
    ("machine_switch_extruder_time", "machine_tool_change_time"),
    (
        "prime_tower_extra_rib_length",
        "wipe_tower_extra_rib_length",
    ),
    ("prime_tower_fillet_wall", "wipe_tower_fillet_wall"),
    ("prime_tower_rib_width", "wipe_tower_rib_width"),
    ("sparse_infill_anchor", "infill_anchor"),
    ("sparse_infill_anchor_max", "infill_anchor_max"),
    ("support_material_angle", "support_angle"),
    ("support_material_enforce_layers", "enforce_support_layers"),
    ("support_material_extruder", "support_filament"),
    (
        "support_material_interface_extruder",
        "support_interface_filament",
    ),
    ("thumbnail_size", "thumbnails"),
    ("timelapse_no_toolhead", "timelapse_type"),
    ("tool_change_gcode", "change_filament_gcode"),
    ("wipe_tower_brim_width", "prime_tower_brim_width"),
    ("wipe_tower_extruder", "wipe_tower_filament"),
    ("wipe_tower_width", "prime_tower_width"),
    ("wiping_volume", "prime_volume"),
];

/// Clés obsolètes explicitement ignorées à l'import (triées).
pub static IGNORED: &[&str] = &[
    "acceleration",
    "adaptive_layer_height",
    "bed_size",
    "bed_temperature",
    "bed_temperature_difference",
    "bed_temperature_initial_layer",
    "can_add_auxiliary_fan",
    "can_switch_nozzle_type",
    "duplicate",
    "duplicate_grid",
    "extra_flush_volume",
    "filament_load_time",
    "filament_prime_volume",
    "filament_unload_time",
    "g0",
    "internal_bridge_support_thickness",
    "long_retraction_when_cut",
    "max_print_speed",
    "max_volumetric_speed",
    "overhang_speed_classic",
    "overhang_totally_speed",
    "print_center",
    "reduce_wall_solid_infill",
    "remove_bed_leveling",
    "remove_extrusion_calibration",
    "remove_freq_sweep",
    "retraction_distance_when_cut",
    "rotate",
    "scale",
    "silent_mode",
    "smooth_coefficient",
    "spaghetti_detector",
    "support_closing_radius",
    "support_remove_small_overhangs",
    "support_sharp_tails",
    "support_transition_line_width",
    "support_transition_speed",
    "support_with_sheath",
    "top_area_threshold",
    "tree_support_collision_resolution",
    "tree_support_with_infill",
    "wipe_tower_per_color_wipe",
    "z_hop_type",
    "z_lift_type",
];
