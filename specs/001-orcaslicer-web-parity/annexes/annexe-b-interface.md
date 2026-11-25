# Annexe B — Inventaire exhaustif de l'interface (normatif)

Source : `audit/ui_inventory.json`. Chaque élément DOIT exister dans
l'application web (même organisation, mêmes groupes) ou figurer au
registre d'exclusions avec justification.

## B.1 Onglets de réglages

### Quality — `TabPrint::build`

- **Layer height** : `layer_height`, `initial_layer_print_height`, `enable_mixed_color_sublayer`
- **Line width** : `line_width`, `initial_layer_line_width`, `outer_wall_line_width`, `inner_wall_line_width`, `top_surface_line_width`, `sparse_infill_line_width`, `internal_solid_infill_line_width`, `support_line_width`, `bridge_line_width`
- **Seam** : `seam_position`, `staggered_inner_seams`, `seam_gap`, `seam_slope_type`, `seam_slope_conditional`, `scarf_angle_threshold`, `scarf_overhang_threshold`, `scarf_joint_speed`, `seam_slope_start_height`, `seam_slope_entire_loop`, `seam_slope_min_length`, `seam_slope_steps`, `scarf_joint_flow_ratio`, `seam_slope_inner_walls`, `role_based_wipe_speed`, `wipe_speed`, `wipe_on_loops`, `wipe_inward`, `wipe_inward_distance`, `wipe_before_external_loop`, `retract_after_outer_wall`
- **Precision** : `slice_closing_radius`, `resolution`, `enable_arc_fitting`, `xy_hole_compensation`, `xy_contour_compensation`, `elefant_foot_compensation`, `elefant_foot_layers_density`, `elefant_foot_compensation_layers`, `precise_outer_wall`, `precise_z_height`, `hole_to_polyhole`, `hole_to_polyhole_threshold`, `hole_to_polyhole_twisted`, `hole_to_polyhole_max_edges`
- **Ironing** : `ironing_type`, `ironing_pattern`, `ironing_flow`, `ironing_spacing`, `ironing_inset`, `ironing_angle`, `ironing_angle_fixed`
- **Z contouring** : `zaa_enabled`, `zaa_minimize_perimeter_height`, `zaa_min_z`, `zaa_dont_alternate_fill_direction`
- **Wall generator** : `wall_generator`, `wall_transition_angle`, `wall_transition_filter_deviation`, `wall_transition_length`, `wall_distribution_count`, `initial_layer_min_bead_width`, `min_bead_width`, `min_feature_size`, `min_length_factor`, `wall_maximum_resolution`, `wall_maximum_deviation`
- **Walls and surfaces** : `wall_sequence`, `is_infill_first`, `wall_direction`, `print_flow_ratio`, `top_solid_infill_flow_ratio`, `bottom_solid_infill_flow_ratio`, `set_other_flow_ratios`, `first_layer_flow_ratio`, `outer_wall_flow_ratio`, `inner_wall_flow_ratio`, `overhang_flow_ratio`, `sparse_infill_flow_ratio`, `internal_solid_infill_flow_ratio`, `gap_fill_flow_ratio`, `support_flow_ratio`, `support_interface_flow_ratio`, `only_one_wall_first_layer`, `only_one_wall_top`, `min_width_top_surface`, `reduce_crossing_wall`, `max_travel_detour_distance`, `small_area_infill_flow_compensation`, `small_area_infill_flow_compensation_model`
- **Bridging** : `bridge_flow`, `internal_bridge_flow`, `bridge_density`, `internal_bridge_density`, `thick_bridges`, `thick_internal_bridges`, `enable_extra_bridge_layer`, `dont_filter_internal_bridges`, `counterbore_hole_bridging`
- **Overhangs** : `detect_overhang_wall`, `unsupported_wall_last`, `make_overhang_printable`, `make_overhang_printable_angle`, `make_overhang_printable_hole_size`, `extra_perimeters_on_overhangs`, `overhang_reverse`, `overhang_reverse_internal_only`, `overhang_reverse_threshold`

### Strength — `TabPrint::build`

- **Walls** : `wall_loops`, `alternate_extra_wall`, `detect_thin_wall`
- **Top/bottom shells** : `top_shell_layers`, `top_shell_thickness`, `top_color_penetration_layers`, `top_surface_density`, `top_surface_pattern`, `top_surface_fill_order`, `top_layer_direction`, `top_surface_expansion`, `top_surface_expansion_margin`, `top_surface_expansion_direction`, `bottom_shell_layers`, `bottom_shell_thickness`, `bottom_color_penetration_layers`, `bottom_surface_density`, `bottom_surface_pattern`, `bottom_surface_fill_order`, `bottom_layer_direction`, `center_of_surface_pattern`, `top_bottom_infill_wall_overlap`
- **Infill** : `sparse_infill_density`, `fill_multiline`, `sparse_infill_pattern`, `sparse_infill_strategy`, `adaptive_infill_mode`, `adaptive_infill_interior_density`, `adaptive_infill_gradient`, `gyroid_optimized`, `sparse_infill_smooth_factor`, `infill_direction`, `sparse_infill_rotate_template`, `skin_infill_density`, `skeleton_infill_density`, `infill_lock_depth`, `skin_infill_depth`, `skin_infill_line_width`, `skeleton_infill_line_width`, `symmetric_infill_y_axis`, `infill_complete_top`, `infill_shift_step`, `lateral_lattice_angle_1`, `lateral_lattice_angle_2`, `infill_overhang_angle`, `lightning_overhang_angle`, `lightning_prune_angle`, `lightning_straightening_angle`, `infill_anchor_max`, `infill_anchor`, `internal_solid_infill_pattern`, `solid_infill_direction`, `solid_infill_rotate_template`, `gap_fill_target`, `filter_out_gap_fill`, `separated_infills`, `infill_wall_overlap`
- **Advanced** : `align_infill_direction_to_model`, `extra_solid_infills`, `bridge_angle`, `internal_bridge_angle`, `relative_bridge_angle`, `minimum_sparse_infill_area`, `infill_combination`, `infill_combination_max_layer_height`, `detect_narrow_internal_solid_infill`, `ensure_vertical_shell_thickness`

### Speed — `TabPrint::build`

- **Speed definition** : `enable_volumetric_speeds`
- **First layer speed** : `initial_layer_speed`, `initial_layer_volumetric_flow`, `initial_layer_infill_speed`, `initial_layer_infill_volumetric_flow`, `initial_layer_travel_speed`, `slow_down_layers`
- **Other layers speed** : `outer_wall_speed`, `outer_wall_volumetric_flow`, `inner_wall_speed`, `inner_wall_volumetric_flow`, `small_perimeter_speed`, `small_perimeter_threshold`, `sparse_infill_speed`, `sparse_infill_volumetric_flow`, `internal_solid_infill_speed`, `internal_solid_infill_volumetric_flow`, `top_surface_speed`, `top_surface_volumetric_flow`, `gap_infill_speed`, `gap_infill_volumetric_flow`, `ironing_speed`, `support_speed`, `support_volumetric_flow`, `support_interface_speed`, `support_interface_volumetric_flow`, `small_support_perimeter_speed`, `small_support_perimeter_threshold`
- **Overhang speed** : `enable_overhang_speed`, `slowdown_for_curled_perimeters`, `overhang_1_4_speed`, `overhang_2_4_speed`, `overhang_3_4_speed`, `overhang_4_4_speed`, `bridge_speed`, `internal_bridge_speed`, `bridge_volumetric_flow`, `internal_bridge_volumetric_flow`
- **Travel speed** : `travel_speed`
- **Acceleration** : `default_acceleration`, `outer_wall_acceleration`, `inner_wall_acceleration`, `bridge_acceleration`, `sparse_infill_acceleration`, `internal_solid_infill_acceleration`, `initial_layer_acceleration`, `initial_layer_travel_acceleration`, `top_surface_acceleration`, `travel_acceleration`, `accel_to_decel_enable`, `accel_to_decel_factor`
- **Junction Deviation** : `default_junction_deviation`
- **Jerk(XY)** : `default_jerk`, `outer_wall_jerk`, `inner_wall_jerk`, `infill_jerk`, `top_surface_jerk`, `initial_layer_jerk`, `initial_layer_travel_jerk`, `travel_jerk`
- **Advanced** : `max_volumetric_extrusion_rate_slope`, `max_volumetric_extrusion_rate_slope_segment_length`, `extrusion_rate_smoothing_external_perimeter_only`

### Support — `TabPrint::build`

- **Support** : `enable_support`, `support_type`, `support_style`, `support_threshold_angle`, `support_threshold_overlap`, `raft_first_layer_density`, `raft_first_layer_expansion`, `support_on_build_plate_only`, `support_critical_regions_only`, `support_remove_small_overhang`
- **Raft** : `raft_layers`, `raft_contact_distance`, `raft_expansion`
- **Filament for Supports** : `support_filament`, `support_interface_filament`, `support_interface_not_for_body`
- **Support ironing** : `support_ironing`, `support_ironing_pattern`, `support_ironing_flow`, `support_ironing_spacing`
- **Advanced** : `support_top_z_distance`, `support_bottom_z_distance`, `tree_support_wall_count`, `support_base_pattern`, `support_base_pattern_spacing`, `support_angle`, `support_interface_top_layers`, `support_interface_bottom_layers`, `support_interface_pattern`, `support_interface_spacing`, `support_bottom_interface_spacing`, `support_expansion`, `support_object_xy_distance`, `support_object_first_layer_gap`, `bridge_no_support`, `max_bridge_length`, `independent_support_layer_height`
- **Tree supports** : `tree_support_tip_diameter`, `tree_support_branch_distance`, `tree_support_branch_distance_organic`, `tree_support_top_rate`, `tree_support_branch_diameter`, `tree_support_branch_diameter_organic`, `tree_support_branch_diameter_angle`, `tree_support_branch_angle`, `tree_support_branch_angle_organic`, `tree_support_angle_slow`, `tree_support_auto_brim`, `tree_support_brim_width`

### Multimaterial — `TabPrint::build`

- **Prime tower** : `enable_prime_tower`, `prime_tower_skip_points`, `enable_tower_interface_features`, `enable_tower_interface_cooldown_during_tower`, `prime_tower_enable_framework`, `prime_tower_width`, `belt_purge_tower_width`, `prime_volume`, `prime_tower_brim_width`, `prime_tower_infill_gap`, `wipe_tower_rotation_angle`, `wipe_tower_bridging`, `wipe_tower_extra_spacing`, `wipe_tower_extra_flow`, `wipe_tower_max_purge_speed`, `wipe_tower_wall_type`, `wipe_tower_cone_angle`, `wipe_tower_extra_rib_length`, `wipe_tower_rib_width`, `wipe_tower_fillet_wall`, `wipe_tower_no_sparse_layers`, `wipe_tower_sparse_layers_combination`, `single_extruder_multi_material_priming`
- **Filament for Features** : `outer_wall_filament_id`, `inner_wall_filament_id`, `sparse_infill_filament_id`, `internal_solid_filament_id`, `top_surface_filament_id`, `bottom_surface_filament_id`, `brim_filament_id`, `wipe_tower_filament`
- **Ooze prevention** : `ooze_prevention`, `standby_temperature_delta`, `preheat_time`, `preheat_steps`
- **Flush options** : `flush_into_infill`, `flush_into_objects`, `flush_into_support`
- **Advanced** : `interlocking_beam`, `toolchange_ordering`, `toolchange_cyclic_order`, `toolchange_cyclic_first_layer`, `interface_shells`, `mmu_segmented_region_max_width`, `mmu_segmented_region_interlocking_depth`, `interlocking_beam_width`, `interlocking_orientation`, `interlocking_beam_layer_count`, `interlocking_depth`, `interlocking_boundary_avoidance`

### Others — `TabPrint::build`

- **Skirt** : `skirt_loops`, `skirt_type`, `min_skirt_length`, `skirt_distance`, `skirt_start_angle`, `skirt_speed`, `skirt_height`, `draft_shield`, `single_loop_draft_shield`
- **Brim** : `brim_type`, `brim_width`, `leading_brim_length`, `extra_brim_width`, `brim_object_gap`, `brim_flow_ratio`, `brim_use_efc_outline`, `combine_brims`, `brim_ears_max_angle`, `brim_ears_detection_length`, `brim_ears_outer_only`
- **Special mode** : `slicing_mode`, `print_sequence`, `print_order`, `spiral_mode`, `spiral_mode_smooth`, `spiral_mode_max_xy_smoothing`, `spiral_starting_flow_ratio`, `spiral_finishing_flow_ratio`, `timelapse_type`, `enable_wrapping_detection`
- **Fuzzy skin** : `fuzzy_skin`, `fuzzy_skin_mode`, `fuzzy_skin_noise_type`, `fuzzy_skin_point_distance`, `fuzzy_skin_thickness`, `fuzzy_skin_scale`, `fuzzy_skin_octaves`, `fuzzy_skin_persistence`, `fuzzy_skin_ripples_per_layer`, `fuzzy_skin_ripple_offset`, `fuzzy_skin_layers_between_ripple_offset`, `fuzzy_skin_first_layer`
- **G-code output** : `reduce_infill_retraction`, `gcode_add_line_number`, `gcode_comments`, `high_precision_gcode`, `gcode_label_objects`, `exclude_object`, `exclude_object_tolerance`, `filename_format`
- **Change extrusion role G-code** : `process_change_extrusion_role_gcode`
- **Post-processing Scripts** : `post_process`
- **Slicing Pipeline Plugin** : `slicing_pipeline_plugin`
- **Plugin Configuration** : `print_plugin_config_overrides`
- **Notes** : `notes`

### Frequent — `TabPrintModel::build`

- **** : `layer_height`, `sparse_infill_density`, `wall_loops`, `enable_support`

### Plate Settings — `TabPrintPlate::build`

- **** : `curr_bed_type`, `skirt_start_angle`, `print_sequence`, `spiral_mode`, `first_layer_sequence_choice`, `other_layers_sequence_choice`

### Filament — `TabFilament::build`

- **Basic information** : `filament_type`, `filament_vendor`, `filament_soluble`, `filament_is_support`, `filament_change_length`, `required_nozzle_HRC`, `default_filament_colour`, `filament_diameter`, `filament_adhesiveness_category`, `filament_density`, `filament_shrink`, `filament_shrinkage_compensation_z`, `filament_cost`, `temperature_vitrification`, `idle_temperature`, `nozzle_temperature_range_low`, `nozzle_temperature_range_high`
- **Flow ratio and Pressure Advance** : `pellet_flow_coefficient`, `filament_flow_ratio`, `enable_pressure_advance`, `pressure_advance`, `adaptive_pressure_advance`, `adaptive_pressure_advance_overhangs`, `adaptive_pressure_advance_bridges`, `adaptive_pressure_advance_model`
- **Print chamber temperature** : `activate_chamber_temp_control`, `chamber_temperature`, `chamber_minimal_temperature`
- **Print temperature** : `nozzle_temperature_initial_layer`, `nozzle_temperature`
- **Bed temperature** : `supertack_plate_temp_initial_layer`, `supertack_plate_temp`, `cool_plate_temp_initial_layer`, `cool_plate_temp`, `textured_cool_plate_temp_initial_layer`, `textured_cool_plate_temp`, `eng_plate_temp_initial_layer`, `eng_plate_temp`, `hot_plate_temp_initial_layer`, `hot_plate_temp`, `textured_plate_temp_initial_layer`, `textured_plate_temp`
- **Volumetric speed limitation** : `filament_adaptive_volumetric_speed`, `filament_max_volumetric_speed`, `filament_max_external_volumetric_speed`

### Cooling — `TabFilament::build`

- **Cooling for specific layer** : `close_fan_the_first_x_layers`, `initial_layer_fan_speed`, `full_fan_speed_layer`
- **Part cooling fan** : `fan_min_speed`, `fan_cooling_layer_time`, `fan_max_speed`, `slow_down_layer_time`, `reduce_fan_stop_start_freq`, `slow_down_for_layer_cooling`, `dont_slow_down_outer_wall`, `slow_down_min_speed`, `enable_overhang_bridge_fan`, `overhang_fan_threshold`, `overhang_fan_speed`, `internal_bridge_fan_speed`, `support_material_interface_fan_speed`, `ironing_fan_speed`
- **Auxiliary part cooling fan** : `additional_cooling_fan_speed`
- **Exhaust fan** : `activate_air_filtration`, `activate_air_filtration_during_print`, `during_print_exhaust_fan_speed`, `activate_air_filtration_on_completion`, `complete_print_exhaust_fan_speed`

### Setting Overrides — `TabFilament::add_filament_overrides_page`

- **Retraction** : `filament_retraction_length`, `filament_z_hop`, `filament_z_hop_types`, `filament_retract_lift_above`, `filament_retract_lift_below`, `filament_retract_lift_enforce`, `filament_retraction_speed`, `filament_deretraction_speed`, `filament_retract_restart_extra`, `filament_retraction_minimum_travel`, `filament_retract_when_changing_layer`, `filament_wipe`, `filament_wipe_distance`, `filament_retract_before_wipe`, `filament_retract_after_wipe`, `filament_long_retractions_when_cut`, `filament_retraction_distances_when_cut`
- **Retraction when switching material** : `filament_retract_length_toolchange`, `filament_retract_restart_extra_toolchange`
- **Ironing** : `filament_ironing_flow`, `filament_ironing_spacing`, `filament_ironing_inset`, `filament_ironing_speed`

### Advanced — `TabFilament::build`

- **Filament start G-code** : `filament_start_gcode`
- **Change extrusion role G-code** : `filament_change_extrusion_role_gcode`
- **Filament end G-code** : `filament_end_gcode`
- **Plugin Configuration** : `filament_plugin_config_overrides`

### Multimaterial — `TabFilament::build`

- **Wipe tower parameters** : `filament_minimal_purge_on_wipe_tower`, `filament_tower_interface_pre_extrusion_dist`, `filament_tower_interface_pre_extrusion_length`, `filament_tower_ironing_area`, `filament_tower_interface_purge_volume`, `filament_tower_interface_print_temp`
- **Prime volume** : `filament_prime_volume`, `filament_prime_volume_nc`
- **Multi Filament** : `long_retractions_when_ec`, `retraction_distances_when_ec`
- **Tool change parameters with single extruder MM printers** : `filament_loading_speed_start`, `filament_loading_speed`, `filament_unloading_speed_start`, `filament_unloading_speed`, `filament_toolchange_delay`, `filament_cooling_moves`, `filament_cooling_initial_speed`, `filament_cooling_final_speed`, `filament_stamping_loading_speed`, `filament_stamping_distance`
- **Tool change parameters with multi extruder MM printers** : `filament_multitool_ramming`, `filament_multitool_ramming_volume`, `filament_multitool_ramming_flow`

### Dependencies — `TabFilament::build`

- **Compatible printers** : `compatible_printers_condition`
- **Compatible process profiles** : `compatible_prints_condition`

### Notes — `TabFilament::build`

- **Notes** : `filament_notes`

### Basic information — `TabPrinter::build_fff`

- **Printable space** : `parallel_printheads_count`, `bed_exclude_area`, `bed_exclude_volumes`, `bed_exclude_volume_mode`, `printable_height`, `support_multi_bed_types`, `best_object_pos`, `z_offset`, `preferred_orientation`
- **Advanced** : `printer_structure`, `gcode_flavor`, `gcode_skip_config_block`, `pellet_modded_printer`, `bbl_use_printhost`, `printer_agent`, `use_3mf`, `scan_first_layer`, `enable_power_loss_recovery`, `disable_m73`
- **Belt printer** : `belt_printer`, `belt_printer_infinite_y`, `belt_slice_rotation_angle`, `belt_slice_rotation`, `belt_support_floor_offset`
- **Machine frame transforms** : `gcode_remap_x`, `gcode_remap_y`, `gcode_remap_z`, `belt_frame_tilt_decouple`, `belt_frame_tilt_angle`, `thumbnails`, `use_relative_e_distances`, `use_firmware_retraction`, `time_cost`, `build_plate_tilt_x`, `build_plate_tilt_y`
- **Plugin Configuration** : `printer_plugin_config_overrides`
- **Cooling Fan** : `fan_speedup_time`, `fan_speedup_overhangs`, `fan_kickstart`, `part_cooling_fan_min_pwm`
- **Extruder Clearance** : `extruder_clearance_radius`, `extruder_clearance_dist_to_rod`, `extruder_clearance_height_to_rod`, `extruder_clearance_height_to_lid`
- **Adaptive bed mesh** : `bed_mesh_min`, `bed_mesh_max`, `bed_mesh_probe_distance`, `adaptive_bed_mesh_margin`
- **Accessory** : `nozzle_hrc`, `auxiliary_fan`, `fan_direction`, `support_chamber_temp_control`, `support_air_filtration`, `cooling_filter_enabled`

### Machine G-code — `TabPrinter::build_fff`

- **File header G-code** : `file_start_gcode`
- **Machine start G-code** : `machine_start_gcode`
- **Machine end G-code** : `machine_end_gcode`
- **Printing by object G-code** : `printing_by_object_gcode`
- **Before layer change G-code** : `before_layer_change_gcode`
- **Layer change G-code** : `layer_change_gcode`
- **Timelapse G-code** : `time_lapse_gcode`
- **Clumping Detection G-code** : `wrapping_detection_gcode`
- **Change filament G-code** : `change_filament_gcode`
- **Change extrusion role G-code** : `change_extrusion_role_gcode`
- **Pause G-code** : `machine_pause_gcode`
- **Template Custom G-code** : `template_custom_gcode`

### Notes — `TabPrinter::build_fff`

- **Notes** : `printer_notes`

### Motion ability — `TabPrinter::build_kinematics_page`

- **Advanced** : `emit_machine_limits_to_gcode`
- **Resonance Compensation** : `resonance_avoidance`, `min_resonance_avoidance_speed`, `max_resonance_avoidance_speed`, `input_shaping_emit`, `input_shaping_type`, `input_shaping_freq_x`, `input_shaping_freq_y`, `input_shaping_damp_x`, `input_shaping_damp_y`
- **Speed limitation** : `machine_max_speed_x`, `machine_max_speed_y`, `machine_max_speed_z`, `machine_max_speed_e`
- **Acceleration limitation** : `machine_max_acceleration_x`, `machine_max_acceleration_y`, `machine_max_acceleration_z`, `machine_max_acceleration_e`, `machine_max_acceleration_extruding`, `machine_max_acceleration_retracting`, `machine_max_acceleration_travel`
- **Jerk limitation** : `machine_max_junction_deviation`, `machine_max_jerk_x`, `machine_max_jerk_y`, `machine_max_jerk_z`, `machine_max_jerk_e`

### Multimaterial — `TabPrinter::build_unregular_pages`

- **Single extruder multi-material setup** : `single_extruder_multi_material`, *dynamique:option*, `manual_filament_change`, `bed_temperature_formula`
- **Wipe tower** : `wipe_tower_type`, `purge_in_prime_tower`, `enable_filament_ramming`, `tool_change_on_wipe_tower`, `wait_for_temp_on_wipe_tower`
- **Belt purge tower** : `enable_belt_purge_tower`
- **Single extruder multi-material parameters** : `cooling_tube_retraction`, `cooling_tube_length`, `parking_pos_retraction`, `extra_loading_move`, `high_current_on_filament_swap`
- **Advanced** : `machine_load_filament_time`, `machine_unload_filament_time`, `machine_tool_change_time`
- **IDEX/IQEX Configuration** : `is_imex`, `imex_firmware_managed_zones`, `imex_gantry_count`, `imex_tools_per_gantry`, `imex_tool_layout`, `imex_nozzle_clearance_x`, `imex_nozzle_clearance_y`, `imex_carriage_margin`, `imex_viz_theme`

### Extruder — `TabPrinter::build_unregular_pages`

- **Basic information** : `nozzle_diameter`, `nozzle_type`, `nozzle_volume`, `extruder_printable_height`, `extruder_printable_area`, `extruder_bed_exclude_volumes`
- **Layer height limits** : `min_layer_height`, `max_layer_height`
- **Position** : `extruder_offset`
- **Retraction** : (widgets spécifiques)
- **Z-Hop** : (widgets spécifiques)
- **Retraction when switching material** : `retract_length_toolchange`, `retract_restart_extra_toolchange`, `long_retractions_when_cut`, `retraction_distances_when_cut`

## B.2 Menus principaux

| Menu (variable) | Libellé | Raccourci |
|---|---|---|
| menu | New Window |  |
| helpMenu | Keyboard Shortcuts | ? |
| helpMenu | Setup Wizard |  |
| helpMenu | Show Configuration Folder |  |
| helpMenu | Migrate Filament Presets |  |
| helpMenu | Troubleshoot Center |  |
| helpMenu | Open Network Test |  |
| helpMenu | Benchmark 3D Scene |  |
| helpMenu | Show Tip of the Day |  |
| helpMenu | Check for Updates |  |
| publish_menu | Upload Models |  |
| publish_menu | Download Models |  |
| view_menu | Default View | Ctrl+0 |
| view_menu | Top | Ctrl+1 |
| view_menu | Bottom | Ctrl+2 |
| view_menu | Front | Ctrl+3 |
| view_menu | Rear | Ctrl+4 |
| view_menu | Left | Ctrl+5 |
| view_menu | Right | Ctrl+6 |
| fileMenu | New Window |  |
| fileMenu | New Project | Ctrl+N |
| fileMenu | Open Project… | Ctrl+O |
| fileMenu | Save Project | Ctrl+S |
| fileMenu | Save Project as… | Ctrl+Shift+S |
| fileMenu | Publish 3MF… | Ctrl+Shift+E |
| import_menu | Import 3MF/STL/STEP/SVG/OBJ/AMF… | Ctrl+I |
| import_menu | Import ZIP Archive… |  |
| import_menu | Import Configs… |  |
| export_menu | Export all objects as one STL… |  |
| export_menu | Export all objects as STLs… |  |
| export_menu | Export all objects as one DRC… |  |
| export_menu | Export all objects as DRCs… |  |
| export_menu | Export Generic 3MF… |  |
| export_menu | Export plate sliced file… | Ctrl+G |
| export_menu | Export all plate sliced file… |  |
| export_menu | Export G-code… |  |
| export_menu | Export toolpaths as OBJ… |  |
| export_menu | Export Preset Bundle… |  |
| fileMenu | Quit | Ctrl+Q |
| fileMenu | Quit |  |
| editMenu | Undo | Ctrl+Z |
| editMenu | Redo | Ctrl+Y |
| editMenu | Cut | Ctrl+X |
| editMenu | Copy | Ctrl+C |
| editMenu | Paste | Ctrl+V |
| editMenu | Delete Selected | Del |
| editMenu | Delete All | Ctrl+D |
| editMenu | Clone Selected | Ctrl+K |
| editMenu | Duplicate Current Plate |  |
| editMenu | Duplicate Current Plate |  |
| editMenu | Select All | Ctrl+A |
| editMenu | Deselect AllEsc |  |
| viewMenu | Use Perspective View |  |
| viewMenu | Use Orthogonal View |  |
| viewMenu | Auto Perspective |  |
| viewMenu | Show &G-code Window |  |
| viewMenu | Show 3D Navigator |  |
| viewMenu | Show Gridlines |  |
| viewMenu | Show IDEX/IQEX Toolhead |  |
| viewMenu | Reset Window Layout |  |
| viewMenu | Show 3Dconnexion Settings |  |
| viewMenu | Show &Labels |  |
| viewMenu | Show all collision volumes |  |
| viewMenu | Show &Overhang |  |
| viewMenu | Show Selected Outline (beta) |  |
| parent_menu | Preferences | Ctrl+P |
| m_topbar->GetTopMenu() | Preferences | Ctrl+P |
| top_menu | Open Speed Dial | Space |
| top_menu | Preset Bundle |  |
| top_menu | Sync Presets |  |
| top_menu | Plugins |  |
| m_topbar->GetCalibMenu() | Temperature |  |
| m_topbar->GetCalibMenu() | Max flowrate |  |
| m_topbar->GetCalibMenu() | Pressure advance |  |
| m_topbar->GetCalibMenu() | Flow ratio |  |
| m_topbar->GetCalibMenu() | Retraction |  |
| m_topbar->GetCalibMenu() | Cornering |  |
| input_shaping_menu | Input Shaping Frequency |  |
| input_shaping_menu | Input Shaping Damping/zeta factor |  |
| m_topbar->GetCalibMenu() | VFA |  |
| m_topbar->GetCalibMenu() | Calibration Guide |  |
| fileMenu | Open Speed Dial | Space |
| fileMenu | Preset Bundle |  |
| fileMenu | Sync Presets |  |
| fileMenu | Plugins |  |
| calib_menu | Temperature |  |
| calib_menu | Max flowrate |  |
| calib_menu | Pressure advance |  |
| calib_menu | Flow ratio |  |
| calib_menu | Retraction |  |
| calib_menu | Cornering |  |
| input_shaping_menu | Input Shaping Frequency |  |
| input_shaping_menu | Input Shaping Damping/zeta factor |  |
| calib_menu | VFA |  |
| calib_menu | Calibration Guide |  |
| fileMenu | &Open G-code… | Ctrl+O |
| fileMenu | Re&load from Disk… | Ctrl+Shift+R |
| fileMenu | Re&load from DiskF5 |  |
| fileMenu | Export &Toolpaths as OBJ… |  |
| fileMenu | Open &Slicer… |  |
| fileMenu | &Quit |  |

## B.3 Menus contextuels du plateau

| Menu (variable) | Libellé |
|---|---|
| menu | HideShow |
| menu | Delete |
| menu | Delete |
| sub_menu | Load... |
| sub_menu | Cube |
| sub_menu | Cylinder |
| sub_menu | Sphere |
| sub_menu | Cone |
| sub_menu | Disc |
| sub_menu | Torus |
| menu | Height Range Modifier |
| menu | Set as An Individual Object |
| menu | Fill bed with copies |
| menu | Printable |
| menu | Rename |
| menu | Fix Model |
| menu | Export as one STL… |
| menu | Export as STLs… |
| menu | Export as one DRC… |
| menu | Export as DRCs… |
| menu | Reload from disk |
| menu | Replace 3D file… |
| menu | Replace all with 3D files… |
| menu | Scale to build volume |
| flush_options_menu | Flush into objects' infill |
| flush_options_menu | Flush into this object |
| flush_options_menu | Flush into objects' support |
| menu | Assemble |
| menu | Assemble |
| sub_menu | Mirror: |
| sub_menu | Along X Axis |
| sub_menu | Along Y Axis |
| sub_menu | Along Z Axis |
| sub_menu | Split: |
| sub_menu | To Objects |
| sub_menu | To Parts |
| sub_menu | Mesh boolean |
| &m_default_menu | Add Models |
| &m_default_menu | Show Labels |
| &m_sla_object_menu | Auto orientation |
| menu | Edit Painting |
| menu | Delete |
| menu | Edit |
| menu | Decompose Color |
| menu | Delete |
| menu | Select All |
| menu | Select All Plates |
| menu | Delete All |
| menu | Arrange |
| menu | Auto Rotate |
| modify_menu | Fix All |
| modify_menu | Fix All Plates |
| menu | Add Models |
| menu | Reload All |
| menu | Add instance |
| menu | Remove instance |
| menu | Set number of instances… |
| menu | Fill bed with instances… |
| menu | Clone |
| menu | Simplify Model |
| menu | Subdivision mesh |
| menu | Center |
| menu | Drop |
| menu | Printable |

## B.4 Barres d'outils 3D

- `GLCanvas3D::_init_main_toolbar` : `add`, `addplate`, `orient`, `arrange`, `more`, `fewer`, `splitobjects`, `splitvolumes`, `layersediting`
- `GLCanvas3D::_init_assemble_view_toolbar` : `assembly_view`
- `GLCanvas3D::_init_separator_toolbar` : `start_seperator`

## B.5 Gizmos

- **Move** (`GLGizmoMove3D`)
- **Rotate** (`GLGizmoRotate3D`)
- **Scale** (`GLGizmoScale3D`)
- **Flatten** (`GLGizmoFlatten`)
- **Cut** (`GLGizmoCut3D`)
- **MeshBoolean** (`GLGizmoMeshBoolean`)
- **FdmSupports** (`GLGizmoFdmSupports`)
- **Seam** (`GLGizmoSeam`)
- **FuzzySkin** (`GLGizmoFuzzySkin`)
- **MmSegmentation** (`GLGizmoMmuSegmentation`)
- **PaintedModifier** (`GLGizmoPaintedModifier`)
- **TextureDisplacement** (`GLGizmoTextureDisplacement`)
- **Emboss** (`GLGizmoEmboss`)
- **GLGizmoSVG** (`GLGizmoSVG`)
- **Measure** (`GLGizmoMeasure`)
- **Assembly** (`GLGizmoAssembly`)
- **Simplify** (`GLGizmoSimplify`)
- **BrimEars** (`GLGizmoBrimEars`)

## B.6 Raccourcis clavier

### Project

| Touches | Action |
|---|---|
| `Ctrl+N` | New Project |
| `Ctrl+O` | Open Project |
| `Ctrl+S` | Save Project |
| `Ctrl+Shift+S` | Save Project as |
| `Ctrl+I` | Import geometry data from STL/STEP/3MF/OBJ/AMF files |
| `Ctrl+Shift+E` | Publish 3MF |

### Slicing and printing

| Touches | Action |
|---|---|
| `Ctrl+R` | Slice plate |
| `Ctrl+G` | Export plate sliced file |
| `Ctrl+Shift+G` | Print plate |
| `Ctrl+J` | Print host upload queue |

### Selection

| Touches | Action |
|---|---|
| `Ctrl+A` | Select all objects |
| `Ctrl+Shift+A` | Select all objects on all plates |

### Editing

| Touches | Action |
|---|---|
| `Ctrl+Z` | Undo |
| `Ctrl+Y` | Redo |
| `Ctrl+X` | Cut |
| `Ctrl+C` | Copy to clipboard |
| `Ctrl+V` | Paste from clipboard |
| `Del` | Delete Selected |
| `Ctrl+D` | Delete All |
| `Ctrl+K` | Clone Selected |

### Objects

| Touches | Action |
|---|---|
| `+` | Add instance |
| `-` | Remove instance |
| `V` | Toggle printable for object/part |
| `D` | Auto Drop |
| `Ctrl+Shift+R` | Reload from disk |

### Placement

| Touches | Action |
|---|---|
| `A` | Arrange all objects |
| `Shift+A` | Arrange objects on selected plates |
| `Q` | Auto orient all/selected objects |
| `Shift+Q` | Auto orient all objects on current plate |
| `Left` | Move selection 10mm in negative X direction |
| `Right` | Move selection 10mm in positive X direction |
| `Up` | Move selection 10mm in positive Y direction |
| `Down` | Move selection 10mm in negative Y direction |
| `PageUp` | Rotate selection 45 degrees counterclockwise |
| `PageDown` | Rotate selection 45 degrees clockwise |

### Gizmos

| Touches | Action |
|---|---|
| `M` | Gizmo move |
| `R` | Gizmo rotate |
| `S` | Gizmo scale |
| `F` | Gizmo place face on bed |
| `C` | Gizmo cut |
| `B` | Gizmo mesh boolean |
| `L` | Gizmo FDM paint-on supports |
| `P` | Gizmo FDM paint-on seam |
| `H` | Gizmo FDM paint-on fuzzy skin |
| `N` | Gizmo multi-material painting |
| `T` | Gizmo text emboss/engrave |
| `U` | Gizmo measure |
| `Y` | Gizmo assemble |
| `E` | Gizmo brim ears |
| `D` | Gizmo texture displacement painting |

### Sliders

| Touches | Action |
|---|---|
| `Shift+G` | Jump to layer |
| `Up` | Vertical slider - Move active thumb Up |
| `Down` | Vertical slider - Move active thumb Down |
| `Left` | Horizontal slider - Move active thumb Left |
| `Right` | Horizontal slider - Move active thumb Right |
| `Home` | Horizontal slider - Move to start position |
| `End` | Horizontal slider - Move to last position |

### Painting tools

| Touches | Action |
|---|---|
| `C` | Circle |
| `S` | Sphere |
| `F` | Fill |
| `G` | Gap Fill |
| `T` | Triangle |
| `H` | Height Range |

### Camera

| Touches | Action |
|---|---|
| `Ctrl+0` | Camera view - Default |
| `Ctrl+1` | Camera view - Top |
| `Ctrl+2` | Camera view - Bottom |
| `Ctrl+3` | Camera view - Front |
| `Ctrl+4` | Camera view - Behind |
| `Ctrl+5` | Camera Angle - Left side |
| `Ctrl+6` | Camera Angle - Right side |
| `Ctrl+7` | Camera view - Current plate |
| `I` | Zoom in |
| `O` | Zoom out |
| `Ctrl+M` | Show/Hide 3Dconnexion devices settings dialog |

### Display

| Touches | Action |
|---|---|
| `Ctrl+E` | Show object labels in 3D scene. |
| `Ctrl+Shift+K` | Show all collision volumes |
| `Ctrl+Shift+Return` | Show/Hide wireframe |
| `C` | On/Off G-code window |
| `L` | On/Off one layer mode of the vertical slider |
| `B` | Show raw G-code (belt only) |

### Application

| Touches | Action |
|---|---|
| `Ctrl+P` | Preferences |
| `Ctrl+F` | Search |
| `Tab` | Switch between Prepare/Preview |
| `Shift+Tab` | Collapse/Expand the sidebar |
| `` | Go to the Home tab |
| `` | Go to the Design tab |
| `` | Go to the Prepare tab |
| `` | Go to the Preview tab |
| `` | Go to the Device tab |
| `` | Go to the Multi-device tab |
| `` | Go to the Project tab |
| `` | Go to the Calibration tab |
| `Ctrl+Tab` | Switch to the next tab |
| `Ctrl+Shift+Tab` | Switch to the previous tab |
| `F5` | Reload the device page |
| `?` | Show keyboard shortcuts list |
| `Ctrl+Q` | Quit |

### Speed Dial

| Touches | Action |
|---|---|
| `Space` | Open the Speed Dial |
