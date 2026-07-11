// GÉNÉRÉ par audit/generate_frontend_ts.py (scripts/codegen.sh)
// — NE PAS ÉDITER À LA MAIN (constitution V).
// Source : audit/ui_inventory.json.
// Fraîcheur verrouillée par frontend/src/lib/settings/ui-layout.test.ts.

/** Ligne d'option : une clé de `PARAMS` (params.ts), ou un marqueur de ligne
 *  générée dynamiquement par Orca (ex. options par extrudeur/plugin). */
export type UiOption = string | { dynamic: string };

/** Un groupe d'options au sein d'une page. */
export interface UiSection {
  title: string;
  options: UiOption[];
}

/** Type de preset auquel une page de réglages appartient (cadrage des onglets). */
export type PresetKind = "process" | "filament" | "machine";

/** Une page d'onglet de réglages (Quality, Strength, Speed…). */
export interface UiPage {
  title: string;
  icon: string;
  kind: PresetKind;
  sections: UiSection[];
}

export const UI_LAYOUT: UiPage[] = [
  {
    title: "Quality",
    icon: "custom-gcode_quality",
    kind: "process",
    sections: [
      { title: "Layer height", options: ["layer_height","initial_layer_print_height"] },
      { title: "Line width", options: ["line_width","initial_layer_line_width","outer_wall_line_width","inner_wall_line_width","top_surface_line_width","sparse_infill_line_width","internal_solid_infill_line_width","support_line_width","bridge_line_width"] },
      { title: "Seam", options: ["seam_position","staggered_inner_seams","seam_gap","seam_slope_type","seam_slope_conditional","scarf_angle_threshold","scarf_overhang_threshold","scarf_joint_speed","seam_slope_start_height","seam_slope_entire_loop","seam_slope_min_length","seam_slope_steps","scarf_joint_flow_ratio","seam_slope_inner_walls","role_based_wipe_speed","wipe_speed","wipe_on_loops","wipe_before_external_loop"] },
      { title: "Precision", options: ["slice_closing_radius","resolution","enable_arc_fitting","xy_hole_compensation","xy_contour_compensation","elefant_foot_compensation","elefant_foot_layers_density","elefant_foot_compensation_layers","precise_outer_wall","precise_z_height","hole_to_polyhole","hole_to_polyhole_threshold","hole_to_polyhole_twisted"] },
      { title: "Ironing", options: ["ironing_type","ironing_pattern","ironing_flow","ironing_spacing","ironing_inset","ironing_angle","ironing_angle_fixed"] },
      { title: "Z contouring", options: ["zaa_enabled","zaa_minimize_perimeter_height","zaa_min_z","zaa_dont_alternate_fill_direction"] },
      { title: "Wall generator", options: ["wall_generator","wall_transition_angle","wall_transition_filter_deviation","wall_transition_length","wall_distribution_count","initial_layer_min_bead_width","min_bead_width","min_feature_size","min_length_factor","wall_maximum_resolution","wall_maximum_deviation"] },
      { title: "Walls and surfaces", options: ["wall_sequence","is_infill_first","wall_direction","print_flow_ratio","top_solid_infill_flow_ratio","bottom_solid_infill_flow_ratio","set_other_flow_ratios","first_layer_flow_ratio","outer_wall_flow_ratio","inner_wall_flow_ratio","overhang_flow_ratio","sparse_infill_flow_ratio","internal_solid_infill_flow_ratio","gap_fill_flow_ratio","support_flow_ratio","support_interface_flow_ratio","only_one_wall_first_layer","only_one_wall_top","min_width_top_surface","reduce_crossing_wall","max_travel_detour_distance","small_area_infill_flow_compensation",{"dynamic":"option"}] },
      { title: "Bridging", options: ["bridge_flow","internal_bridge_flow","bridge_density","internal_bridge_density","thick_bridges","thick_internal_bridges","enable_extra_bridge_layer","dont_filter_internal_bridges","counterbore_hole_bridging"] },
      { title: "Overhangs", options: ["detect_overhang_wall","make_overhang_printable","make_overhang_printable_angle","make_overhang_printable_hole_size","extra_perimeters_on_overhangs","overhang_reverse","overhang_reverse_internal_only","overhang_reverse_threshold"] },
    ],
  },
  {
    title: "Strength",
    icon: "custom-gcode_strength",
    kind: "process",
    sections: [
      { title: "Walls", options: ["wall_loops","alternate_extra_wall","detect_thin_wall"] },
      { title: "Top/bottom shells", options: ["top_shell_layers","top_shell_thickness","top_surface_density","top_surface_pattern","bottom_shell_layers","bottom_shell_thickness","bottom_surface_density","bottom_surface_pattern","top_bottom_infill_wall_overlap"] },
      { title: "Infill", options: ["sparse_infill_density","fill_multiline","sparse_infill_pattern","gyroid_optimized","infill_direction","sparse_infill_rotate_template","skin_infill_density","skeleton_infill_density","infill_lock_depth","skin_infill_depth","skin_infill_line_width","skeleton_infill_line_width","symmetric_infill_y_axis","infill_shift_step","lateral_lattice_angle_1","lateral_lattice_angle_2","infill_overhang_angle","lightning_overhang_angle","lightning_prune_angle","lightning_straightening_angle","infill_anchor_max","infill_anchor","internal_solid_infill_pattern","solid_infill_direction","solid_infill_rotate_template","gap_fill_target","filter_out_gap_fill","infill_wall_overlap"] },
      { title: "Advanced", options: ["align_infill_direction_to_model","extra_solid_infills","bridge_angle","internal_bridge_angle","relative_bridge_angle","minimum_sparse_infill_area","infill_combination","infill_combination_max_layer_height","detect_narrow_internal_solid_infill","ensure_vertical_shell_thickness"] },
    ],
  },
  {
    title: "Speed",
    icon: "custom-gcode_speed",
    kind: "process",
    sections: [
      { title: "First layer speed", options: ["initial_layer_speed","initial_layer_infill_speed","initial_layer_travel_speed","slow_down_layers"] },
      { title: "Other layers speed", options: ["outer_wall_speed","inner_wall_speed","small_perimeter_speed","small_perimeter_threshold","sparse_infill_speed","internal_solid_infill_speed","top_surface_speed","gap_infill_speed","ironing_speed","support_speed","support_interface_speed"] },
      { title: "Overhang speed", options: ["enable_overhang_speed","slowdown_for_curled_perimeters"] },
      { title: "Travel speed", options: ["travel_speed"] },
      { title: "Acceleration", options: ["default_acceleration","outer_wall_acceleration","inner_wall_acceleration","bridge_acceleration","sparse_infill_acceleration","internal_solid_infill_acceleration","initial_layer_acceleration","initial_layer_travel_acceleration","top_surface_acceleration","travel_acceleration","accel_to_decel_enable","accel_to_decel_factor"] },
      { title: "Junction Deviation", options: ["default_junction_deviation"] },
      { title: "Jerk(XY)", options: ["default_jerk","outer_wall_jerk","inner_wall_jerk","infill_jerk","top_surface_jerk","initial_layer_jerk","initial_layer_travel_jerk","travel_jerk"] },
      { title: "Advanced", options: ["max_volumetric_extrusion_rate_slope","max_volumetric_extrusion_rate_slope_segment_length","extrusion_rate_smoothing_external_perimeter_only"] },
    ],
  },
  {
    title: "Support",
    icon: "custom-gcode_support",
    kind: "process",
    sections: [
      { title: "Support", options: ["enable_support","support_type","support_style","support_threshold_angle","support_threshold_overlap","raft_first_layer_density","raft_first_layer_expansion","support_on_build_plate_only","support_critical_regions_only","support_remove_small_overhang"] },
      { title: "Raft", options: ["raft_layers","raft_contact_distance"] },
      { title: "Support filament", options: ["support_filament","support_interface_filament","support_interface_not_for_body"] },
      { title: "Support ironing", options: ["support_ironing","support_ironing_pattern","support_ironing_flow","support_ironing_spacing"] },
      { title: "Advanced", options: ["support_top_z_distance","support_bottom_z_distance","tree_support_wall_count","support_base_pattern","support_base_pattern_spacing","support_angle","support_interface_top_layers","support_interface_bottom_layers","support_interface_pattern","support_interface_spacing","support_bottom_interface_spacing","support_expansion","support_object_xy_distance","support_object_first_layer_gap","bridge_no_support","max_bridge_length","independent_support_layer_height"] },
      { title: "Tree supports", options: ["tree_support_tip_diameter","tree_support_branch_distance","tree_support_branch_distance_organic","tree_support_top_rate","tree_support_branch_diameter","tree_support_branch_diameter_organic","tree_support_branch_diameter_angle","tree_support_branch_angle","tree_support_branch_angle_organic","tree_support_angle_slow","tree_support_auto_brim","tree_support_brim_width"] },
    ],
  },
  {
    title: "Multimaterial",
    icon: "custom-gcode_multi_material",
    kind: "process",
    sections: [
      { title: "Prime tower", options: ["enable_prime_tower","prime_tower_skip_points","enable_tower_interface_features","enable_tower_interface_cooldown_during_tower","prime_tower_enable_framework","prime_tower_width","prime_volume","prime_tower_brim_width","prime_tower_infill_gap","wipe_tower_rotation_angle","wipe_tower_bridging","wipe_tower_extra_spacing","wipe_tower_extra_flow","wipe_tower_max_purge_speed","wipe_tower_wall_type","wipe_tower_cone_angle","wipe_tower_extra_rib_length","wipe_tower_rib_width","wipe_tower_fillet_wall","wipe_tower_no_sparse_layers","single_extruder_multi_material_priming"] },
      { title: "Filament for Features", options: ["outer_wall_filament_id","inner_wall_filament_id","sparse_infill_filament_id","internal_solid_filament_id","top_surface_filament_id","bottom_surface_filament_id","wipe_tower_filament"] },
      { title: "Ooze prevention", options: ["ooze_prevention","standby_temperature_delta","preheat_time","preheat_steps"] },
      { title: "Flush options", options: ["flush_into_infill","flush_into_objects","flush_into_support"] },
      { title: "Advanced", options: ["interlocking_beam","interface_shells","mmu_segmented_region_max_width","mmu_segmented_region_interlocking_depth","interlocking_beam_width","interlocking_orientation","interlocking_beam_layer_count","interlocking_depth","interlocking_boundary_avoidance"] },
    ],
  },
  {
    title: "Others",
    icon: "custom-gcode_other",
    kind: "process",
    sections: [
      { title: "Skirt", options: ["skirt_loops","skirt_type","min_skirt_length","skirt_distance","skirt_start_angle","skirt_speed","skirt_height","draft_shield","single_loop_draft_shield"] },
      { title: "Brim", options: ["brim_type","brim_width","brim_object_gap","brim_flow_ratio","brim_use_efc_outline","combine_brims","brim_ears_max_angle","brim_ears_detection_length"] },
      { title: "Special mode", options: ["slicing_mode","print_sequence","print_order","spiral_mode","spiral_mode_smooth","spiral_mode_max_xy_smoothing","spiral_starting_flow_ratio","spiral_finishing_flow_ratio","timelapse_type","enable_wrapping_detection"] },
      { title: "Fuzzy Skin", options: ["fuzzy_skin","fuzzy_skin_mode","fuzzy_skin_noise_type","fuzzy_skin_point_distance","fuzzy_skin_thickness","fuzzy_skin_scale","fuzzy_skin_octaves","fuzzy_skin_persistence","fuzzy_skin_ripples_per_layer","fuzzy_skin_ripple_offset","fuzzy_skin_layers_between_ripple_offset","fuzzy_skin_first_layer"] },
      { title: "G-code output", options: ["reduce_infill_retraction","gcode_add_line_number","gcode_comments","gcode_label_objects","exclude_object",{"dynamic":"option"}] },
      { title: "Change extrusion role G-code", options: [{"dynamic":"option"}] },
      { title: "Post-processing Scripts", options: [{"dynamic":"option"}] },
      { title: "Notes", options: [{"dynamic":"option"}] },
    ],
  },
  {
    title: "Frequent",
    icon: "empty",
    kind: "process",
    sections: [
      { title: "", options: ["layer_height","sparse_infill_density","wall_loops","enable_support"] },
    ],
  },
  {
    title: "Plate Settings",
    icon: "empty",
    kind: "process",
    sections: [
      { title: "", options: ["curr_bed_type","skirt_start_angle","print_sequence","spiral_mode","first_layer_sequence_choice","other_layers_sequence_choice"] },
    ],
  },
  {
    title: "Setting Overrides",
    icon: "custom-gcode_setting_override",
    kind: "filament",
    sections: [
      { title: "Retraction", options: ["filament_retraction_length","filament_z_hop","filament_z_hop_types","filament_retract_lift_above","filament_retract_lift_below","filament_retract_lift_enforce","filament_retraction_speed","filament_deretraction_speed","filament_retract_restart_extra","filament_retraction_minimum_travel","filament_retract_when_changing_layer","filament_wipe","filament_wipe_distance","filament_retract_before_wipe","filament_long_retractions_when_cut","filament_retraction_distances_when_cut"] },
      { title: "Ironing", options: ["filament_ironing_flow","filament_ironing_spacing","filament_ironing_inset","filament_ironing_speed"] },
    ],
  },
  {
    title: "Filament",
    icon: "custom-gcode_filament",
    kind: "filament",
    sections: [
      { title: "Basic information", options: ["filament_type","filament_vendor","filament_soluble","filament_is_support","filament_change_length","required_nozzle_HRC","default_filament_colour","filament_diameter","filament_adhesiveness_category","filament_density","filament_shrink","filament_shrinkage_compensation_z","filament_cost","temperature_vitrification","idle_temperature"] },
      { title: "Flow ratio and Pressure Advance", options: ["pellet_flow_coefficient","filament_flow_ratio","enable_pressure_advance","pressure_advance","adaptive_pressure_advance","adaptive_pressure_advance_overhangs","adaptive_pressure_advance_bridges",{"dynamic":"option"}] },
      { title: "Print chamber temperature", options: ["activate_chamber_temp_control"] },
      { title: "Print temperature", options: [] },
      { title: "Bed temperature", options: [] },
      { title: "Volumetric speed limitation", options: ["filament_adaptive_volumetric_speed","filament_max_volumetric_speed"] },
    ],
  },
  {
    title: "Cooling",
    icon: "custom-gcode_cooling_fan",
    kind: "filament",
    sections: [
      { title: "Cooling for specific layer", options: ["close_fan_the_first_x_layers","full_fan_speed_layer"] },
      { title: "Part cooling fan", options: ["reduce_fan_stop_start_freq","slow_down_for_layer_cooling","dont_slow_down_outer_wall","slow_down_min_speed","enable_overhang_bridge_fan","overhang_fan_threshold","overhang_fan_speed","internal_bridge_fan_speed","support_material_interface_fan_speed","ironing_fan_speed"] },
      { title: "Auxiliary part cooling fan", options: ["additional_cooling_fan_speed"] },
      { title: "Exhaust fan", options: ["activate_air_filtration"] },
    ],
  },
  {
    title: "Advanced",
    icon: "custom-gcode_advanced",
    kind: "filament",
    sections: [
      { title: "Filament start G-code", options: [{"dynamic":"option"}] },
      { title: "Change extrusion role G-code", options: [{"dynamic":"option"}] },
      { title: "Filament end G-code", options: [{"dynamic":"option"}] },
    ],
  },
  {
    title: "Multimaterial",
    icon: "custom-gcode_multi_material",
    kind: "filament",
    sections: [
      { title: "Wipe tower parameters", options: ["filament_minimal_purge_on_wipe_tower","filament_tower_interface_pre_extrusion_dist","filament_tower_interface_pre_extrusion_length","filament_tower_ironing_area","filament_tower_interface_purge_volume","filament_tower_interface_print_temp"] },
      { title: "Multi Filament", options: ["long_retractions_when_ec","retraction_distances_when_ec"] },
      { title: "Tool change parameters with single extruder MM printers", options: ["filament_loading_speed_start","filament_loading_speed","filament_unloading_speed_start","filament_unloading_speed","filament_toolchange_delay","filament_cooling_moves","filament_cooling_initial_speed","filament_cooling_final_speed","filament_stamping_loading_speed","filament_stamping_distance"] },
      { title: "Tool change parameters with multi extruder MM printers", options: ["filament_multitool_ramming","filament_multitool_ramming_volume","filament_multitool_ramming_flow"] },
    ],
  },
  {
    title: "Dependencies",
    icon: "advanced",
    kind: "filament",
    sections: [
      { title: "Compatible printers", options: [{"dynamic":"option"}] },
      { title: "Compatible process profiles", options: [{"dynamic":"option"}] },
    ],
  },
  {
    title: "Notes",
    icon: "custom-gcode_note",
    kind: "filament",
    sections: [
      { title: "Notes", options: [{"dynamic":"option"}] },
    ],
  },
  {
    title: "Basic information",
    icon: "custom-gcode_object-info",
    kind: "machine",
    sections: [
      { title: "Printable space", options: ["parallel_printheads_count",{"dynamic":"option"},"printable_height","support_multi_bed_types","best_object_pos","z_offset","preferred_orientation"] },
      { title: "Advanced", options: ["printer_structure","gcode_flavor","pellet_modded_printer","bbl_use_printhost","use_3mf","scan_first_layer","enable_power_loss_recovery","disable_m73",{"dynamic":"option"},"use_relative_e_distances","use_firmware_retraction","time_cost"] },
      { title: "Cooling Fan", options: ["fan_kickstart","part_cooling_fan_min_pwm"] },
      { title: "Extruder Clearance", options: ["extruder_clearance_radius","extruder_clearance_height_to_rod","extruder_clearance_height_to_lid"] },
      { title: "Adaptive bed mesh", options: ["bed_mesh_min","bed_mesh_max","bed_mesh_probe_distance","adaptive_bed_mesh_margin"] },
      { title: "Accessory", options: ["nozzle_type","nozzle_hrc","auxiliary_fan","support_chamber_temp_control","support_air_filtration"] },
    ],
  },
  {
    title: "Machine G-code",
    icon: "custom-gcode_gcode",
    kind: "machine",
    sections: [
      { title: "File header G-code", options: [{"dynamic":"option"}] },
      { title: "Machine start G-code", options: [{"dynamic":"option"}] },
      { title: "Machine end G-code", options: [{"dynamic":"option"}] },
      { title: "Printing by object G-code", options: [{"dynamic":"option"}] },
      { title: "Before layer change G-code", options: [{"dynamic":"option"}] },
      { title: "Layer change G-code", options: [{"dynamic":"option"}] },
      { title: "Timelapse G-code", options: [{"dynamic":"option"}] },
      { title: "Clumping Detection G-code", options: [{"dynamic":"option"}] },
      { title: "Change filament G-code", options: [{"dynamic":"option"}] },
      { title: "Change extrusion role G-code", options: [{"dynamic":"option"}] },
      { title: "Pause G-code", options: [{"dynamic":"option"}] },
      { title: "Template Custom G-code", options: [{"dynamic":"option"}] },
    ],
  },
  {
    title: "Notes",
    icon: "custom-gcode_note",
    kind: "machine",
    sections: [
      { title: "Notes", options: [{"dynamic":"option"}] },
    ],
  },
  {
    title: "Motion ability",
    icon: "custom-gcode_motion",
    kind: "machine",
    sections: [
      { title: "", options: [] },
      { title: "Advanced", options: ["emit_machine_limits_to_gcode"] },
      { title: "Resonance Compensation", options: ["resonance_avoidance","input_shaping_emit","input_shaping_type"] },
      { title: "Speed limitation", options: [] },
      { title: "Acceleration limitation", options: [] },
      { title: "Jerk limitation", options: [] },
    ],
  },
  {
    title: "Multimaterial",
    icon: "custom-gcode_multi_material",
    kind: "machine",
    sections: [
      { title: "Single extruder multi-material setup", options: ["single_extruder_multi_material",{"dynamic":"option"},"manual_filament_change","bed_temperature_formula"] },
      { title: "Wipe tower", options: ["wipe_tower_type","purge_in_prime_tower","enable_filament_ramming","tool_change_on_wipe_tower"] },
      { title: "Single extruder multi-material parameters", options: ["cooling_tube_retraction","cooling_tube_length","parking_pos_retraction","extra_loading_move","high_current_on_filament_swap"] },
      { title: "Advanced", options: ["machine_load_filament_time","machine_unload_filament_time","machine_tool_change_time"] },
    ],
  },
  {
    title: "Extruder",
    icon: "custom-gcode_extruder",
    kind: "machine",
    sections: [
      { title: "Basic information", options: ["nozzle_diameter","nozzle_volume","extruder_printable_height",{"dynamic":"option"}] },
      { title: "Layer height limits", options: ["min_layer_height","max_layer_height"] },
      { title: "Position", options: ["extruder_offset"] },
      { title: "Retraction", options: ["retraction_length","retract_restart_extra","retraction_speed","deretraction_speed","retraction_minimum_travel","retract_when_changing_layer","wipe","wipe_distance","retract_before_wipe"] },
      { title: "Z-Hop", options: ["retract_lift_enforce","z_hop_types","z_hop","travel_slope","retract_lift_above","retract_lift_below"] },
      { title: "Retraction when switching material", options: ["retract_length_toolchange","retract_restart_extra_toolchange","long_retractions_when_cut","retraction_distances_when_cut"] },
    ],
  },
];

/** Compteurs de structure (contrôle de fraîcheur). */
export const UI_LAYOUT_COUNTS = {
  pages: 21,
  sections: 100,
  optionLines: 525,
};
