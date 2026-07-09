# Annexe B — Inventaire exhaustif de l'interface (normatif)

Source : `audit/ui_inventory.json`. Chaque élément DOIT exister dans
l'application web (même organisation, mêmes groupes) ou figurer au
registre d'exclusions avec justification.

## B.1 Onglets de réglages

### Quality — `TabPrint::build`

- **Layer height** : `layer_height`, `initial_layer_print_height`
- **Line width** : `line_width`, `initial_layer_line_width`, `outer_wall_line_width`, `inner_wall_line_width`, `top_surface_line_width`, `sparse_infill_line_width`, `internal_solid_infill_line_width`, `support_line_width`, `bridge_line_width`
- **Seam** : `seam_position`, `staggered_inner_seams`, `seam_gap`, `seam_slope_type`, `seam_slope_conditional`, `scarf_angle_threshold`, `scarf_overhang_threshold`, `scarf_joint_speed`, `seam_slope_start_height`, `seam_slope_entire_loop`, `seam_slope_min_length`, `seam_slope_steps`, `scarf_joint_flow_ratio`, `seam_slope_inner_walls`, `role_based_wipe_speed`, `wipe_speed`, `wipe_on_loops`, `wipe_before_external_loop`
- **Precision** : `slice_closing_radius`, `resolution`, `enable_arc_fitting`, `xy_hole_compensation`, `xy_contour_compensation`, `elefant_foot_compensation`, `elefant_foot_layers_density`, `elefant_foot_compensation_layers`, `precise_outer_wall`, `precise_z_height`, `hole_to_polyhole`, `hole_to_polyhole_threshold`, `hole_to_polyhole_twisted`
- **Ironing** : `ironing_type`, `ironing_pattern`, `ironing_flow`, `ironing_spacing`, `ironing_inset`, `ironing_angle`, `ironing_angle_fixed`
- **Z contouring** : `zaa_enabled`, `zaa_minimize_perimeter_height`, `zaa_min_z`, `zaa_dont_alternate_fill_direction`
- **Wall generator** : `wall_generator`, `wall_transition_angle`, `wall_transition_filter_deviation`, `wall_transition_length`, `wall_distribution_count`, `initial_layer_min_bead_width`, `min_bead_width`, `min_feature_size`, `min_length_factor`, `wall_maximum_resolution`, `wall_maximum_deviation`
- **Walls and surfaces** : `wall_sequence`, `is_infill_first`, `wall_direction`, `print_flow_ratio`, `top_solid_infill_flow_ratio`, `bottom_solid_infill_flow_ratio`, `set_other_flow_ratios`, `first_layer_flow_ratio`, `outer_wall_flow_ratio`, `inner_wall_flow_ratio`, `overhang_flow_ratio`, `sparse_infill_flow_ratio`, `internal_solid_infill_flow_ratio`, `gap_fill_flow_ratio`, `support_flow_ratio`, `support_interface_flow_ratio`, `only_one_wall_first_layer`, `only_one_wall_top`, `min_width_top_surface`, `reduce_crossing_wall`, `max_travel_detour_distance`, `small_area_infill_flow_compensation`, *dynamique:option*
- **Bridging** : `bridge_flow`, `internal_bridge_flow`, `bridge_density`, `internal_bridge_density`, `thick_bridges`, `thick_internal_bridges`, `enable_extra_bridge_layer`, `dont_filter_internal_bridges`, `counterbore_hole_bridging`
- **Overhangs** : `detect_overhang_wall`, `make_overhang_printable`, `make_overhang_printable_angle`, `make_overhang_printable_hole_size`, `extra_perimeters_on_overhangs`, `overhang_reverse`, `overhang_reverse_internal_only`, `overhang_reverse_threshold`

### Strength — `TabPrint::build`

- **Walls** : `wall_loops`, `alternate_extra_wall`, `detect_thin_wall`
- **Top/bottom shells** : `top_shell_layers`, `top_shell_thickness`, `top_surface_density`, `top_surface_pattern`, `bottom_shell_layers`, `bottom_shell_thickness`, `bottom_surface_density`, `bottom_surface_pattern`, `top_bottom_infill_wall_overlap`
- **Infill** : `sparse_infill_density`, `fill_multiline`, `sparse_infill_pattern`, `gyroid_optimized`, `infill_direction`, `sparse_infill_rotate_template`, `skin_infill_density`, `skeleton_infill_density`, `infill_lock_depth`, `skin_infill_depth`, `skin_infill_line_width`, `skeleton_infill_line_width`, `symmetric_infill_y_axis`, `infill_shift_step`, `lateral_lattice_angle_1`, `lateral_lattice_angle_2`, `infill_overhang_angle`, `lightning_overhang_angle`, `lightning_prune_angle`, `lightning_straightening_angle`, `infill_anchor_max`, `infill_anchor`, `internal_solid_infill_pattern`, `solid_infill_direction`, `solid_infill_rotate_template`, `gap_fill_target`, `filter_out_gap_fill`, `infill_wall_overlap`
- **Advanced** : `align_infill_direction_to_model`, `extra_solid_infills`, `bridge_angle`, `internal_bridge_angle`, `relative_bridge_angle`, `minimum_sparse_infill_area`, `infill_combination`, `infill_combination_max_layer_height`, `detect_narrow_internal_solid_infill`, `ensure_vertical_shell_thickness`

### Speed — `TabPrint::build`

- **First layer speed** : `initial_layer_speed`, `initial_layer_infill_speed`, `initial_layer_travel_speed`, `slow_down_layers`
- **Other layers speed** : `outer_wall_speed`, `inner_wall_speed`, `small_perimeter_speed`, `small_perimeter_threshold`, `sparse_infill_speed`, `internal_solid_infill_speed`, `top_surface_speed`, `gap_infill_speed`, `ironing_speed`, `support_speed`, `support_interface_speed`
- **Overhang speed** : `enable_overhang_speed`, `slowdown_for_curled_perimeters`
- **Travel speed** : `travel_speed`
- **Acceleration** : `default_acceleration`, `outer_wall_acceleration`, `inner_wall_acceleration`, `bridge_acceleration`, `sparse_infill_acceleration`, `internal_solid_infill_acceleration`, `initial_layer_acceleration`, `initial_layer_travel_acceleration`, `top_surface_acceleration`, `travel_acceleration`, `accel_to_decel_enable`, `accel_to_decel_factor`
- **Junction Deviation** : `default_junction_deviation`
- **Jerk(XY)** : `default_jerk`, `outer_wall_jerk`, `inner_wall_jerk`, `infill_jerk`, `top_surface_jerk`, `initial_layer_jerk`, `initial_layer_travel_jerk`, `travel_jerk`
- **Advanced** : `max_volumetric_extrusion_rate_slope`, `max_volumetric_extrusion_rate_slope_segment_length`, `extrusion_rate_smoothing_external_perimeter_only`

### Support — `TabPrint::build`

- **Support** : `enable_support`, `support_type`, `support_style`, `support_threshold_angle`, `support_threshold_overlap`, `raft_first_layer_density`, `raft_first_layer_expansion`, `support_on_build_plate_only`, `support_critical_regions_only`, `support_remove_small_overhang`
- **Raft** : `raft_layers`, `raft_contact_distance`
- **Support filament** : `support_filament`, `support_interface_filament`, `support_interface_not_for_body`
- **Support ironing** : `support_ironing`, `support_ironing_pattern`, `support_ironing_flow`, `support_ironing_spacing`
- **Advanced** : `support_top_z_distance`, `support_bottom_z_distance`, `tree_support_wall_count`, `support_base_pattern`, `support_base_pattern_spacing`, `support_angle`, `support_interface_top_layers`, `support_interface_bottom_layers`, `support_interface_pattern`, `support_interface_spacing`, `support_bottom_interface_spacing`, `support_expansion`, `support_object_xy_distance`, `support_object_first_layer_gap`, `bridge_no_support`, `max_bridge_length`, `independent_support_layer_height`
- **Tree supports** : `tree_support_tip_diameter`, `tree_support_branch_distance`, `tree_support_branch_distance_organic`, `tree_support_top_rate`, `tree_support_branch_diameter`, `tree_support_branch_diameter_organic`, `tree_support_branch_diameter_angle`, `tree_support_branch_angle`, `tree_support_branch_angle_organic`, `tree_support_angle_slow`, `tree_support_auto_brim`, `tree_support_brim_width`

### Multimaterial — `TabPrint::build`

- **Prime tower** : `enable_prime_tower`, `prime_tower_skip_points`, `enable_tower_interface_features`, `enable_tower_interface_cooldown_during_tower`, `prime_tower_enable_framework`, `prime_tower_width`, `prime_volume`, `prime_tower_brim_width`, `prime_tower_infill_gap`, `wipe_tower_rotation_angle`, `wipe_tower_bridging`, `wipe_tower_extra_spacing`, `wipe_tower_extra_flow`, `wipe_tower_max_purge_speed`, `wipe_tower_wall_type`, `wipe_tower_cone_angle`, `wipe_tower_extra_rib_length`, `wipe_tower_rib_width`, `wipe_tower_fillet_wall`, `wipe_tower_no_sparse_layers`, `single_extruder_multi_material_priming`
- **Filament for Features** : `outer_wall_filament_id`, `inner_wall_filament_id`, `sparse_infill_filament_id`, `internal_solid_filament_id`, `top_surface_filament_id`, `bottom_surface_filament_id`, `wipe_tower_filament`
- **Ooze prevention** : `ooze_prevention`, `standby_temperature_delta`, `preheat_time`, `preheat_steps`
- **Flush options** : `flush_into_infill`, `flush_into_objects`, `flush_into_support`
- **Advanced** : `interlocking_beam`, `interface_shells`, `mmu_segmented_region_max_width`, `mmu_segmented_region_interlocking_depth`, `interlocking_beam_width`, `interlocking_orientation`, `interlocking_beam_layer_count`, `interlocking_depth`, `interlocking_boundary_avoidance`

### Others — `TabPrint::build`

- **Skirt** : `skirt_loops`, `skirt_type`, `min_skirt_length`, `skirt_distance`, `skirt_start_angle`, `skirt_speed`, `skirt_height`, `draft_shield`, `single_loop_draft_shield`
- **Brim** : `brim_type`, `brim_width`, `brim_object_gap`, `brim_flow_ratio`, `brim_use_efc_outline`, `combine_brims`, `brim_ears_max_angle`, `brim_ears_detection_length`
- **Special mode** : `slicing_mode`, `print_sequence`, `print_order`, `spiral_mode`, `spiral_mode_smooth`, `spiral_mode_max_xy_smoothing`, `spiral_starting_flow_ratio`, `spiral_finishing_flow_ratio`, `timelapse_type`, `enable_wrapping_detection`
- **Fuzzy Skin** : `fuzzy_skin`, `fuzzy_skin_mode`, `fuzzy_skin_noise_type`, `fuzzy_skin_point_distance`, `fuzzy_skin_thickness`, `fuzzy_skin_scale`, `fuzzy_skin_octaves`, `fuzzy_skin_persistence`, `fuzzy_skin_ripples_per_layer`, `fuzzy_skin_ripple_offset`, `fuzzy_skin_layers_between_ripple_offset`, `fuzzy_skin_first_layer`
- **G-code output** : `reduce_infill_retraction`, `gcode_add_line_number`, `gcode_comments`, `gcode_label_objects`, `exclude_object`, *dynamique:option*
- **Change extrusion role G-code** : *dynamique:option*
- **Post-processing Scripts** : *dynamique:option*
- **Notes** : *dynamique:option*

### Frequent — `TabPrintModel::build`

- **** : `layer_height`, `sparse_infill_density`, `wall_loops`, `enable_support`

### Plate Settings — `TabPrintPlate::build`

- **** : `curr_bed_type`, `skirt_start_angle`, `print_sequence`, `spiral_mode`, `first_layer_sequence_choice`, `other_layers_sequence_choice`

### Setting Overrides — `TabFilament::add_filament_overrides_page`

- **Retraction** : `filament_retraction_length`, `filament_z_hop`, `filament_z_hop_types`, `filament_retract_lift_above`, `filament_retract_lift_below`, `filament_retract_lift_enforce`, `filament_retraction_speed`, `filament_deretraction_speed`, `filament_retract_restart_extra`, `filament_retraction_minimum_travel`, `filament_retract_when_changing_layer`, `filament_wipe`, `filament_wipe_distance`, `filament_retract_before_wipe`, `filament_long_retractions_when_cut`, `filament_retraction_distances_when_cut`
- **Ironing** : `filament_ironing_flow`, `filament_ironing_spacing`, `filament_ironing_inset`, `filament_ironing_speed`

### Filament — `TabFilament::build`

- **Basic information** : `filament_type`, `filament_vendor`, `filament_soluble`, `filament_is_support`, `filament_change_length`, `required_nozzle_HRC`, `default_filament_colour`, `filament_diameter`, `filament_adhesiveness_category`, `filament_density`, `filament_shrink`, `filament_shrinkage_compensation_z`, `filament_cost`, `temperature_vitrification`, `idle_temperature`
- **Flow ratio and Pressure Advance** : `pellet_flow_coefficient`, `filament_flow_ratio`, `enable_pressure_advance`, `pressure_advance`, `adaptive_pressure_advance`, `adaptive_pressure_advance_overhangs`, `adaptive_pressure_advance_bridges`, *dynamique:option*
- **Print chamber temperature** : `activate_chamber_temp_control`
- **Print temperature** : (widgets spécifiques)
- **Bed temperature** : (widgets spécifiques)
- **Volumetric speed limitation** : `filament_adaptive_volumetric_speed`, `filament_max_volumetric_speed`

### Cooling — `TabFilament::build`

- **Cooling for specific layer** : `close_fan_the_first_x_layers`, `full_fan_speed_layer`
- **Part cooling fan** : `reduce_fan_stop_start_freq`, `slow_down_for_layer_cooling`, `dont_slow_down_outer_wall`, `slow_down_min_speed`, `enable_overhang_bridge_fan`, `overhang_fan_threshold`, `overhang_fan_speed`, `internal_bridge_fan_speed`, `support_material_interface_fan_speed`, `ironing_fan_speed`
- **Auxiliary part cooling fan** : `additional_cooling_fan_speed`
- **Exhaust fan** : `activate_air_filtration`

### Advanced — `TabFilament::build`

- **Filament start G-code** : *dynamique:option*
- **Change extrusion role G-code** : *dynamique:option*
- **Filament end G-code** : *dynamique:option*

### Multimaterial — `TabFilament::build`

- **Wipe tower parameters** : `filament_minimal_purge_on_wipe_tower`, `filament_tower_interface_pre_extrusion_dist`, `filament_tower_interface_pre_extrusion_length`, `filament_tower_ironing_area`, `filament_tower_interface_purge_volume`, `filament_tower_interface_print_temp`
- **Multi Filament** : `long_retractions_when_ec`, `retraction_distances_when_ec`
- **Tool change parameters with single extruder MM printers** : `filament_loading_speed_start`, `filament_loading_speed`, `filament_unloading_speed_start`, `filament_unloading_speed`, `filament_toolchange_delay`, `filament_cooling_moves`, `filament_cooling_initial_speed`, `filament_cooling_final_speed`, `filament_stamping_loading_speed`, `filament_stamping_distance`
- **Tool change parameters with multi extruder MM printers** : `filament_multitool_ramming`, `filament_multitool_ramming_volume`, `filament_multitool_ramming_flow`

### Dependencies — `TabFilament::build`

- **Compatible printers** : *dynamique:option*
- **Compatible process profiles** : *dynamique:option*

### Notes — `TabFilament::build`

- **Notes** : *dynamique:option*

### Basic information — `TabPrinter::build_fff`

- **Printable space** : `parallel_printheads_count`, *dynamique:option*, `printable_height`, `support_multi_bed_types`, `best_object_pos`, `z_offset`, `preferred_orientation`
- **Advanced** : `printer_structure`, `gcode_flavor`, `pellet_modded_printer`, `bbl_use_printhost`, `use_3mf`, `scan_first_layer`, `enable_power_loss_recovery`, `disable_m73`, *dynamique:option*, `use_relative_e_distances`, `use_firmware_retraction`, `time_cost`
- **Cooling Fan** : `fan_kickstart`, `part_cooling_fan_min_pwm`
- **Extruder Clearance** : `extruder_clearance_radius`, `extruder_clearance_height_to_rod`, `extruder_clearance_height_to_lid`
- **Adaptive bed mesh** : `bed_mesh_min`, `bed_mesh_max`, `bed_mesh_probe_distance`, `adaptive_bed_mesh_margin`
- **Accessory** : `nozzle_type`, `nozzle_hrc`, `auxiliary_fan`, `support_chamber_temp_control`, `support_air_filtration`

### Machine G-code — `TabPrinter::build_fff`

- **File header G-code** : *dynamique:option*
- **Machine start G-code** : *dynamique:option*
- **Machine end G-code** : *dynamique:option*
- **Printing by object G-code** : *dynamique:option*
- **Before layer change G-code** : *dynamique:option*
- **Layer change G-code** : *dynamique:option*
- **Timelapse G-code** : *dynamique:option*
- **Clumping Detection G-code** : *dynamique:option*
- **Change filament G-code** : *dynamique:option*
- **Change extrusion role G-code** : *dynamique:option*
- **Pause G-code** : *dynamique:option*
- **Template Custom G-code** : *dynamique:option*

### Notes — `TabPrinter::build_fff`

- **Notes** : *dynamique:option*

### Motion ability — `TabPrinter::build_kinematics_page`

- **** : (widgets spécifiques)
- **Advanced** : `emit_machine_limits_to_gcode`
- **Resonance Compensation** : `resonance_avoidance`, `input_shaping_emit`, `input_shaping_type`
- **Speed limitation** : (widgets spécifiques)
- **Acceleration limitation** : (widgets spécifiques)
- **Jerk limitation** : (widgets spécifiques)

### Multimaterial — `TabPrinter::build_unregular_pages`

- **Single extruder multi-material setup** : `single_extruder_multi_material`, *dynamique:option*, `manual_filament_change`, `bed_temperature_formula`
- **Wipe tower** : `wipe_tower_type`, `purge_in_prime_tower`, `enable_filament_ramming`, `tool_change_on_wipe_tower`
- **Single extruder multi-material parameters** : `cooling_tube_retraction`, `cooling_tube_length`, `parking_pos_retraction`, `extra_loading_move`, `high_current_on_filament_swap`
- **Advanced** : `machine_load_filament_time`, `machine_unload_filament_time`, `machine_tool_change_time`

### page_name — `TabPrinter::build_unregular_pages`

- **Basic information** : `nozzle_diameter`, `nozzle_volume`, `extruder_printable_height`, *dynamique:option*
- **Layer height limits** : `min_layer_height`, `max_layer_height`
- **Position** : `extruder_offset`
- **Retraction** : `retraction_length`, `retract_restart_extra`, `retraction_speed`, `deretraction_speed`, `retraction_minimum_travel`, `retract_when_changing_layer`, `wipe`, `wipe_distance`, `retract_before_wipe`
- **Z-Hop** : `retract_lift_enforce`, `z_hop_types`, `z_hop`, `travel_slope`, `retract_lift_above`, `retract_lift_below`
- **Retraction when switching material** : `retract_length_toolchange`, `retract_restart_extra_toolchange`, `long_retractions_when_cut`, `retraction_distances_when_cut`

## B.2 Menus principaux

| Menu (variable) | Libellé | Raccourci |
|---|---|---|
| menu | New Window |  |
| helpMenu | Keyboard Shortcuts&? |  |
| helpMenu | Setup Wizard |  |
| helpMenu | Show Configuration Folder |  |
| helpMenu | Troubleshoot Center |  |
| helpMenu | Open Network Test |  |
| helpMenu | Show Tip of the Day |  |
| helpMenu | Check for Updates |  |
| publish_menu | Upload Models |  |
| publish_menu | Download Models |  |
| view_menu | Default View | Ctrl+0 |
| view_menu | Top | Ctrl+1 |
| view_menu | Bottom | Ctrl+2 |
| view_menu | Front | Ctrl+3 |
| view_menu | Rear | Ctrl+4 |
| view_menu | LeftCameraCamera | Ctrl+5 |
| view_menu | RightCameraCamera | Ctrl+6 |
| fileMenu | New Window |  |
| fileMenu | New Project | Ctrl+N |
| fileMenu | Open Project… | Ctrl+O |
| fileMenu | Open Project… | Ctrl+O |
| fileMenu | Save Project | Ctrl+S |
| fileMenu | Save Project | Ctrl+S |
| fileMenu | Save Project as… | Ctrl+Shift+S |
| fileMenu | Save Project as… | Ctrl+Shift+S |
| import_menu | Import 3MF/STL/STEP/SVG/OBJ/AMF… | Ctrl+I |
| import_menu | Import 3MF/STL/STEP/SVG/OBJ/AMF… | Ctrl+I |
| import_menu | Import Zip Archive… |  |
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
| fileMenu | Sync Presets |  |
| fileMenu | Quit |  |
| fileMenu | Quit |  |
| editMenu | Undo | Ctrl+Z |
| editMenu | Redo | Ctrl+Y |
| editMenu | Cut | Ctrl+X |
| editMenu | Copy | Ctrl+C |
| editMenu | Paste | Ctrl+V |
| editMenu | Delete selected | Del |
| editMenu | Delete all | Ctrl+D |
| editMenu | Clone selected |  |
| editMenu | Duplicate Current Plate |  |
| editMenu | UndoZ |  |
| editMenu | RedoY |  |
| editMenu | CutX |  |
| editMenu | CopyC |  |
| editMenu | PasteV |  |
| editMenu | Delete selected | Backspace |
| editMenu | Delete all | Ctrl+D |
| editMenu | Clone selected | Ctrl+K |
| editMenu | Duplicate Current Plate |  |
| editMenu | Select allA |  |
| editMenu | Deselect allEsc |  |
| viewMenu | Use Perspective View |  |
| viewMenu | Use Orthogonal View |  |
| viewMenu | Auto Perspective |  |
| viewMenu | Show &G-code WindowC |  |
| viewMenu | Show 3D Navigator |  |
| viewMenu | Show Gridlines |  |
| viewMenu | Reset Window Layout |  |
| viewMenu | Show &Labels | Ctrl+E |
| viewMenu | Show &Overhang |  |
| viewMenu | Show Selected Outline (beta) |  |
| parent_menu | Preferences | Ctrl+, |
| m_topbar->GetTopMenu() | Preferences | Ctrl+P |
| m_topbar->GetTopMenu() | Preset Bundle |  |
| m_topbar->GetTopMenu() | Sync Presets |  |
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
| fileMenu | Preset Bundle |  |
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
| menu | Height range Modifier |
| menu | Set as an individual object |
| menu | Fill bed with copies |
| menu | Printable |
| menu | Rename |
| menu | Fix model |
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
| menu | Mesh boolean |
| mirror_menu | Along X axis |
| mirror_menu | Along Y axis |
| mirror_menu | Along Z axis |
| &m_default_menu | Add Models |
| &m_default_menu | Add Models |
| &m_default_menu | Show Labels |
| split_menu | To objects |
| split_menu | To parts |
| split_menu | To objects |
| split_menu | To parts |
| &m_sla_object_menu | Split |
| &m_sla_object_menu | Auto orientation |
| menu | Split |
| &m_part_menu | Split |
| split_menu | To objects |
| split_menu | To parts |
| menu | Edit |
| menu | Delete |
| menu | Select All |
| menu | Select All Plates |
| menu | Delete All |
| menu | Arrange |
| menu | Reload All |
| menu | Auto Rotate |
| menu | Delete Plate |
| menu | Delete Plate |
| menu | Add Models |
| menu | Add Models |
| split_menu | To objects |
| split_menu | To parts |
| menu | Add instance |
| menu | Remove instance |
| menu | Set number of instances… |
| menu | Fill bed with instances… |
| menu | Clone |
| menu | Simplify Model |
| menu | Subdivision mesh(Lost color) |
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
- **Emboss** (`GLGizmoEmboss`)
- **GLGizmoSVG** (`GLGizmoSVG`)
- **Measure** (`GLGizmoMeasure`)
- **Assembly** (`GLGizmoAssembly`)
- **Simplify** (`GLGizmoSimplify`)
- **BrimEars** (`GLGizmoBrimEars`)

## B.6 Raccourcis clavier

### Global shortcuts

| Touches | Action |
|---|---|
| `Ctrl+N` | New Project |
| `Ctrl+O` | Open Project |
| `Ctrl+S` | Save Project |
| `Ctrl+Shift+S` | Save Project as |
| `Ctrl+I` | Import geometry data from STL/STEP/3MF/OBJ/AMF files |
| `Ctrl+G` | Export plate sliced file |
| `Ctrl+R` | Slice plate |
| `Ctrl+Shift+G` | Print plate |
| `Ctrl+X` | Cut |
| `Ctrl+C` | Copy to clipboard |
| `Ctrl+V` | Paste from clipboard |
| `Ctrl+P` | Preferences |
| `Ctrl+Shift+M` | Show/Hide 3Dconnexion devices settings dialog |
| `Ctrl+M` | Show/Hide 3Dconnexion devices settings dialog |
| `Ctrl+Tab` | Switch table page |
| `fn+⌫` | Delete selected |
| `Del` | Delete selected |
| `?` | Show keyboard shortcuts list |

### Plater

| Touches | Action |
|---|---|
| `Mouse wheel` | Zoom View |
| `A` | Arrange all objects |
| `Shift+A` | Arrange objects on selected plates |
| `Q` | Auto orients selected objects or all objects. If there are selected objects, it just orients the selected ones. Otherwise, it will orient all objects in the current project. |
| `Shift+Q` | Auto orients all objects on the active plate. |
| `Shift+Tab` | Collapse/Expand the sidebar |
| `Ctrl+Any arrow` | Movement in camera space |
| `Alt+Left mouse button` | Select a part |
| `Ctrl+Left mouse button` | Select multiple objects |
| `Shift+Left mouse button` | Select objects by rectangle |
| `Arrow Up` | Move selection 10 mm in positive Y direction |
| `Arrow Down` | Move selection 10 mm in negative Y direction |
| `Arrow Left` | Move selection 10 mm in negative X direction |
| `Arrow Right` | Move selection 10 mm in positive X direction |
| `Shift+Any arrow` | Movement step set to 1 mm |
| `Esc` | Deselect all |
| `1-9` | Keyboard 1-9: set filament for object/part |
| `Ctrl+0` | Camera view - Default |
| `Ctrl+1` | Camera view - Top |
| `Ctrl+2` | Camera view - Bottom |
| `Ctrl+3` | Camera view - Front |
| `Ctrl+4` | Camera view - Behind |
| `Ctrl+5` | Camera Angle - Left side |
| `Ctrl+6` | Camera Angle - Right side |
| `Ctrl+A` | Select all objects |
| `Ctrl+D` | Delete all |
| `Ctrl+Z` | Undo |
| `Ctrl+Y` | Redo |
| `M` | Gizmo move |
| `R` | Gizmo rotate |
| `S` | Gizmo scale |
| `F` | Gizmo place face on bed |
| `C` | Gizmo cut |
| `B` | Gizmo mesh boolean |
| `H` | Gizmo FDM paint-on fuzzy skin |
| `L` | Gizmo SLA support points |
| `P` | Gizmo FDM paint-on seam |
| `T` | Gizmo text emboss/engrave |
| `U` | Gizmo measure |
| `Y` | Gizmo assemble |
| `E` | Gizmo brim ears |
| `I` | Zoom in |
| `O` | Zoom out |
| `V` | Toggle printable for object/part |
| `Tab` | Switch between Prepare/Preview |

### Gizmo

| Touches | Action |
|---|---|
| `Esc` | Deselect all |
| `Shift+` | Move: press to snap by 1mm |
| `Ctrl+Mouse wheel` | Support/Color Painting: adjust pen radius |
| `Alt+Mouse wheel` | Support/Color Painting: adjust section position |

### Objects List

| Touches | Action |
|---|---|
| `1-9` | Set extruder number for the objects and parts |
| `Del` | Delete objects, parts, modifiers |
| `Esc` | Deselect all |
| `Ctrl+C` | Copy to clipboard |
| `Ctrl+V` | Paste from clipboard |
| `Ctrl+X` | Cut |
| `Ctrl+A` | Select all objects |
| `Ctrl+K` | Clone selected |
| `Ctrl+Z` | Undo |
| `Ctrl+Y` | Redo |
| `Space` | Select the object/part and press space to change the name |
| `Mouse click` | Select the object/part and mouse click to change the name |

### Preview

| Touches | Action |
|---|---|
| `Arrow Up` | Vertical slider - Move active thumb Up |
| `Arrow Down` | Vertical slider - Move active thumb Down |
| `Arrow Left` | Horizontal slider - Move active thumb Left |
| `Arrow Right` | Horizontal slider - Move active thumb Right |
| `L` | On/Off one layer mode of the vertical slider |
| `C` | On/Off G-code window |
| `Tab` | Switch between Prepare/Preview |
| `Shift+Any arrow` | Move slider 5x faster |
| `Shift+Mouse wheel` | Move slider 5x faster |
| `Ctrl+Any arrow` | Move slider 5x faster |
| `Ctrl+Mouse wheel` | Move slider 5x faster |
| `Home` | Horizontal slider - Move to start position |
| `End` | Horizontal slider - Move to last position |
