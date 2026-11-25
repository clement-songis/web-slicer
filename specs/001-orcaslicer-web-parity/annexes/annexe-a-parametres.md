# Annexe A — Registre exhaustif des paramètres (normatif)

Source : `audit/parameters.json` (1105 paramètres, généré depuis
`vendor/OrcaSlicer/src/libslic3r/PrintConfig.cpp`). Chaque ligne est une
exigence : le paramètre DOIT être exposé par l'application (stockage,
API, UI selon son groupe) ou inscrit au registre d'exclusions avec
justification.

Portée par groupe : `fff`/`common` → exposés dans l'UI de réglages ;
`sla` → registre + API (pas d'UI, OrcaSlicer n'expose pas d'onglet SLA) ;
`cli:*` → équivalents des actions serveur (tranchage, transformations) ;
`other:*` → placeholders G-code et états de slicing (moteur de templates).

## Groupe `common` (37 paramètres)

| Clé | Type | Catégorie | Mode | Défaut | Libellé |
|---|---|---|---|---|---|
| `bbl_use_printhost` | coBool |  | advanced | False | Use 3rd-party print host |
| `bed_custom_model` | coString |  | advanced |  | Bed custom model |
| `bed_custom_texture` | coString |  | advanced |  | Bed custom texture |
| `bed_exclude_area` | coPoints |  | advanced | ["0x0"] | Excluded bed area |
| `bed_exclude_volume_mode` | coEnum |  | advanced | shared | Collision volume behavior |
| `bed_exclude_volumes` | coString |  | advanced |  | Toolhead collision volumes |
| `elefant_foot_compensation` | coFloat | Quality | advanced | 0.0 | Elephant foot compensation |
| `elefant_foot_compensation_layers` | coInt | Quality | advanced | 1 | Elephant foot compensation layers |
| `elefant_foot_layers_density` | coPercent | Quality | expert | 100 | Elephant foot layers density |
| `extruder_bed_exclude_volumes` | coStrings |  | advanced | [""] | Extruder collision volumes |
| `extruder_printable_area` | coPointsGroups |  | advanced | [] | Extruder printable area |
| `extruder_printable_height` | coFloats |  | advanced | [0] | Extruder printable height |
| `filament_plugin_config_overrides` | coString |  | advanced |  | Capabilities |
| `flashforge_serial_number` | coString |  | advanced |  | Serial Number |
| `layer_height` | coFloat | Quality | simple | 0.2 | Layer height |
| `parallel_printheads_bed_exclude_areas` | coStrings |  | advanced |  | Parallel printheads bed exclude areas |
| `parallel_printheads_count` | coInt |  | advanced | 1 | Parallel printheads count |
| `preferred_orientation` | coFloat |  | advanced | 0.0 | Preferred orientation |
| `preset_name` | coString |  | simple |  |  |
| `preset_names` | coStrings |  | advanced |  | Printer preset names |
| `print_host` | coString |  | advanced |  | Hostname, IP or URL |
| `print_host_webui` | coString |  | advanced |  | Device UI |
| `print_plugin_config_overrides` | coString |  | advanced |  | Capabilities |
| `printable_area` | coPoints |  | advanced | ["0x0", "200x0", "200x200", "0x200"] | Printable area |
| `printable_height` | coFloat |  | simple | 100.0 | Printable height |
| `printer_agent` | coString |  | advanced |  | Printer Agent |
| `printer_plugin_config_overrides` | coString |  | advanced |  | Capabilities |
| `printer_technology` | coEnum |  | simple | FFF | Printer technology |
| `printhost_apikey` | coString |  | advanced |  | API Key / Password |
| `printhost_authorization_type` | coEnum |  | advanced | key | Authorization Type |
| `printhost_cafile` | coString |  | advanced |  | HTTPS CA File |
| `printhost_password` | coString |  | advanced |  | Password |
| `printhost_port` | coString |  | advanced |  | Printer |
| `printhost_ssl_ignore_revoke` | coBool |  | advanced | False | Ignore HTTPS certificate revocation checks |
| `printhost_user` | coString |  | advanced |  | User |
| `support_parallel_printheads` | coBool |  | advanced | False | Support parallel printheads |
| `use_3mf` | coBool |  | advanced | False | Use 3MF instead of G-code |

## Groupe `fff` (785 paramètres)

| Clé | Type | Catégorie | Mode | Défaut | Libellé |
|---|---|---|---|---|---|
| `accel_to_decel_enable` | coBool | Speed | advanced | True | Enable accel_to_decel |
| `accel_to_decel_factor` | coPercent | Speed | advanced | 50 | accel_to_decel |
| `activate_air_filtration` | coBools |  | simple | [false] | Activate air filtration |
| `activate_air_filtration_during_print` | coBools |  | simple | [true] |  |
| `activate_air_filtration_on_completion` | coBools |  | simple | [true] |  |
| `activate_chamber_temp_control` | coBools |  | simple | [false] | Activate temperature control |
| `adaptive_bed_mesh_margin` | coFloat |  | advanced | 0 | Mesh margin |
| `adaptive_infill_gradient` | coEnum | Strength | advanced | linear | Adaptive gradient |
| `adaptive_infill_interior_density` | coPercent | Strength | advanced | 5 | Interior density |
| `adaptive_infill_mode` | coEnum | Strength | advanced | disabled | Adaptive density (experimental) |
| `adaptive_pressure_advance` | coBools |  | advanced | [false] | Enable adaptive pressure advance (beta) |
| `adaptive_pressure_advance_bridges` | coFloats |  | advanced | [0.0] | Static pressure advance for bridges |
| `adaptive_pressure_advance_model` | coStrings |  | advanced | ["0,0,0\\n0,0,0"] | Adaptive pressure advance measurements (beta) |
| `adaptive_pressure_advance_overhangs` | coBools |  | advanced | [false] | Enable adaptive pressure advance within features (beta) |
| `additional_cooling_fan_speed` | coInts |  | simple | [0] | Fan speed |
| `additional_fan_full_speed_layer` | coInts |  | simple | [0] | Full fan speed at layer |
| `align_infill_direction_to_model` | coBool | Strength | advanced | False | Align directions to model |
| `alternate_extra_wall` | coBool | Strength | advanced | False | Alternate extra wall |
| `auxiliary_fan` | coBool |  | advanced | False | Auxiliary part cooling fan |
| `bbl_calib_mark_logo` | coBool |  | advanced | True | Show auto-calibration marks |
| `bed_mesh_max` | coPoint |  | advanced | [99999, 99999] | Bed mesh max |
| `bed_mesh_min` | coPoint |  | advanced | [-99999, -99999] | Bed mesh min |
| `bed_mesh_probe_distance` | coPoint |  | advanced | [50, 50] | Probe point distance |
| `bed_temperature_formula` | coEnum |  | advanced | by_highest_temp | Bed temperature type |
| `before_layer_change_gcode` | coString |  | advanced |  | Before layer change G-code |
| `belt_frame_tilt_angle` | coFloat | Printable space | expert | 45.0 | Machine-frame tilt angle |
| `belt_frame_tilt_decouple` | coBool | Printable space | expert | False | Decouple machine-frame tilt |
| `belt_printer` | coBool | Printable space | advanced | False | Enable belt printing |
| `belt_printer_infinite_y` | coBool | Printable space | advanced | True | Infinite Y axis |
| `belt_purge_tower_object` | coBool | Flush options | develop | False | Belt purge tower object |
| `belt_purge_tower_width` | coFloat | Printable space | advanced | 35.0 | Belt purge tower width |
| `belt_slice_rotation` | coEnum | Printable space | develop | x | Belt tilt axis |
| `belt_slice_rotation_angle` | coFloat | Printable space | advanced | 45.0 | Belt tilt angle |
| `belt_support_floor_offset` | coFloat | Printable space | advanced | 0 | Support Floor Z offset |
| `best_object_pos` | coPoint |  | advanced | [0.5, 0.5] | Best object position |
| `bottom_color_penetration_layers` | coInt | Strength | advanced | 0 | Bottom paint penetration layers |
| `bottom_layer_direction` | coFloat | Strength | simple | -1 | Bottom layer direction |
| `bottom_shell_layers` | coInt | Strength | simple | 3 | Bottom shell layers |
| `bottom_shell_thickness` | coFloat | Strength | simple | 0.0 | Bottom shell thickness |
| `bottom_solid_infill_flow_ratio` | coFloat | Advanced | advanced | 1 | Bottom surface flow ratio |
| `bottom_surface_density` | coPercent | Strength | simple | 100 | Bottom surface density |
| `bottom_surface_filament_id` | coInt | Extruders | advanced | 0 | Bottom surface |
| `bottom_surface_fill_order` | coEnum | Strength | advanced | default | Bottom surface fill order |
| `bottom_surface_pattern` | coEnum | Strength | simple | monotonic | Bottom surface pattern |
| `bridge_acceleration` | coFloatsOrPercents | Speed | advanced | {FloatOrPercent(50,true)} | Bridge |
| `bridge_angle` | coFloat | Strength | advanced | 0.0 | External bridge infill direction |
| `bridge_density` | coPercent | Strength | advanced | 100 | External bridge density |
| `bridge_flow` | coFloat | Quality | advanced | 1 | Bridge flow ratio |
| `bridge_line_width` | coFloatOrPercent | Quality | advanced | {"value": 100.0, "percent": true} | Bridge |
| `bridge_no_support` | coBool | Support | advanced | False | Don't support bridges |
| `bridge_speed` | coFloats | Speed | advanced | [25] | External |
| `bridge_volumetric_flow` | coFloatsOrPercents | Speed | advanced | {FloatOrPercent(25, true)} | External |
| `brim_ears` | coBool | Support | advanced | False | Brim ears |
| `brim_ears_detection_length` | coFloat | Support | advanced | 1 | Brim ear detection radius |
| `brim_ears_max_angle` | coFloat | Support | advanced | 125 | Brim ear max angle |
| `brim_ears_outer_only` | coBool | Support | advanced | False | Brim ears outer only |
| `brim_filament_id` | coInt | Extruders | advanced | 0 | Brim |
| `brim_flow_ratio` | coFloat | Support | advanced | 1 | Brim flow ratio |
| `brim_object_gap` | coFloat | Support | advanced | 0.0 | Brim-object gap |
| `brim_type` | coEnum | Support | simple | auto_brim | Brim type |
| `brim_use_efc_outline` | coBool | Support | advanced | False | Brim follows compensated outline |
| `brim_width` | coFloat | Support | simple | 0.0 | Brim width |
| `build_plate_tilt_x` | coFloat | Support | expert | 0.0 | Build plate tilt X |
| `build_plate_tilt_y` | coFloat | Support | expert | 0.0 | Build plate tilt Y |
| `calib_flowrate_topinfill_special_order` | coBool |  | develop | False |  |
| `center_of_surface_pattern` | coEnum | Strength | expert | each_surface | Center surface pattern on |
| `chamber_minimal_temperature` | coInts |  | simple | [0] | Minimal |
| `chamber_temperature` | coInts |  | simple | [0] | Chamber temperature |
| `change_extrusion_role_gcode` | coString |  | advanced |  | Change extrusion role G-code |
| `change_filament_gcode` | coString |  | advanced |  | Change filament G-code |
| `close_additional_fan_first_x_layers` | coInts |  | simple | [1] | For the first |
| `close_fan_the_first_x_layers` | coInts |  | simple | [1] | No cooling for the first |
| `combine_brims` | coBool | Support | advanced | False | Combine brims |
| `compatible_machine_expression_group` | coStrings |  | simple |  |  |
| `compatible_printers` | coStrings |  | advanced |  | Select printers |
| `compatible_printers_condition` | coString |  | advanced |  | Condition |
| `compatible_prints` | coStrings |  | advanced |  | Select profiles |
| `compatible_prints_condition` | coString |  | advanced |  | Condition |
| `compatible_process_expression_group` | coStrings |  | simple |  |  |
| `complete_print_exhaust_fan_speed` | coInts |  | simple | [80] | Fan speed |
| `cool_plate_temp` | coInts |  | simple | [35] | Other layers |
| `cool_plate_temp_initial_layer` | coInts |  | simple | [35] | First layer |
| `cooling_filter_enabled` | coBool |  | advanced | False | Use cooling filter |
| `cooling_tube_length` | coFloat |  | advanced | 5.0 | Cooling tube length |
| `cooling_tube_retraction` | coFloat |  | advanced | 91.5 | Cooling tube position |
| `counterbore_hole_bridging` | coEnum | Quality | advanced | none | Bridge counterbore holes |
| `curr_bed_type` | coEnum |  | simple | Cool Plate | Bed type |
| `default_acceleration` | coFloats | Speed | advanced | [500.0] | Normal printing |
| `default_bed_type` | coString |  | advanced |  | Default bed type |
| `default_filament_colour` | coStrings |  | advanced | [""] | Default color |
| `default_filament_profile` | coStrings |  | simple |  | Default filament profile |
| `default_jerk` | coFloats | Speed | advanced | [0] | Default |
| `default_junction_deviation` | coFloats | Speed | advanced | [0.0] | Junction Deviation |
| `default_nozzle_volume_type` | coEnums |  | develop | ["Standard"] | Default Nozzle Volume Type. |
| `default_print_profile` | coString |  | simple |  | Default process profile |
| `deretract_speed_extruder_change` | coFloats |  | develop | [0.0] | Deretraction speed (extruder change) |
| `deretraction_speed` | coFloats |  | advanced | [0.0] | Deretraction speed |
| `detect_narrow_internal_solid_infill` | coBool | Strength | advanced | True | Detect narrow internal solid infills |
| `detect_overhang_wall` | coBool | Quality | advanced | True | Detect overhang walls |
| `detect_thin_wall` | coBool | Strength | advanced | False | Detect thin walls |
| `different_settings_to_system` | coStrings |  | simple |  |  |
| `disable_m73` | coBool |  | advanced | False | Disable set remaining print time |
| `dont_filter_internal_bridges` | coEnum | Quality | advanced | disabled | Filter out small internal bridges |
| `dont_slow_down_outer_wall` | coBools |  | simple | [false] | Don't slow down outer walls |
| `draft_shield` | coEnum |  | advanced | disabled | Draft shield |
| `during_print_exhaust_fan_speed` | coInts |  | simple | [60] | Fan speed |
| `emit_machine_limits_to_gcode` | coBool | Machine limits | advanced | True | Emit limits to G-code |
| `enable_arc_fitting` | coEnum |  | advanced | disabled | Arc fitting |
| `enable_belt_purge_tower` | coBool | Multimaterial | advanced | False | Enable belt purge tower |
| `enable_extra_bridge_layer` | coEnum | Quality | advanced | disabled | Extra bridge layers (beta) |
| `enable_filament_dynamic_map` | coBool |  | develop | False | Enable filament dynamic map |
| `enable_filament_ramming` | coBool |  | advanced | True | Enable filament ramming |
| `enable_long_retraction_when_cut` | coInt |  | develop | 0 |  |
| `enable_mixed_color_sublayer` | coBool | Quality | simple | False | Mixed color sublayer |
| `enable_overhang_bridge_fan` | coBools |  | simple | [true] | Force cooling for overhangs and bridges |
| `enable_overhang_speed` | coBools | Speed | advanced | [true] | Slow down for overhangs |
| `enable_power_loss_recovery` | coEnum |  | advanced | printer_configuration | Power Loss Recovery |
| `enable_pre_heating` | coBool |  | simple | False |  |
| `enable_pressure_advance` | coBools |  | simple | [false] | Enable pressure advance |
| `enable_prime_tower` | coBool |  | simple | False | Enable |
| `enable_support` | coBool | Support | simple | False | Enable support |
| `enable_tower_interface_cooldown_during_tower` | coBool |  | advanced | False | Cool down from interface boost during prime tower |
| `enable_tower_interface_features` | coBool |  | advanced | False | Enable tower interface features |
| `enable_volumetric_speeds` | coBool | Speed | advanced | False | Volumetric speeds |
| `enable_wrapping_detection` | coBool |  | advanced | False | Enable clumping detection |
| `enforce_support_layers` | coInt | Support | develop | 0 |  |
| `eng_plate_temp` | coInts |  | simple | [45] | Other layers |
| `eng_plate_temp_initial_layer` | coInts |  | simple | [45] | First layer |
| `ensure_vertical_shell_thickness` | coEnum | Strength | advanced | ensure_all | Ensure vertical shell thickness |
| `exclude_object` | coBool |  | advanced | False | Exclude objects |
| `exclude_object_tolerance` | coFloat |  | expert | 0.1 | Outline tolerance |
| `extra_brim_width` | coFloat | Support | advanced | 0.0 | Extra brim width |
| `extra_loading_move` | coFloat |  | advanced | -2.0 | Extra loading distance |
| `extra_perimeters_on_overhangs` | coBool | Quality | advanced | False | Extra perimeters on overhangs |
| `extra_solid_infills` | coString | Strength | advanced |  | Insert solid layers |
| `extruder` | coInt | Extruders | advanced |  | Extruder |
| `extruder_ams_count` | coStrings |  | simple | [] | Extruder AMS count |
| `extruder_clearance_dist_to_rod` | coFloat |  | advanced | 40 | Distance to rod |
| `extruder_clearance_height_to_lid` | coFloat |  | advanced | 120 | Height to lid |
| `extruder_clearance_height_to_rod` | coFloat |  | advanced | 40 | Height to rod |
| `extruder_clearance_radius` | coFloat |  | advanced | 40 | Radius |
| `extruder_colour` | coStrings |  | advanced | [""] | Extruder Color |
| `extruder_max_nozzle_count` | coInts |  | develop | [1] |  |
| `extruder_nozzle_count` | coInts |  | develop | [1] | extruder nozzle count |
| `extruder_nozzle_stats` | coStrings |  | simple | [] | Extruder nozzle stats |
| `extruder_nozzle_volume_type` | coEnums |  | develop | ["Standard"] | Extruder nozzle volume type |
| `extruder_offset` | coPoints |  | advanced | ["0x0"] | Extruder offset |
| `extruder_type` | coEnums |  | advanced | ["Direct Drive"] | Type |
| `extruder_variant_list` | coStrings |  | simple | ["Direct Drive Standard"] | Extruder variant list |
| `extrusion_rate_smoothing_external_perimeter_only` | coBool |  | advanced | False | Apply only on external features |
| `fan_cooling_layer_time` | coFloats |  | simple | [60.0] | Layer time |
| `fan_direction` | coEnum |  | develop | undefine | Fan direction |
| `fan_kickstart` | coFloat |  | advanced | 0 | Fan kick-start time |
| `fan_max_speed` | coFloats |  | simple | [100] | Fan speed |
| `fan_min_speed` | coFloats |  | simple | [20] | Fan speed |
| `fan_speedup_overhangs` | coBool |  | advanced | True | Only overhangs |
| `fan_speedup_time` | coFloat |  | advanced | 0 |  |
| `farthest_point_timelapse` | coBool |  | simple | False | Farthest point timelapse |
| `filament_adaptive_volumetric_speed` | coBools |  | develop | [false] | Adaptive volumetric speed |
| `filament_adhesiveness_category` | coInts |  | develop | [0] | Adhesiveness Category |
| `filament_change_extrusion_role_gcode` | coStrings |  | advanced | [""] | Change extrusion role G-code (filament) |
| `filament_change_length` | coFloats |  | advanced | [10] | Filament ramming length |
| `filament_change_length_nc` | coFloats |  | advanced | [10] | Hotend change |
| `filament_colour` | coStrings |  | advanced | ["#F2754E"] | Color |
| `filament_colour_type` | coStrings |  | simple | ["1"] |  |
| `filament_cooling_before_tower` | coFloats |  | develop | [10.0] | Wipe tower cooling |
| `filament_cooling_final_speed` | coFloats |  | advanced | [3.4] | Speed of the last cooling move |
| `filament_cooling_initial_speed` | coFloats |  | advanced | [2.2] | Speed of the first cooling move |
| `filament_cooling_moves` | coInts |  | advanced | [4] | Number of cooling moves |
| `filament_cost` | coFloats |  | advanced | [0.0] | Price |
| `filament_density` | coFloats |  | advanced | [0.0] | Density |
| `filament_deretraction_speed` | coFloatsNullable |  | advanced |  | Deretraction speed |
| `filament_dev_ams_drying_ams_limitations` | coStrings |  | simple | [""] |  |
| `filament_dev_ams_drying_heat_distortion_temperature` | coFloats |  | simple | [0] |  |
| `filament_dev_ams_drying_temperature` | coFloats |  | simple | [0] |  |
| `filament_dev_ams_drying_time` | coFloats |  | simple | [0] |  |
| `filament_dev_chamber_drying_bed_temperature` | coFloats |  | simple | [0] |  |
| `filament_dev_chamber_drying_time` | coFloats |  | simple | [0] |  |
| `filament_dev_drying_cooling_temperature` | coFloats |  | simple | [0] |  |
| `filament_dev_drying_softening_temperature` | coFloats |  | simple | [0] |  |
| `filament_diameter` | coFloats |  | simple | [1.75] | Diameter |
| `filament_end_gcode` | coStrings |  | advanced | [" "] | End G-code |
| `filament_extruder_compatibility` | coInts |  | develop | [0] | Filament-extruder compatibility |
| `filament_extruder_variant` | coStrings |  | simple | ["Direct Drive Standard"] | Filament's extruder variant |
| `filament_flow_ratio` | coFloats |  | advanced | [1.0] | Flow ratio |
| `filament_flush_temp` | coInts |  | advanced | [0] | Flush temperature |
| `filament_flush_temp_fast` | coInts |  | advanced | [0] | Flush temperature |
| `filament_flush_volumetric_speed` | coFloats |  | advanced | [0] | Flush volumetric speed |
| `filament_ids` | coStrings |  | simple |  |  |
| `filament_ironing_flow` | coPercents | Quality | advanced |  | Ironing flow |
| `filament_ironing_inset` | coFloats | Quality | advanced |  | Ironing inset |
| `filament_ironing_spacing` | coFloats | Quality | advanced |  | Ironing line spacing |
| `filament_ironing_speed` | coFloats | Speed | advanced |  | Ironing speed |
| `filament_is_mixed` | coBools |  | develop | [false] | Is mixed filament |
| `filament_is_support` | coBools |  | advanced | [false] | Support material |
| `filament_loading_speed` | coFloats |  | advanced | [28.0] | Loading speed |
| `filament_loading_speed_start` | coFloats |  | advanced | [3.0] | Loading speed at the start |
| `filament_long_retractions_when_cut` | coBoolsNullable |  | develop |  | Long retraction when cut (beta) |
| `filament_map` | coInts |  | develop | [1] | Filament map to extruder |
| `filament_map_2` | coInts |  | develop | [1] | Filament map plus for multi nozzle |
| `filament_map_mode` | coEnum |  | advanced | Auto For Flush | filament mapping mode |
| `filament_max_external_volumetric_speed` | coFloats |  | advanced | [0.0] | Max external volumetric speed |
| `filament_max_temperature_drop_when_ec` | coFloats |  | develop | [0] |  |
| `filament_max_volumetric_speed` | coFloats |  | advanced | [2.0] | Max volumetric speed |
| `filament_minimal_purge_on_wipe_tower` | coFloats |  | advanced | [15.0] | Minimal purge on wipe tower |
| `filament_mixed_components` | coStrings |  | develop | [""] | Mixed filament components |
| `filament_mixed_gradient` | coBools |  | develop | [false] | Mixed filament gradient |
| `filament_mixed_gradient_curve` | coStrings |  | develop | [""] | Mixed filament gradient curve |
| `filament_mixed_gradient_per_part` | coBools |  | develop | [false] | Mixed filament per-part gradient |
| `filament_mixed_gradient_range` | coStrings |  | develop | [""] | Mixed filament gradient range |
| `filament_mixed_sublayer_ratios` | coStrings |  | develop | [""] | Mixed filament sublayer ratios |
| `filament_multi_colour` | coStrings |  | simple | [""] |  |
| `filament_multitool_ramming` | coBools |  | advanced | [false] | Enable ramming for multi-tool setups |
| `filament_multitool_ramming_flow` | coFloats |  | advanced | [10.0] | Multi-tool ramming flow |
| `filament_multitool_ramming_volume` | coFloats |  | advanced | [10.0] | Multi-tool ramming volume |
| `filament_notes` | coStrings |  | advanced | [""] | Filament notes |
| `filament_nozzle_map` | coInts |  | develop | [1] |  |
| `filament_pre_cooling_temperature` | coInts |  | advanced | [0] | Extruder change |
| `filament_pre_cooling_temperature_nc` | coInts |  | advanced | [0] | Hotend change |
| `filament_preheat_temperature_delta` | coFloats |  | develop | [0] | Preheat temperature delta |
| `filament_prime_volume` | coFloats |  | simple | [45.0] | Filament change |
| `filament_prime_volume_nc` | coFloats |  | simple | [60.0] | Hotend change |
| `filament_printable` | coInts |  | develop | [3] | Filament printable |
| `filament_ramming_parameters` | coStrings |  | advanced | ["120 100 6.6 6.8 7.2 7.6 7.9 8.2 8.7 9.4 9.9 10.0\| 0.05 6.6 0.45 6.8 0.95 7.8 1.45 8.3 1.95 9.7 2.45 10 2.95 7.6 3.45 7.6 3.95 7.6 4.45 7.6 4.95 7.6"] | Ramming parameters |
| `filament_ramming_travel_time` | coFloats |  | advanced | [0] | Extruder change |
| `filament_ramming_travel_time_nc` | coFloats |  | advanced | [0] | Hotend change |
| `filament_ramming_volumetric_speed` | coFloats |  | advanced | [-1] | Extruder change |
| `filament_ramming_volumetric_speed_nc` | coFloats |  | advanced | [-1] | Hotend change |
| `filament_retract_after_wipe` | coPercentsNullable |  | expert |  | Retract amount after wipe |
| `filament_retract_before_wipe` | coPercentsNullable |  | advanced |  | Retract amount before wipe |
| `filament_retract_length_nc` | coFloats |  | develop | [10.0] | length when change hotend |
| `filament_retract_length_toolchange` | coFloatsNullable |  | advanced |  | Retraction Length (Toolchange) |
| `filament_retract_lift_above` | coFloatsNullable |  | advanced |  | Only lift Z above |
| `filament_retract_lift_below` | coFloatsNullable |  | advanced |  | Only lift Z below |
| `filament_retract_lift_enforce` | coEnumsNullable |  | advanced |  | On surfaces |
| `filament_retract_restart_extra` | coFloatsNullable |  | advanced |  | Extra length on restart |
| `filament_retract_restart_extra_toolchange` | coFloatsNullable |  | advanced |  | Extra length on restart (Toolchange) |
| `filament_retract_when_changing_layer` | coBoolsNullable |  | advanced |  | Retract on layer change |
| `filament_retraction_distances_when_cut` | coFloatsNullable |  | develop |  | Retraction distance when cut |
| `filament_retraction_length` | coFloatsNullable |  | simple |  | Length |
| `filament_retraction_minimum_travel` | coFloatsNullable |  | advanced |  | Travel distance threshold |
| `filament_retraction_speed` | coFloatsNullable |  | advanced |  | Retraction speed |
| `filament_self_index` | coInts |  | simple | [1] | Filament self index |
| `filament_settings_id` | coStrings |  | simple | [""] |  |
| `filament_shrink` | coPercents |  | advanced | [100] | Shrinkage (XY) |
| `filament_shrinkage_compensation_z` | coPercents |  | advanced | [100] | Shrinkage (Z) |
| `filament_soluble` | coBools |  | advanced | [false] | Soluble material |
| `filament_stamping_distance` | coFloats |  | advanced | [0.0] | Stamping distance measured from the center of the cooling tube |
| `filament_stamping_loading_speed` | coFloats |  | advanced | [0.0] | Stamping loading speed |
| `filament_start_gcode` | coStrings |  | advanced | [" "] | Start G-code |
| `filament_toolchange_delay` | coFloats |  | advanced | [0.0] | Delay after unloading |
| `filament_tower_interface_pre_extrusion_dist` | coFloats |  | advanced | [10.0] | Interface layer pre-extrusion distance |
| `filament_tower_interface_pre_extrusion_length` | coFloats |  | advanced | [0.0] | Interface layer pre-extrusion length |
| `filament_tower_interface_print_temp` | coInts |  | advanced | [-1] | Interface layer print temperature |
| `filament_tower_interface_purge_volume` | coFloats |  | advanced | [20.0] | Interface layer purge length |
| `filament_tower_ironing_area` | coFloats |  | advanced | [4.0] | Tower ironing area |
| `filament_type` | coStrings |  | simple | ["PLA"] | Type |
| `filament_unloading_speed` | coFloats |  | advanced | [90.0] | Unloading speed |
| `filament_unloading_speed_start` | coFloats |  | advanced | [100.0] | Unloading speed at the start |
| `filament_vendor` | coStrings |  | advanced | ["(Undefined)"] | Vendor |
| `filament_volume_map` | coInts |  | develop | {(int)(NozzleVolumeType::nvtStandard)} |  |
| `filament_wipe` | coBoolsNullable |  | advanced |  | Wipe while retracting |
| `filament_wipe_distance` | coFloatsNullable |  | advanced |  | Wipe distance |
| `filament_z_hop` | coFloatsNullable |  | simple |  | Z-hop height |
| `filament_z_hop_types` | coEnumsNullable |  | advanced |  | Z-hop type |
| `file_start_gcode` | coString |  | advanced |  | File header G-code |
| `filename_format` | coString |  | advanced | {input_filename_base}_{filament_type[initial_tool]}_{print_time}.gcode | Filename format |
| `fill_multiline` | coInt | Strength | simple | 1 | Fill Multiline |
| `filter_out_gap_fill` | coFloat | Layers and Perimeters | advanced | 0 | Filter out tiny gaps |
| `first_layer_flow_ratio` | coFloat | Advanced | advanced | 1 | First layer flow ratio |
| `first_layer_print_sequence` | coInts |  | simple | [0] | First layer print sequence |
| `first_layer_sequence_choice` | coEnum | Quality | simple | Auto | First layer filament sequence |
| `first_x_layer_fan_speed` | coFloats |  | simple | [0] | Fan speed |
| `flush_into_infill` | coBool | Flush options | simple | False | Flush into objects' infill |
| `flush_into_objects` | coBool | Flush options | simple | False | Flush into this object |
| `flush_into_support` | coBool | Flush options | simple | True | Flush into objects' support |
| `flush_multiplier` | coFloats |  | simple | [0.3] | Flush multiplier |
| `flush_multiplier_fast` | coFloats |  | simple | [1.2] | Flush multiplier (Fast mode) |
| `flush_volumes_matrix` | coFloats |  | simple | [0.0, 280.0, 280.0, 280.0, 280.0, 0.0, 280.0, 280.0, 280.0, 280.0, 0.0, 280.0, 280.0, 280.0, 280.0, 0.0] | Purging volumes |
| `flush_volumes_vector` | coFloats |  | simple | [140.0, 140.0, 140.0, 140.0, 140.0, 140.0, 140.0, 140.0] | Purging volumes - load/unload volumes |
| `full_fan_speed_layer` | coInts |  | advanced | [0] | Full fan speed at layer |
| `fuzzy_skin` | coEnum | Others | simple | FuzzySkinType::Disabled_fuzzy | Fuzzy skin |
| `fuzzy_skin_first_layer` | coBool | Others | simple | 0 | Apply fuzzy skin to first layer |
| `fuzzy_skin_layers_between_ripple_offset` | coInt | Others | advanced | 1 | Layers between ripple offset |
| `fuzzy_skin_mode` | coEnum | Others | simple | displacement | Fuzzy skin generator mode |
| `fuzzy_skin_noise_type` | coEnum | Others | simple | classic | Fuzzy skin noise type |
| `fuzzy_skin_octaves` | coInt | Others | advanced | 4 | Fuzzy Skin Noise Octaves |
| `fuzzy_skin_persistence` | coFloat | Others | advanced | 0.5 | Fuzzy skin noise persistence |
| `fuzzy_skin_point_distance` | coFloat | Others | simple | 0.3f | Fuzzy skin point distance |
| `fuzzy_skin_ripple_offset` | coPercent | Others | advanced | 50 | Ripple offset |
| `fuzzy_skin_ripples_per_layer` | coInt | Others | advanced | 15 | Number of ripples per layer |
| `fuzzy_skin_scale` | coFloat | Others | advanced | 1.0 | Fuzzy skin feature size |
| `fuzzy_skin_thickness` | coFloat | Others | simple | 0.2 | Fuzzy skin thickness |
| `gap_fill_flow_ratio` | coFloat | Advanced | advanced | 1 | Gap fill flow ratio |
| `gap_fill_target` | coEnum | Strength | advanced | nowhere | Apply gap fill |
| `gap_infill_speed` | coFloats | Speed | advanced | [30] | Gap infill |
| `gap_infill_volumetric_flow` | coFloatsOrPercents | Speed | advanced | {FloatOrPercent(50, true)} | Gap infill |
| `gcode_add_line_number` | coBool |  | develop | 0 | Add line number |
| `gcode_comments` | coBool |  | advanced | 0 | Verbose G-code |
| `gcode_flavor` | coEnum |  | advanced | marlin | G-code flavor |
| `gcode_label_objects` | coBool |  | advanced | 1 | Label objects |
| `gcode_remap_x` | coEnum | Printable space | develop | pos_x | G-code remap X |
| `gcode_remap_y` | coEnum | Printable space | develop | pos_y | G-code remap Y |
| `gcode_remap_z` | coEnum | Printable space | develop | pos_z | G-code remap Z |
| `gcode_skip_config_block` | coBool |  | advanced | False | Skip G-code config block |
| `grab_length` | coFloats |  | develop | [0] | Grab length |
| `group_algo_with_time` | coBool |  | develop | False |  |
| `gyroid_optimized` | coBool | Strength | simple | False | Z-buckling bias optimization (experimental) |
| `handle_hotend_as_extruder` | coBool |  | develop | False |  |
| `has_filament_switcher` | coBool |  | develop | False | Has filament switcher |
| `has_scarf_joint_seam` | coBool | Machine limits | advanced | False |  |
| `head_wrap_detect_zone` | coPoints |  | develop | [] | Head wrap detect zone |
| `high_current_on_filament_swap` | coBool |  | advanced | 0 | High extruder current on filament swap |
| `high_precision_gcode` | coBool |  | expert | False | High precision G-code (experimental) |
| `hole_to_polyhole` | coBool | Quality | advanced | False | Convert holes to polyholes |
| `hole_to_polyhole_max_edges` | coInt | Quality | expert | 50 | Maximum Polyhole edge count |
| `hole_to_polyhole_threshold` | coFloatOrPercent | Quality | advanced | {"value": 0.01, "percent": false} | Polyhole detection margin |
| `hole_to_polyhole_twisted` | coBool | Quality | advanced | True | Polyhole twist |
| `host_type` | coEnum |  | advanced | octoprint | Host Type |
| `hot_plate_temp` | coInts |  | simple | [45] | Other layers |
| `hot_plate_temp_initial_layer` | coInts |  | simple | [45] | First layer |
| `hotend_cooling_rate` | coFloats |  | simple | [2] |  |
| `hotend_heating_rate` | coFloats |  | simple | [2] |  |
| `idle_temperature` | coInts |  | simple | [0] | Idle temperature |
| `imex_carriage_margin` | coFloat |  | advanced | 0.0 | Safety Margin |
| `imex_firmware_managed_zones` | coBool |  | advanced | False | Firmware-managed zones |
| `imex_gantry_count` | coInt |  | advanced | 1 | Gantry Count |
| `imex_head_filament_map` | coString |  | develop |  | IDEX/IQEX head filament map |
| `imex_mode_active_tools` | coStrings |  | advanced |  | IDEX/IQEX Mode Active Tools |
| `imex_mode_gcodes` | coStrings |  | advanced |  | IDEX/IQEX Mode G-codes |
| `imex_mode_names` | coStrings |  | advanced |  | IDEX/IQEX Mode Names |
| `imex_nozzle_clearance_x` | coFloat |  | advanced | 30.0 | Nozzle Clearance X |
| `imex_nozzle_clearance_y` | coFloat |  | advanced | 30.0 | Nozzle Clearance Y |
| `imex_parallel_mode` | coString |  | advanced | primary | IDEX/IQEX Print Mode |
| `imex_tool_layout` | coEnum |  | advanced | front-left | Tool 0 Position |
| `imex_tools_per_gantry` | coInt |  | advanced | 2 | Tools per Gantry |
| `imex_viz_theme` | coEnum |  | advanced | standard | Visualization Theme |
| `independent_support_layer_height` | coBool | Support | advanced | True | Independent support layer height |
| `infill_anchor` | coFloatOrPercent | Strength | advanced | {"value": 400.0, "percent": true} | Sparse infill anchor length |
| `infill_anchor_max` | coFloatOrPercent | Strength | advanced | {"value": 20.0, "percent": false} | Maximum length of the infill anchor |
| `infill_combination` | coBool | Strength | advanced | False | Infill combination |
| `infill_combination_max_layer_height` | coFloatOrPercent | Strength | advanced | {"value": 100.0, "percent": true} | Infill combination - Max layer height |
| `infill_complete_top` | coBool | Strength | advanced | False | Fill pattern tops |
| `infill_direction` | coFloat | Strength | advanced | 45 | Sparse infill direction |
| `infill_jerk` | coFloats | Speed | advanced | [9] | Infill |
| `infill_lock_depth` | coFloat | Strength | advanced | 1.0 | Infill lock depth |
| `infill_overhang_angle` | coFloat | Strength | advanced | 60 | Infill overhang angle |
| `infill_shift_step` | coFloat | Strength | advanced | 0.4 | Infill shift step |
| `infill_wall_overlap` | coPercent | Strength | advanced | 15 | Infill/wall overlap |
| `inherits` | coString |  | simple |  | Inherits profile |
| `inherits_group` | coStrings |  | simple |  |  |
| `initial_layer_acceleration` | coFloats | Speed | advanced | [300] | First layer |
| `initial_layer_fan_speed` | coInts |  | advanced | [-1] | First layer fan speed |
| `initial_layer_infill_speed` | coFloats |  | advanced | [60.0] | First layer infill |
| `initial_layer_infill_volumetric_flow` | coFloatsOrPercents | Speed | advanced | {FloatOrPercent(50, true)} | First layer infill |
| `initial_layer_jerk` | coFloats | Speed | advanced | [9] | First layer |
| `initial_layer_line_width` | coFloatOrPercent | Quality | advanced | {"value": 0.0, "percent": false} | First layer |
| `initial_layer_min_bead_width` | coPercent | Quality | advanced | 85 | First layer minimum wall width |
| `initial_layer_print_height` | coFloat | Quality | simple | 0.2 | First layer height |
| `initial_layer_speed` | coFloats |  | advanced | [30] | First layer |
| `initial_layer_travel_acceleration` | coFloatsOrPercents |  | advanced | {FloatOrPercent(100, true)} | First layer travel |
| `initial_layer_travel_jerk` | coFloatsOrPercents |  | advanced | {FloatOrPercent(100, true)} | First layer travel |
| `initial_layer_travel_speed` | coFloatsOrPercents | Speed | advanced | {FloatOrPercent(100, true)} | First layer travel speed |
| `initial_layer_volumetric_flow` | coFloatsOrPercents | Speed | advanced | {FloatOrPercent(25, true)} | First layer |
| `inner_wall_acceleration` | coFloats | Speed | advanced | [10000] | Inner wall |
| `inner_wall_filament_id` | coInt | Extruders | advanced | 0 | Inner walls |
| `inner_wall_flow_ratio` | coFloat | Advanced | advanced | 1 | Inner wall flow ratio |
| `inner_wall_jerk` | coFloats | Speed | advanced | [9] | Inner wall |
| `inner_wall_line_width` | coFloatOrPercent | Quality | advanced | {"value": 0.0, "percent": false} | Inner wall |
| `inner_wall_speed` | coFloats | Speed | advanced | [60] | Inner wall |
| `inner_wall_volumetric_flow` | coFloatsOrPercents | Speed | advanced | {FloatOrPercent(100, true)} | Inner wall |
| `input_shaping_damp_x` | coFloat |  | expert | 0.1 | XAxis |
| `input_shaping_damp_y` | coFloat |  | expert | 0.1 | YAxis |
| `input_shaping_emit` | coBool |  | expert | False | Emit input shaping |
| `input_shaping_freq_x` | coFloat |  | expert | 0 | XAxis |
| `input_shaping_freq_y` | coFloat |  | expert | 0 | YAxis |
| `input_shaping_type` | coEnum |  | expert | Default | Input shaper type |
| `interface_shells` | coBool | Quality | advanced | False | Interface shells |
| `interlocking_beam` | coBool | Advanced | advanced | False | Use beam interlocking |
| `interlocking_beam_layer_count` | coInt | Advanced | advanced | 2 | Interlocking beam layers |
| `interlocking_beam_width` | coFloat | Advanced | advanced | 0.8 | Interlocking beam width |
| `interlocking_boundary_avoidance` | coInt | Advanced | advanced | 2 | Interlocking boundary avoidance |
| `interlocking_depth` | coInt | Advanced | advanced | 2 | Interlocking depth |
| `interlocking_orientation` | coFloat | Advanced | advanced | 22.5 | Interlocking direction |
| `internal_bridge_angle` | coFloat | Strength | advanced | 0.0 | Internal bridge infill direction |
| `internal_bridge_density` | coPercent | Strength | advanced | 100 | Internal bridge density |
| `internal_bridge_fan_speed` | coInts |  | advanced | [-1] | Internal bridges fan speed |
| `internal_bridge_flow` | coFloat | Quality | advanced | 1 | Internal bridge flow ratio |
| `internal_bridge_speed` | coFloatsOrPercents | Speed | advanced | {FloatOrPercent(150, true)} | Internal |
| `internal_bridge_volumetric_flow` | coFloatsOrPercents | Speed | advanced | {FloatOrPercent(40, true)} | Internal |
| `internal_solid_filament_id` | coInt | Extruders | advanced | 0 | Internal solid infill |
| `internal_solid_infill_acceleration` | coFloatsOrPercents | Speed | advanced | {FloatOrPercent(100, true)} | Internal solid infill |
| `internal_solid_infill_flow_ratio` | coFloat | Advanced | advanced | 1 | Internal solid infill flow ratio |
| `internal_solid_infill_line_width` | coFloatOrPercent | Quality | advanced | {"value": 0.0, "percent": false} | Internal solid infill |
| `internal_solid_infill_pattern` | coEnum | Strength | simple | monotonic | Internal solid infill pattern |
| `internal_solid_infill_speed` | coFloats | Speed | advanced | [100] | Internal solid infill |
| `internal_solid_infill_volumetric_flow` | coFloatsOrPercents | Speed | advanced | {FloatOrPercent(100, true)} | Internal solid infill |
| `ironing_angle` | coFloat | Quality | advanced | 0 | Ironing angle offset |
| `ironing_angle_fixed` | coBool | Quality | advanced | False | Fixed ironing angle |
| `ironing_expansion` | coFloat | Quality | expert | 0 | Ironing expansion |
| `ironing_fan_speed` | coInts |  | advanced | [-1] | Ironing fan speed |
| `ironing_flow` | coPercent | Quality | advanced | 10 | Ironing flow |
| `ironing_inset` | coFloat | Quality | advanced | 0 | Ironing inset |
| `ironing_pattern` | coEnum | Quality | advanced | rectilinear | Ironing Pattern |
| `ironing_spacing` | coFloat | Quality | advanced | 0.1 | Ironing line spacing |
| `ironing_speed` | coFloat | Quality | advanced | 20 | Ironing speed |
| `ironing_type` | coEnum | Quality | advanced | no ironing | Ironing type |
| `is_imex` | coBool |  | advanced | False | IDEX/IQEX Printer |
| `is_infill_first` | coBool | Quality | advanced | False | Print infill first |
| `lateral_lattice_angle_1` | coFloat | Strength | advanced | -45 | Lateral lattice angle 1 |
| `lateral_lattice_angle_2` | coFloat | Strength | advanced | 45 | Lateral lattice angle 2 |
| `layer_change_gcode` | coString |  | advanced |  | Layer change G-code |
| `leading_brim_length` | coFloat | Support | advanced | 0.0 | Leading brim length |
| `lightning_overhang_angle` | coFloat | Strength | expert | 45 | Lightning overhang angle |
| `lightning_prune_angle` | coFloat | Strength | expert | 45 | Prune angle |
| `lightning_straightening_angle` | coFloat | Strength | expert | 45 | Straightening angle |
| `line_width` | coFloatOrPercent | Quality | advanced | {"value": 0.0, "percent": false} | Default |
| `long_retractions_when_cut` | coBools |  | develop | [false] | Long retraction when cut (beta) |
| `long_retractions_when_ec` | coBools |  | advanced | [false] | Long retraction when extruder change |
| `machine_bed_mass_Y` | coFloat | Machine limits | develop | 0 |  |
| `machine_end_gcode` | coString |  | advanced | M104 S0 ; turn off temperature G28 X0  ; home X axis M84     ; disable motors  | End G-code |
| `machine_hotend_change_time` | coFloat |  | advanced | 0.0 | Hotend change time |
| `machine_load_filament_time` | coFloat |  | advanced | 0.0 | Filament load time |
| `machine_max_acceleration_e` | coFloats | Machine limits | simple | [5000, 5000] |  |
| `machine_max_acceleration_extruding` | coFloats | Machine limits | simple | [1500.0, 1250.0] |  |
| `machine_max_acceleration_retracting` | coFloats | Machine limits | simple | [1500.0, 1250.0] |  |
| `machine_max_acceleration_travel` | coFloats | Machine limits | advanced | [0.0, 0.0] |  |
| `machine_max_acceleration_x` | coFloats | Machine limits | simple | [1000, 1000] |  |
| `machine_max_acceleration_y` | coFloats | Machine limits | simple | [1000, 1000] |  |
| `machine_max_acceleration_z` | coFloats | Machine limits | simple | [500, 200] |  |
| `machine_max_force_Y` | coFloat | Machine limits | develop | 0 |  |
| `machine_max_jerk_e` | coFloats | Machine limits | simple | [2.5, 2.5] |  |
| `machine_max_jerk_x` | coFloats | Machine limits | simple | [10, 10] |  |
| `machine_max_jerk_y` | coFloats | Machine limits | simple | [10, 10] |  |
| `machine_max_jerk_z` | coFloats | Machine limits | simple | [0.2, 0.4] |  |
| `machine_max_junction_deviation` | coFloats | Machine limits | advanced | [0.01] |  |
| `machine_max_printed_mass` | coFloat | Machine limits | develop | 0 |  |
| `machine_max_speed_e` | coFloats | Machine limits | simple | [120, 120] |  |
| `machine_max_speed_x` | coFloats | Machine limits | simple | [500, 200] |  |
| `machine_max_speed_y` | coFloats | Machine limits | simple | [500, 200] |  |
| `machine_max_speed_z` | coFloats | Machine limits | simple | [12, 12] |  |
| `machine_min_extruding_rate` | coFloats | Machine limits | develop | [0.0, 0.0] |  |
| `machine_min_travel_rate` | coFloats | Machine limits | develop | [0.0, 0.0] |  |
| `machine_pause_gcode` | coString |  | advanced |  | Pause G-code |
| `machine_prepare_compensation_time` | coFloat |  | develop | 260 |  |
| `machine_start_gcode` | coString |  | advanced | G28 ; home all axes G1 Z5 F5000 ; lift nozzle  | Start G-code |
| `machine_tool_change_time` | coFloat |  | advanced | 0.0 | Tool change time |
| `machine_unload_filament_time` | coFloat |  | advanced | 0.0 | Filament unload time |
| `make_overhang_printable` | coBool | Quality | advanced | False | Make overhangs printable |
| `make_overhang_printable_angle` | coFloat | Quality | advanced | 55.0 | Make overhangs printable - Maximum angle |
| `make_overhang_printable_hole_size` | coFloat | Quality | advanced | 0.0 | Make overhangs printable - Hole area |
| `manual_filament_change` | coBool |  | advanced | False | Manual Filament Change |
| `master_extruder_id` | coInt |  | simple | 1 | Master extruder id |
| `max_bridge_length` | coFloat | Support | advanced | 10 | Max bridge length |
| `max_layer_height` | coFloats |  | advanced | [0.0] | Max |
| `max_resonance_avoidance_speed` | coFloat |  | advanced | 120 | Max |
| `max_travel_detour_distance` | coFloatOrPercent | Quality | advanced | {"value": 0.0, "percent": false} | Avoid crossing walls - Max detour length |
| `max_volumetric_extrusion_rate_slope` | coFloat |  | advanced | 0 | Extrusion rate smoothing |
| `max_volumetric_extrusion_rate_slope_segment_length` | coFloat |  | advanced | 3.0 | Smoothing segment length |
| `min_bead_width` | coPercent | Quality | advanced | 85 | Minimum wall width |
| `min_feature_size` | coPercent | Quality | advanced | 25 | Minimum feature size |
| `min_layer_height` | coFloats |  | advanced | [0.07] | Min |
| `min_length_factor` | coFloat | Quality | advanced | 0.5 | Minimum wall length |
| `min_resonance_avoidance_speed` | coFloat |  | advanced | 70 | Min |
| `min_skirt_length` | coFloat |  | advanced | 0.0 | Skirt minimum extrusion length |
| `min_width_top_surface` | coFloatOrPercent | Quality | advanced | {"value": 300.0, "percent": true} | One wall threshold |
| `minimum_sparse_infill_area` | coFloat | Strength | advanced | 15 | Minimum sparse infill threshold |
| `mmu_segmented_region_interlocking_depth` | coFloat | Advanced | advanced | 0.0 | Interlocking depth of a segmented region |
| `mmu_segmented_region_max_width` | coFloat | Advanced | advanced | 0.0 | Maximum width of a segmented region |
| `notes` | coString |  | advanced |  | Configuration notes |
| `nozzle_diameter` | coFloats |  | advanced | [0.4] | Nozzle diameter |
| `nozzle_flush_dataset` | coInts |  | simple | [0] |  |
| `nozzle_height` | coFloat |  | develop | 2.5 | Nozzle height |
| `nozzle_hrc` | coInt |  | develop | 0 | Nozzle HRC |
| `nozzle_temperature` | coInts |  | simple | [200] | Other layers |
| `nozzle_temperature_initial_layer` | coInts |  | simple | [200] | First layer |
| `nozzle_temperature_range_high` | coInts |  | simple | [240] | Max |
| `nozzle_temperature_range_low` | coInts |  | simple | [190] | Min |
| `nozzle_type` | coEnums |  | advanced | { ntUndefine } | Nozzle type |
| `nozzle_volume` | coFloats |  | advanced | [0.0] | Nozzle volume |
| `nozzle_volume_type` | coEnums |  | simple | ["Standard"] | Nozzle Volume Type |
| `only_one_wall_first_layer` | coBool | Quality | simple | False | Only one wall on first layer |
| `only_one_wall_top` | coBool | Quality | simple | False | Only one wall on top surfaces |
| `ooze_prevention` | coBool |  | advanced | False | Enable |
| `other_layers_print_sequence` | coInts |  | simple | [0] | Other layers print sequence |
| `other_layers_print_sequence_nums` | coInt |  | simple | 0 | The number of other layers print sequence |
| `other_layers_sequence_choice` | coEnum | Quality | simple | Auto | Other layers filament sequence |
| `outer_wall_acceleration` | coFloats | Speed | advanced | [500] | Outer wall |
| `outer_wall_filament_id` | coInt | Extruders | advanced | 0 | Outer walls |
| `outer_wall_flow_ratio` | coFloat | Advanced | advanced | 1 | Outer wall flow ratio |
| `outer_wall_jerk` | coFloats | Speed | advanced | [9] | Outer wall |
| `outer_wall_line_width` | coFloatOrPercent | Quality | advanced | {"value": 0.0, "percent": false} | Outer wall |
| `outer_wall_speed` | coFloats | Speed | advanced | [60] | Outer wall |
| `outer_wall_volumetric_flow` | coFloatsOrPercents | Speed | advanced | {FloatOrPercent(50, true)} | Outer wall |
| `overhang_1_4_speed` | coFloatsOrPercents | Speed | advanced | {FloatOrPercent(0, false)} | 10% |
| `overhang_2_4_speed` | coFloatsOrPercents | Speed | advanced | {FloatOrPercent(0, false)} | 25% |
| `overhang_3_4_speed` | coFloatsOrPercents | Speed | advanced | {FloatOrPercent(0, false)} | 50% |
| `overhang_4_4_speed` | coFloatsOrPercents | Speed | advanced | {FloatOrPercent(0, false)} | 75% |
| `overhang_fan_speed` | coInts |  | advanced | [100] | Overhangs and external bridges fan speed |
| `overhang_fan_threshold` | coEnums |  | advanced | ["95%"] | Overhang cooling activation threshold |
| `overhang_flow_ratio` | coFloat | Advanced | advanced | 1 | Overhang flow ratio |
| `overhang_reverse` | coBool | Quality | advanced | False | Reverse on even |
| `overhang_reverse_internal_only` | coBool | Quality | advanced | False | Reverse only internal perimeters |
| `overhang_reverse_threshold` | coFloatOrPercent | Quality | advanced | {"value": 50.0, "percent": true} | Reverse threshold |
| `parking_pos_retraction` | coFloat |  | advanced | 92.0 | Filament parking position |
| `part_cooling_fan_min_pwm` | coInt |  | advanced | 0 | Minimum non-zero part cooling fan speed |
| `pellet_flow_coefficient` | coFloats |  | simple | [0.4157] | Pellet flow coefficient |
| `pellet_modded_printer` | coBool |  | simple | False | Pellet Modded Printer |
| `physical_extruder_map` | coInts |  | develop | [0] | Map the logical extruder to physical extruder |
| `plugins` | coStrings |  | develop |  | Plugins Used |
| `post_process` | coStrings |  | advanced |  | Post-processing Scripts |
| `precise_outer_wall` | coBool | Quality | simple | True | Precise wall |
| `precise_z_height` | coBool | Quality | advanced | 0 | Precise Z height |
| `preheat_steps` | coInt |  | develop | 1 | Preheat steps |
| `preheat_time` | coFloat |  | advanced | 30.0 | Preheat time |
| `pressure_advance` | coFloats |  | advanced | [0.02] | Pressure advance |
| `prime_tower_brim_width` | coFloat |  | advanced | 3.0 | Brim width |
| `prime_tower_enable_framework` | coBool |  | advanced | False | Internal ribs |
| `prime_tower_flat_ironing` | coBool |  | advanced | False |  |
| `prime_tower_infill_gap` | coPercent |  | advanced | 150 | Infill gap |
| `prime_tower_skip_points` | coBool |  | advanced | True | Skip points |
| `prime_tower_width` | coFloat |  | simple | 60.0 | Width |
| `prime_volume` | coFloat |  | simple | 45.0 | Prime volume |
| `prime_volume_mode` | coEnum |  | develop | Default | Prime volume mode |
| `print_compatible_printers` | coStrings |  | simple |  |  |
| `print_extruder_id` | coInts |  | simple | [1] | Print extruder id |
| `print_extruder_variant` | coStrings |  | simple | ["Direct Drive Standard"] | Print's extruder variant |
| `print_flow_ratio` | coFloat | Quality | advanced | 1.f | Flow ratio |
| `print_order` | coEnum |  | advanced | default | Intra-layer order |
| `print_sequence` | coEnum |  | simple | by layer | Print sequence |
| `print_settings_id` | coString |  | simple |  |  |
| `printer_extruder_id` | coInts |  | simple | [1] | Printer extruder id |
| `printer_extruder_variant` | coStrings |  | simple | ["Direct Drive Standard"] | Printer's extruder variant |
| `printer_model` | coString |  | simple |  | Printer type |
| `printer_notes` | coString |  | advanced |  | Printer notes |
| `printer_settings_id` | coString |  | simple |  |  |
| `printer_structure` | coEnum |  | develop | undefine | Printer structure |
| `printer_variant` | coString |  | simple |  | Printer variant |
| `printing_by_object_gcode` | coString |  | advanced |  | Between Object G-code |
| `process_change_extrusion_role_gcode` | coString |  | advanced |  | Change extrusion role G-code (process) |
| `purge_in_prime_tower` | coBool |  | advanced | True | Purge in prime tower |
| `raft_contact_distance` | coFloat | Support | advanced | 0.1 | Raft contact Z distance |
| `raft_expansion` | coFloat | Support | advanced | 1.5 | Raft expansion |
| `raft_first_layer_density` | coPercent | Support | advanced | 90 | First layer density |
| `raft_first_layer_expansion` | coFloat | Support | advanced | 2.0 | First layer expansion |
| `raft_layers` | coInt | Support | advanced | 0 | Raft layers |
| `reduce_crossing_wall` | coBool | Quality | advanced | False | Avoid crossing walls |
| `reduce_fan_stop_start_freq` | coBools |  | simple | [false] | Keep fan always on |
| `reduce_infill_retraction` | coBool |  | advanced | False | Reduce infill retraction |
| `relative_bridge_angle` | coBool | Strength | advanced | False | Relative bridge angle |
| `required_nozzle_HRC` | coInts |  | develop | [0] | Required nozzle HRC |
| `resolution` | coFloat |  | advanced | 0.01 | Resolution |
| `resonance_avoidance` | coBool |  | advanced | False | Resonance avoidance |
| `retract_after_outer_wall` | coBool | Quality | develop | True | Retract after outer wall |
| `retract_after_wipe` | coPercents |  | expert | [0] | Retract amount after wipe |
| `retract_before_wipe` | coPercents |  | advanced | [100] | Retract amount before wipe |
| `retract_length_toolchange` | coFloats |  | advanced | [10.0] | Retraction Length (Toolchange) |
| `retract_lift_above` | coFloats |  | advanced | [0.0] | Only lift Z above |
| `retract_lift_below` | coFloats |  | advanced | [0.0] | Only lift Z below |
| `retract_lift_enforce` | coEnums |  | advanced | ["All Surfaces"] | On surfaces |
| `retract_restart_extra` | coFloats |  | advanced | [0.0] | Extra length on restart |
| `retract_restart_extra_toolchange` | coFloats |  | advanced | [0.0] | Extra length on restart (Toolchange) |
| `retract_when_changing_layer` | coBools |  | advanced | [false] | Retract on layer change |
| `retraction_distances_when_cut` | coFloats |  | develop | [18] | Retraction distance when cut |
| `retraction_distances_when_ec` | coFloats |  | advanced | [10] | Retraction distance when extruder change |
| `retraction_length` | coFloats |  | simple | [0.8] | Length |
| `retraction_minimum_travel` | coFloats |  | advanced | [2.0] | Travel distance threshold |
| `retraction_speed` | coFloats |  | advanced | [30.0] | Retraction speed |
| `role_based_wipe_speed` | coBool | Speed | advanced | True | Role based wipe speed |
| `scan_first_layer` | coBool |  | advanced | False | Scan first layer |
| `scarf_angle_threshold` | coInt | Quality | advanced | 155 | Conditional angle threshold |
| `scarf_joint_flow_ratio` | coFloat | Quality | expert | 1 | Scarf joint flow ratio |
| `scarf_joint_speed` | coFloatOrPercent | Quality | advanced | {"value": 100.0, "percent": true} | Scarf joint speed |
| `scarf_overhang_threshold` | coPercent | Quality | advanced | 40 | Conditional overhang threshold |
| `seam_gap` | coFloatOrPercent | Quality | advanced | {"value": 10.0, "percent": true} | Seam gap |
| `seam_position` | coEnum | Quality | simple | aligned | Seam position |
| `seam_slope_conditional` | coBool | Quality | advanced | False | Conditional scarf joint |
| `seam_slope_entire_loop` | coBool | Quality | advanced | False | Scarf around entire wall |
| `seam_slope_inner_walls` | coBool | Quality | advanced | False | Scarf joint for inner walls |
| `seam_slope_min_length` | coFloat | Quality | advanced | 20 | Scarf length |
| `seam_slope_start_height` | coFloatOrPercent | Quality | advanced | {"value": 0.0, "percent": false} | Scarf start height |
| `seam_slope_steps` | coInt | Quality | advanced | 10 | Scarf steps |
| `seam_slope_type` | coEnum | Quality | advanced | none | Scarf joint seam (beta) |
| `separated_infills` | coBool | Strength | expert | False | Separated infills |
| `set_other_flow_ratios` | coBool | Advanced | advanced | False | Set other flow ratios |
| `single_extruder_multi_material` | coBool |  | advanced | True | Single Extruder Multi Material |
| `single_extruder_multi_material_priming` | coBool |  | advanced | False | Prime all printing extruders |
| `single_loop_draft_shield` | coBool |  | advanced | False | Single loop after first layer |
| `skeleton_infill_density` | coPercent | Strength | advanced | 25 | Skeleton infill density |
| `skeleton_infill_line_width` | coFloatOrPercent | Strength | advanced | {"value": 100.0, "percent": true} | Skeleton line width |
| `skin_infill_density` | coPercent | Strength | advanced | 25 | Skin infill density |
| `skin_infill_depth` | coFloat | Strength | advanced | 2.0 | Skin infill depth |
| `skin_infill_line_width` | coFloatOrPercent | Strength | advanced | {"value": 100.0, "percent": true} | Skin line width |
| `skirt_distance` | coFloat |  | advanced | 2 | Skirt distance |
| `skirt_height` | coInt |  | simple | 1 | Skirt height |
| `skirt_loops` | coInt |  | simple | 1 | Skirt loops |
| `skirt_speed` | coFloat |  | advanced | 50.0 | Skirt speed |
| `skirt_start_angle` | coFloat | Support | advanced | -135 | Skirt start point |
| `skirt_type` | coEnum |  | advanced | combined | Skirt type |
| `slice_closing_radius` | coFloat | Quality | advanced | 0.049 | Slice gap closing radius |
| `slicing_mode` | coEnum | Others | advanced | regular | Slicing Mode |
| `slicing_pipeline_plugin` | coStrings |  | advanced |  | Slicing Pipeline Plugin |
| `slow_down_for_layer_cooling` | coBools |  | simple | [true] | Slow printing down for better layer cooling |
| `slow_down_layer_time` | coFloats |  | simple | [5.0] | Layer time |
| `slow_down_layers` | coInt | Speed | advanced | 0 | Number of slow layers |
| `slow_down_min_speed` | coFloats |  | advanced | [10.0] | Min print speed |
| `slowdown_for_curled_perimeters` | coBools | Speed | advanced | [false] | Slow down for curled perimeters |
| `small_area_infill_flow_compensation` | coBool | Quality | advanced | False | Small area flow compensation (beta) |
| `small_area_infill_flow_compensation_model` | coStrings |  | advanced | ["0,0", "\\n0.2,0.4444", "\\n0.4,0.6145", "\\n0.6,0.7059", "\\n0.8,0.7619", "\\n1.5,0.8571", "\\n2,0.8889", "\\n3,0.9231", "\\n5,0.9520", "\\n10,1"] | Flow Compensation Model |
| `small_perimeter_speed` | coFloatsOrPercents | Speed | advanced | {FloatOrPercent(50, true)} | Small perimeters |
| `small_perimeter_threshold` | coFloats | Speed | advanced | [0] | Small perimeters threshold |
| `small_support_perimeter_speed` | coFloatsOrPercents | Speed | advanced | {FloatOrPercent(50, true)} | Small support perimeters |
| `small_support_perimeter_threshold` | coFloats | Speed | advanced | [0] | Small support perimeters threshold |
| `solid_infill_direction` | coFloat | Strength | advanced | 45 | Solid infill direction |
| `solid_infill_rotate_template` | coString | Strength | advanced |  | Solid infill rotation template |
| `sparse_infill_acceleration` | coFloatsOrPercents | Speed | advanced | {FloatOrPercent(100, true)} | Sparse infill |
| `sparse_infill_density` | coPercent | Strength | simple | 20 | Sparse infill density |
| `sparse_infill_filament_id` | coInt | Extruders | advanced | 0 | Infill |
| `sparse_infill_flow_ratio` | coFloat | Advanced | advanced | 1 | Sparse infill flow ratio |
| `sparse_infill_line_width` | coFloatOrPercent | Quality | advanced | {"value": 0.0, "percent": false} | Sparse infill |
| `sparse_infill_pattern` | coEnum | Strength | simple | crosshatch | Sparse infill pattern |
| `sparse_infill_rotate_template` | coString | Strength | advanced |  | Sparse infill rotation template |
| `sparse_infill_smooth_factor` | coPercent | Strength | advanced | 0 | Sparse infill smooth factor |
| `sparse_infill_speed` | coFloats | Speed | advanced | [100] | Sparse infill |
| `sparse_infill_strategy` | coEnum | Strength | expert | default | Sparse infill strategy |
| `sparse_infill_volumetric_flow` | coFloatsOrPercents | Speed | advanced | {FloatOrPercent(100, true)} | Sparse infill |
| `spiral_finishing_flow_ratio` | coFloat |  | advanced | 0 | Spiral finishing flow ratio |
| `spiral_mode` | coBool |  | simple | False | Spiral vase |
| `spiral_mode_max_xy_smoothing` | coFloatOrPercent |  | advanced | {"value": 200.0, "percent": true} | Max XY Smoothing |
| `spiral_mode_smooth` | coBool |  | simple | False | Smooth Spiral |
| `spiral_starting_flow_ratio` | coFloat |  | advanced | 0 | Spiral starting flow ratio |
| `staggered_inner_seams` | coBool | Quality | advanced | False | Staggered inner seams |
| `standby_temperature_delta` | coInt |  | advanced | -5 | Temperature variation |
| `start_end_points` | coPoints |  | develop | ["30x-3", "54x245"] | Start end points |
| `supertack_plate_temp` | coInts |  | simple | [35] | Other layers |
| `supertack_plate_temp_initial_layer` | coInts |  | simple | [35] | First layer |
| `support_air_filtration` | coBool |  | develop | True | Support air filtration |
| `support_angle` | coFloat | Support | advanced | 0 | Pattern angle |
| `support_base_pattern` | coEnum | Support | advanced | default | Base pattern |
| `support_base_pattern_spacing` | coFloat | Support | advanced | 2.5 | Base pattern spacing |
| `support_bottom_interface_spacing` | coFloat | Support | advanced | 0.5 | Bottom interface spacing |
| `support_bottom_z_distance` | coFloat | Support | advanced | 0.2 | Bottom Z distance |
| `support_chamber_temp_control` | coBool |  | advanced | True | Support controlling chamber temperature |
| `support_cooling_filter` | coBool |  | advanced | False |  |
| `support_critical_regions_only` | coBool | Support | advanced | False | Support critical regions only |
| `support_expansion` | coFloat | Support | advanced | 0 | Normal support expansion |
| `support_fast_purge_mode` | coBool |  | develop | False | Support fast purge mode |
| `support_filament` | coInt | Support | simple | 0 | Support/raft base |
| `support_flow_ratio` | coFloat | Advanced | advanced | 1 | Support flow ratio |
| `support_interface_bottom_layers` | coInt | Support | advanced | 0 | Bottom interface layers |
| `support_interface_filament` | coInt | Support | simple | 0 | Support/raft interface |
| `support_interface_flow_ratio` | coFloat | Advanced | advanced | 1 | Support interface flow ratio |
| `support_interface_loop_pattern` | coBool | Support | advanced | False | Loop pattern interface |
| `support_interface_not_for_body` | coBool | Support | simple | True | Avoid interface filament for base |
| `support_interface_pattern` | coEnum | Support | advanced | auto | Interface pattern |
| `support_interface_spacing` | coFloat | Support | advanced | 0.5 | Top interface spacing |
| `support_interface_speed` | coFloats | Speed | advanced | [80] | Support interface |
| `support_interface_top_layers` | coInt | Support | advanced | 3 | Top interface layers |
| `support_interface_volumetric_flow` | coFloatsOrPercents | Speed | advanced | {FloatOrPercent(50, true)} | Support interface |
| `support_ironing` | coBool | Support | advanced | False | Ironing Support Interface |
| `support_ironing_flow` | coPercent | Support | advanced | 10 | Support Ironing flow |
| `support_ironing_pattern` | coEnum | Support | advanced | rectilinear | Support Ironing Pattern |
| `support_ironing_spacing` | coFloat | Support | advanced | 0.1 | Support Ironing line spacing |
| `support_line_width` | coFloatOrPercent | Quality | advanced | {"value": 0.0, "percent": false} | Support |
| `support_material_interface_fan_speed` | coInts |  | advanced | [-1] | Support interface fan speed |
| `support_multi_bed_types` | coBool |  | simple | False | Support multi bed types |
| `support_object_first_layer_gap` | coFloat | Support | advanced | 0.2 | Support/object first layer gap |
| `support_object_skip_flush` | coBool |  | simple | False |  |
| `support_object_xy_distance` | coFloat | Support | advanced | 0.35 | Support/object XY distance |
| `support_on_build_plate_only` | coBool | Support | simple | False | On build plate only |
| `support_remove_small_overhang` | coBool | Support | advanced | True | Ignore small overhangs |
| `support_speed` | coFloats | Speed | advanced | [80] | Support |
| `support_style` | coEnum | Support | advanced | default | Style |
| `support_threshold_angle` | coInt | Support | simple | 30 | Threshold angle |
| `support_threshold_overlap` | coFloatOrPercent | Support | simple | {"value": 50.0, "percent": true} | Threshold overlap |
| `support_top_z_distance` | coFloat | Support | advanced | 0.2 | Top Z distance |
| `support_type` | coEnum | Support | simple | normal(auto) | Type |
| `support_volumetric_flow` | coFloatsOrPercents | Speed | advanced | {FloatOrPercent(100, true)} | Support |
| `symmetric_infill_y_axis` | coBool | Strength | advanced | False | Symmetric infill Y axis |
| `temperature_vitrification` | coInts |  | simple | [100] | Softening temperature |
| `template_custom_gcode` | coString |  | advanced |  | Custom G-code |
| `textured_cool_plate_temp` | coInts |  | simple | [40] | Other layers |
| `textured_cool_plate_temp_initial_layer` | coInts |  | simple | [40] | First layer |
| `textured_plate_temp` | coInts |  | simple | [45] | Other layers |
| `textured_plate_temp_initial_layer` | coInts |  | simple | [45] | First layer |
| `thick_bridges` | coBool | Quality | advanced | False | Thick external bridges |
| `thick_internal_bridges` | coBool | Quality | advanced | True | Thick internal bridges |
| `thumbnails` | coString |  | advanced | 48x48/PNG,300x300/PNG | G-code thumbnails |
| `thumbnails_format` | coEnum |  | advanced | PNG | Format of G-code thumbnails |
| `time_cost` | coFloat |  | advanced | 0 | Time cost |
| `time_lapse_gcode` | coString |  | advanced |  | Timelapse G-code |
| `timelapse_type` | coEnum |  | simple | 0 | Timelapse |
| `tool_change_on_wipe_tower` | coBool |  | advanced | False | Tool change on wipe tower |
| `toolchange_cyclic_first_layer` | coBool | Advanced | expert | False | Apply cyclic order to first layer |
| `toolchange_cyclic_order` | coString | Advanced | expert |  | Cyclic order |
| `toolchange_ordering` | coEnum | Advanced | advanced | default | Toolchange ordering |
| `top_bottom_infill_wall_overlap` | coPercent | Strength | advanced | 25 | Top/Bottom solid infill/wall overlap |
| `top_color_penetration_layers` | coInt | Strength | advanced | 0 | Top paint penetration layers |
| `top_layer_direction` | coFloat | Strength | simple | -1 | Top layer direction |
| `top_shell_layers` | coInt | Strength | simple | 4 | Top shell layers |
| `top_shell_thickness` | coFloat | Strength | simple | 0.6 | Top shell thickness |
| `top_solid_infill_flow_ratio` | coFloats | Advanced | advanced | [1] | Top surface flow ratio |
| `top_surface_acceleration` | coFloats | Speed | advanced | [500] | Top surface |
| `top_surface_density` | coPercent | Strength | simple | 100 | Top surface density |
| `top_surface_expansion` | coFloat | Strength | advanced | 0 | Top surface expansion |
| `top_surface_expansion_direction` | coEnum | Strength | advanced | inward_and_outward | Top expansion direction |
| `top_surface_expansion_margin` | coFloat | Strength | advanced | 0 | Top expansion wall margin |
| `top_surface_filament_id` | coInt | Extruders | advanced | 0 | Top surface |
| `top_surface_fill_order` | coEnum | Strength | advanced | default | Top surface fill order |
| `top_surface_jerk` | coFloats | Speed | advanced | [9] | Top surface |
| `top_surface_line_width` | coFloatOrPercent | Quality | advanced | {"value": 0.0, "percent": false} | Top surface |
| `top_surface_pattern` | coEnum | Strength | simple | monotonicline | Top surface pattern |
| `top_surface_speed` | coFloats | Speed | advanced | [100] | Top surface |
| `top_surface_volumetric_flow` | coFloatsOrPercents | Speed | advanced | {FloatOrPercent(50, true)} | Top surface |
| `travel_acceleration` | coFloats | Speed | advanced | [10000.0] | Travel |
| `travel_jerk` | coFloats | Speed | advanced | [12] | Travel |
| `travel_slope` | coFloats |  | advanced | [3] | Traveling angle |
| `travel_speed` | coFloats |  | advanced | [120] | Travel |
| `travel_speed_z` | coFloats |  | develop | [0.0] |  |
| `tree_support_angle_slow` | coFloat | Support | advanced | 25 | Preferred Branch Angle |
| `tree_support_auto_brim` | coBool | Quality | simple | 1 | Auto brim width |
| `tree_support_branch_angle` | coFloat | Support | advanced | 40.0 | Tree support branch angle |
| `tree_support_branch_angle_organic` | coFloat | Support | advanced | 40.0 | Tree support branch angle |
| `tree_support_branch_diameter` | coFloat | Support | advanced | 5.0 | Tree support branch diameter |
| `tree_support_branch_diameter_angle` | coFloat | Support | advanced | 5 | Branch Diameter Angle |
| `tree_support_branch_diameter_organic` | coFloat | Support | advanced | 2.0 | Tree support branch diameter |
| `tree_support_branch_distance` | coFloat | Support | advanced | 5.0 | Tree support branch distance |
| `tree_support_branch_distance_organic` | coFloat | Support | advanced | 1.0 | Tree support branch distance |
| `tree_support_brim_width` | coFloat | Quality | simple | 3 | Tree support brim width |
| `tree_support_tip_diameter` | coFloat | Support | advanced | 0.8 | Tip Diameter |
| `tree_support_top_rate` | coPercent | Support | advanced | 30 | Branch Density |
| `tree_support_wall_count` | coInt | Support | advanced | 0 | Support wall loops |
| `tree_support_with_infill` | coBool | Support | advanced | False | Tree support with infill |
| `unsupported_wall_last` | coBool | Quality | advanced | False | Print unsupported walls last |
| `upward_compatible_machine` | coStrings |  | advanced |  | upward compatible machine |
| `use_firmware_retraction` | coBool |  | advanced | False | Use firmware retraction |
| `use_relative_e_distances` | coBool |  | advanced | True | Use relative E distances |
| `volumetric_speed_coefficients` | coStrings |  | simple | [""] | Max volumetric speed multinomial coefficients |
| `wait_for_temp_on_wipe_tower` | coBool |  | advanced | False | Wait for temperature on wipe tower |
| `wall_direction` | coEnum | Quality | advanced | ccw | Wall loop direction |
| `wall_distribution_count` | coInt | Quality | advanced | 1 | Wall distribution count |
| `wall_generator` | coEnum | Quality | advanced | arachne | Wall generator |
| `wall_loops` | coInt | Strength | simple | 2 | Wall loops |
| `wall_maximum_deviation` | coFloat | Quality | expert | 0.025f | Maximum wall deviation |
| `wall_maximum_resolution` | coFloat | Quality | expert | 0.5f | Maximum wall resolution |
| `wall_sequence` | coEnum | Quality | advanced | inner wall/outer wall | Walls printing order |
| `wall_transition_angle` | coFloat | Quality | advanced | 10.0 | Wall transitioning threshold angle |
| `wall_transition_filter_deviation` | coPercent | Quality | advanced | 25 | Wall transitioning filter margin |
| `wall_transition_length` | coPercent | Quality | advanced | 100 | Wall transition length |
| `wipe` | coBools |  | advanced | [false] | Wipe while retracting |
| `wipe_before_external_loop` | coBool | Quality | advanced | False | Wipe before external loop |
| `wipe_distance` | coFloats |  | advanced | [1.0] | Wipe distance |
| `wipe_inward` | coBool | Quality | expert | False | Wipe inward |
| `wipe_inward_distance` | coFloatOrPercent | Quality | expert | {"value": 50.0, "percent": true} | Wipe inward distance |
| `wipe_on_loops` | coBool | Quality | advanced | False | Wipe on loops |
| `wipe_speed` | coFloatOrPercent | Speed | advanced | {"value": 80.0, "percent": true} | Wipe speed |
| `wipe_tower_bridging` | coFloat |  | advanced | 10.0 | Maximal bridging distance |
| `wipe_tower_cone_angle` | coFloat |  | advanced | 30.0 | Stabilization cone apex angle |
| `wipe_tower_extra_flow` | coPercent |  | advanced | 100.0 | Extra flow for purging |
| `wipe_tower_extra_rib_length` | coFloat |  | advanced | 0 | Extra rib length |
| `wipe_tower_extra_spacing` | coPercent |  | advanced | 100.0 | Wipe tower purge lines spacing |
| `wipe_tower_filament` | coInt | Extruders | advanced | 0 | Wipe tower |
| `wipe_tower_fillet_wall` | coBool |  | advanced | True | Fillet wall |
| `wipe_tower_max_purge_speed` | coFloat |  | advanced | 90.0 | Maximum wipe tower print speed |
| `wipe_tower_no_sparse_layers` | coBool |  | advanced | False | No sparse layers (beta) |
| `wipe_tower_rib_width` | coFloat |  | advanced | 8 | Rib width |
| `wipe_tower_rotation_angle` | coFloat |  | advanced | 0.0 | Wipe tower rotation angle |
| `wipe_tower_sparse_layers_combination` | coBool |  | advanced | False | Combine sparse layers |
| `wipe_tower_type` | coEnum |  | advanced | type2 | Wipe tower type |
| `wipe_tower_wall_type` | coEnum |  | advanced | rib | Wall type |
| `wipe_tower_x` | coFloats |  | develop | [15.0] |  |
| `wipe_tower_y` | coFloats |  | develop | [220.0] |  |
| `wiping_volumes_extruders` | coFloats |  | simple | [70.0, 70.0, 70.0, 70.0, 70.0, 70.0, 70.0, 70.0, 70.0, 70.0] | Purging volumes - load/unload volumes |
| `wrapping_detection_gcode` | coString |  | advanced |  | Clumping detection G-code |
| `wrapping_detection_layers` | coInt |  | develop | 20 | Clumping detection layers |
| `wrapping_exclude_area` | coPoints |  | advanced |  | Probing exclude area of clumping |
| `xy_contour_compensation` | coFloat | Quality | advanced | 0 | X-Y contour compensation |
| `xy_hole_compensation` | coFloat | Quality | advanced | 0 | X-Y hole compensation |
| `z_hop` | coFloats |  | simple | [0.4] | Z-hop height |
| `z_hop_types` | coEnums |  | advanced | ["Slope Lift"] | Z-hop type |
| `z_offset` | coFloat |  | advanced | 0 | Z offset |
| `zaa_dont_alternate_fill_direction` | coBool | Quality | expert | False | Don't alternate fill direction |
| `zaa_enabled` | coBool | Quality | expert | False | Z contouring enabled |
| `zaa_min_z` | coFloat | Quality | expert | 0.05 | Minimum Z height |
| `zaa_minimize_perimeter_height` | coFloat | Quality | expert | 35 | Minimize wall height angle |

## Groupe `sla` (76 paramètres)

| Clé | Type | Catégorie | Mode | Défaut | Libellé |
|---|---|---|---|---|---|
| `absolute_correction` | coFloat |  | advanced | 0.0 |  |
| `area_fill` | coFloat |  | advanced | 50.0 |  |
| `bottle_cost` | coFloat |  | simple | 0.0 |  |
| `bottle_volume` | coFloat |  | simple | 1000.0 |  |
| `bottle_weight` | coFloat |  | simple | 1.0 |  |
| `default_sla_material_profile` | coString |  | simple |  |  |
| `default_sla_print_profile` | coString |  | simple |  |  |
| `display_height` | coFloat |  | simple | 68.0 |  |
| `display_mirror_x` | coBool |  | advanced | True |  |
| `display_mirror_y` | coBool |  | advanced | False |  |
| `display_orientation` | coEnum |  | advanced | portrait |  |
| `display_pixels_x` | coInt |  | simple | 2560 | XAxis |
| `display_pixels_y` | coInt |  | simple | 1440 | YAxis |
| `display_width` | coFloat |  | simple | 120.0 |  |
| `elefant_foot_min_width` | coFloat |  | advanced | 0.2 |  |
| `exposure_time` | coFloat |  | simple | 10 |  |
| `faded_layers` | coInt |  | advanced | 10 |  |
| `fast_tilt_time` | coFloat |  | advanced | 5.0 |  |
| `gamma_correction` | coFloat |  | advanced | 1.0 |  |
| `hollowing_closing_distance` | coFloat |  | advanced | 2.0 |  |
| `hollowing_enable` | coBool |  | simple | False |  |
| `hollowing_min_thickness` | coFloat |  | simple | 3.0 |  |
| `hollowing_quality` | coFloat |  | advanced | 0.5 |  |
| `initial_exposure_time` | coFloat |  | simple | 15 |  |
| `initial_layer_height` | coFloat |  | simple | 0.3 |  |
| `material_colour` | coString |  | simple | #29B2B2 |  |
| `material_correction` | coFloats |  | advanced | [1.0, 1.0, 1.0] |  |
| `material_correction_x` | coFloat |  | advanced | 1.0 |  |
| `material_correction_y` | coFloat |  | advanced | 1.0 |  |
| `material_correction_z` | coFloat |  | advanced | 1.0 |  |
| `material_density` | coFloat |  | simple | 1.0 |  |
| `material_print_speed` | coEnum |  | advanced | fast |  |
| `material_type` | coString |  | simple | Tough |  |
| `material_vendor` | coString |  | simple |  |  |
| `max_exposure_time` | coFloat |  | advanced | 100 |  |
| `max_initial_exposure_time` | coFloat |  | advanced | 150 |  |
| `min_exposure_time` | coFloat |  | advanced | 0 |  |
| `min_initial_exposure_time` | coFloat |  | advanced | 0 |  |
| `pad_around_object` | coBool |  | simple | False |  |
| `pad_around_object_everywhere` | coBool |  | simple | False |  |
| `pad_brim_size` | coFloat |  | advanced | 1.6 |  |
| `pad_enable` | coBool |  | simple | True |  |
| `pad_max_merge_distance` | coFloat |  | advanced | 50.0 |  |
| `pad_object_connector_penetration` | coFloat |  | advanced | 0.3 |  |
| `pad_object_connector_stride` | coFloat |  | advanced | 10 |  |
| `pad_object_connector_width` | coFloat |  | advanced | 0.5 |  |
| `pad_object_gap` | coFloat |  | advanced | 1 |  |
| `pad_wall_height` | coFloat |  | advanced | 0.0 |  |
| `pad_wall_slope` | coFloat |  | advanced | 90.0 |  |
| `pad_wall_thickness` | coFloat |  | simple | 2.0 |  |
| `relative_correction` | coFloats |  | advanced | [1.0, 1.0] |  |
| `relative_correction_x` | coFloat |  | advanced | 1.0 |  |
| `relative_correction_y` | coFloat |  | advanced | 1.0 |  |
| `relative_correction_z` | coFloat |  | advanced | 1.0 |  |
| `sla_material_settings_id` | coString |  | simple |  |  |
| `sla_print_settings_id` | coString |  | simple |  |  |
| `slow_tilt_time` | coFloat |  | advanced | 8.0 |  |
| `support_base_diameter` | coFloat |  | advanced | 4.0 |  |
| `support_base_height` | coFloat |  | advanced | 1.0 |  |
| `support_base_safety_distance` | coFloat |  | advanced | 1 |  |
| `support_buildplate_only` | coBool |  | simple | False |  |
| `support_critical_angle` | coFloat |  | advanced | 45 |  |
| `support_head_front_diameter` | coFloat |  | advanced | 0.4 |  |
| `support_head_penetration` | coFloat |  | advanced | 0.2 |  |
| `support_head_width` | coFloat |  | advanced | 1.0 |  |
| `support_max_bridge_length` | coFloat |  | advanced | 15.0 |  |
| `support_max_bridges_on_pillar` | coInt |  | advanced | 3 |  |
| `support_max_pillar_link_distance` | coFloat |  | advanced | 10.0 |  |
| `support_object_elevation` | coFloat |  | advanced | 5.0 |  |
| `support_pillar_connection_mode` | coEnum |  | advanced | dynamic |  |
| `support_pillar_diameter` | coFloat |  | simple | 1.0 |  |
| `support_pillar_widening_factor` | coFloat |  | advanced | 0.0 |  |
| `support_points_density_relative` | coInt |  | simple | 100 |  |
| `support_points_minimal_distance` | coFloat |  | simple | 1.0 |  |
| `support_small_pillar_diameter_percent` | coPercent |  | advanced | 50 |  |
| `supports_enable` | coBool |  | simple | True |  |

## Groupe `cli:CLIActions` (20 paramètres)

| Clé | Type | Catégorie | Mode | Défaut | Libellé |
|---|---|---|---|---|---|
| `export_3mf` | coString |  | simple | output.3mf | Export 3MF |
| `export_settings` | coString |  | simple | output.json | Export Settings |
| `export_slicedata` | coString |  | simple | cached_data | Export slicing data |
| `export_stl` | coBool |  | simple | False | Export STL |
| `export_stls` | coString |  | simple | stl_path | Export multiple STLs |
| `help` | coBool |  | simple | False | Help |
| `info` | coBool |  | simple | False | Output Model Info |
| `inspect_mesh` | coBool |  | simple | False | Inspect mesh (JSON to stdout) |
| `inspect_paint` | coBool |  | simple | False | Inspect paint (JSON to stdout) |
| `load_defaultfila` | coBool |  | simple | False | Load default filaments |
| `load_slicedata` | coStrings |  | simple | cached_data | Load slicing data |
| `min_save` | coBool |  | simple | False | Minimum save |
| `mstpp` | coInt |  | simple | 300 | mstpp |
| `mtcpp` | coInt |  | simple | 1000000 | mtcpp |
| `no_check` | coBool |  | simple | False | No check |
| `normative_check` | coBool |  | simple | True | Normative check |
| `pipe` | coString |  | simple |  | Send progress to pipe |
| `slice` | coInt |  | simple | 0 | Slice |
| `strict` | coBool |  | simple | False | Strict mode |
| `uptodate` | coBool |  | simple | False | UpToDate |

## Groupe `cli:CLIMisc` (28 paramètres)

| Clé | Type | Catégorie | Mode | Défaut | Libellé |
|---|---|---|---|---|---|
| `align_to_y_axis` | coBool |  | simple | False | Align to Y axis when arranging |
| `allow_mix_temp` | coBool |  | simple | False | Allow filaments with high/low temperature to be printed together |
| `allow_multicolor_oneplate` | coBool |  | simple | True | Allow multiple colors on one plate |
| `allow_newer_file` | coBool |  | simple | False | Allow 3MF with newer version to be sliced |
| `allow_rotations` | coBool |  | simple | False | Allow rotation when arranging |
| `avoid_extrusion_cali_region` | coBool |  | simple | False | Avoid extrusion calibrate region when arranging |
| `clone_objects` | coInts |  | simple |  | Clone Objects |
| `datadir` | coString |  | simple |  | Data directory |
| `debug` | coInt |  | simple | 1 | Debug level |
| `downward_check` | coBool |  | simple | False | Downward machines check |
| `downward_settings` | coStrings |  | simple |  | Downward machines settings |
| `enable_timelapse` | coBool |  | simple | False | Enable timelapse for print |
| `export_statistics` | coBool |  | simple | False | Export statistics |
| `load_assemble_list` | coString |  | simple |  | Load assemble list |
| `load_custom_gcodes` | coString |  | simple |  | Load custom G-code |
| `load_filament_ids` | coInts |  | simple |  | Load filament IDs |
| `load_filaments` | coStrings |  | simple |  | Load Filament Settings |
| `load_settings` | coStrings |  | simple |  | Load General Settings |
| `logfile` | coString |  | simple |  | Log file |
| `makerlab_name` | coString |  | simple |  | MakerLab name |
| `makerlab_version` | coString |  | simple |  | MakerLab version |
| `metadata_name` | coStrings |  | simple |  | Metadata name list |
| `metadata_value` | coStrings |  | simple |  | Metadata value list |
| `outputdir` | coString |  | simple |  | Output directory |
| `skip_modified_gcodes` | coBool |  | simple | False | Skip modified G-code in 3MF |
| `skip_objects` | coInts |  | simple |  | Skip Objects |
| `uptodate_filaments` | coStrings |  | simple |  | Load uptodate filament settings when using uptodate |
| `uptodate_settings` | coStrings |  | simple |  | Load uptodate process/machine settings when using uptodate |

## Groupe `cli:CLITransform` (13 paramètres)

| Clé | Type | Catégorie | Mode | Défaut | Libellé |
|---|---|---|---|---|---|
| `arrange` | coInt |  | simple | 0 | Arrange Options |
| `assemble` | coBool |  | simple | False | Assemble |
| `convert_unit` | coBool |  | simple | False | Convert Unit |
| `ensure_on_bed` | coBool |  | simple | False | Ensure on bed |
| `ground_face_normal` | coString |  | simple |  | Ground face by normal |
| `ground_face_point` | coString |  | simple |  | Ground face at point |
| `ground_largest_face` | coBool |  | simple | False | Ground largest face |
| `orient` | coInt |  | simple | 0 | Orient Options |
| `repair` | coBool |  | simple | False | Repair |
| `repetitions` | coInt |  | simple | 1 | Repetition count |
| `rotate` | coFloat |  | simple | 0 | Rotate |
| `rotate_x` | coFloat |  | simple | 0 | Rotate around X |
| `rotate_y` | coFloat |  | simple | 0 | Rotate around Y |

## Groupe `other:CustomGcodeSpecific` (75 paramètres)

| Clé | Type | Catégorie | Mode | Défaut | Libellé |
|---|---|---|---|---|---|
| `clear_to_x0` | coBool |  | simple |  | Clear path to X0 |
| `curr_accumulated_mass` | coFloat |  | simple |  | Current accumulated mass |
| `curr_layer_mass` | coFloat |  | simple |  | Previous layer mass |
| `curr_physical_extruder_id` | coInt |  | simple |  | Current physical extruder ID |
| `curr_y_acceleration_limit` | coFloat |  | simple |  | Current Y acceleration limit |
| `current_extruder_id` | coInt |  | simple |  | Current extruder ID |
| `current_filament_id` | coInt |  | simple |  | Current filament ID |
| `current_hotend` | coInt |  | simple |  | Current hotend |
| `current_nozzle_id` | coInt |  | simple |  | Current nozzle ID |
| `extrusion_role` | coString |  | simple |  | Extrusion role |
| `fan_speed` | coNone |  | simple |  |  |
| `farthest_point_timelapse_enabled` | coBool |  | simple |  | Farthest-point timelapse enabled |
| `filament_extruder_id` | coInt |  | simple |  | Filament extruder ID |
| `first_flush_volume` | coFloat |  | simple |  | First flush volume |
| `flush_length` | coFloat |  | simple |  | Flush length |
| `flush_length_1` | coFloat |  | simple |  | Flush Length 1 |
| `flush_length_2` | coFloat |  | simple |  | Flush Length 2 |
| `flush_length_3` | coFloat |  | simple |  | Flush Length 3 |
| `flush_length_4` | coFloat |  | simple |  | Flush Length 4 |
| `flush_temperatures` | coInts |  | simple |  | Flush temperatures |
| `flush_volumetric_speeds` | coFloats |  | simple |  | Flush volumetric speeds |
| `has_timelapse_safe_pos` | coBool |  | simple |  | Has timelapse safe position |
| `is_prime_tower_interface` | coBool |  | simple |  | Prime tower interface |
| `last_extrusion_role` | coString |  | simple |  | Last extrusion role |
| `layer_num` | coInt |  | simple |  | Layer number |
| `layer_z` | coFloat |  | simple |  | Layer Z |
| `long_retraction_when_cut` | coBool |  | simple |  | Long retraction when cut |
| `max_layer_z` | coFloat |  | simple |  | Maximal layer Z |
| `most_used_physical_extruder_id` | coInt |  | simple |  | Most-used physical extruder ID |
| `new_extruder_retracted_length` | coFloat |  | simple |  | New extruder retracted length |
| `new_extruder_variant` | coString |  | simple |  | New extruder variant |
| `new_filament_e_feedrate` | coInt |  | simple |  | New filament e feedrate |
| `new_filament_temp` | coInt |  | simple |  | New filament temp |
| `new_retract_length` | coFloat |  | simple |  | New retract length |
| `new_retract_length_toolchange` | coFloat |  | simple |  | New retract length toolchange |
| `next_extruder` | coInt |  | simple |  | Next extruder |
| `next_filament_id` | coInt |  | simple |  | Next filament ID |
| `next_hotend` | coInt |  | simple |  | Next hotend |
| `next_nozzle_id` | coInt |  | simple |  | Next nozzle ID |
| `nozzle_diameter_at_nozzle_id` | coFloats |  | simple |  | Nozzle diameters |
| `nozzle_volume_types` | coStrings |  | simple |  | Nozzle volume types |
| `old_extruder_variant` | coString |  | simple |  | Old extruder variant |
| `old_filament_e_feedrate` | coInt |  | simple |  | Old filament e feedrate |
| `old_filament_temp` | coInt |  | simple |  | Old filament temp |
| `old_retract_length` | coFloat |  | simple |  | Old retract length |
| `old_retract_length_toolchange` | coFloat |  | simple |  | Old retract length toolchange |
| `outer_wall_volumetric_speed` | coFloat |  | simple |  | Outer wall volumetric speed |
| `previous_extruder` | coInt |  | simple |  | Previous extruder |
| `previous_layer_mass` | coFloat |  | simple |  | Previous layer mass |
| `relative_e_axis` | coBool |  | simple |  | Relative e-axis |
| `retraction_distance_when_cut` | coFloat |  | simple |  | Retraction distance when cut |
| `second_flush_volume` | coFloat |  | simple |  | Second flush volume |
| `temperature` | coInts |  | simple |  | Temperature |
| `timelapse_inline_photo` | coBool |  | simple |  | Timelapse inline photo |
| `timelapse_pos_x` | coInt |  | simple |  | Timelapse position X |
| `timelapse_pos_y` | coInt |  | simple |  | Timelapse position Y |
| `toolchange_count` | coInt |  | simple |  | Toolchange count |
| `toolchange_z` | coFloat |  | simple |  | Toolchange Z |
| `travel_point_1_x` | coFloat |  | simple |  | Travel point 1 X |
| `travel_point_1_y` | coFloat |  | simple |  | Travel point 1 Y |
| `travel_point_2_x` | coFloat |  | simple |  | Travel point 2 X |
| `travel_point_2_y` | coFloat |  | simple |  | Travel point 2 Y |
| `travel_point_3_x` | coFloat |  | simple |  | Travel point 3 X |
| `travel_point_3_y` | coFloat |  | simple |  | Travel point 3 Y |
| `wipe_avoid_perimeter` | coBool |  | simple |  | Avoid wipe perimeter |
| `wipe_avoid_pos_x` | coFloat |  | simple |  | Wipe tower avoidance X |
| `wipe_tower_center_pos_valid` | coBool |  | simple |  | Wipe tower wait position valid |
| `wipe_tower_center_pos_x` | coFloat |  | simple |  | Wipe tower wait position X |
| `wipe_tower_center_pos_y` | coFloat |  | simple |  | Wipe tower wait position Y |
| `wipe_tower_wait_pos_valid` | coBool |  | simple |  | Wipe tower wait position valid |
| `wipe_tower_wait_pos_x` | coFloat |  | simple |  | Wipe tower wait position X |
| `wipe_tower_wait_pos_y` | coFloat |  | simple |  | Wipe tower wait position Y |
| `x_after_toolchange` | coFloat |  | simple |  | X after toolchange |
| `y_after_toolchange` | coFloat |  | simple |  | Y after toolchange |
| `z_after_toolchange` | coFloat |  | simple |  | Z after toolchange |

## Groupe `other:Dimensions` (9 paramètres)

| Clé | Type | Catégorie | Mode | Défaut | Libellé |
|---|---|---|---|---|---|
| `first_layer_center_no_wipe_tower` | coFloats |  | simple |  | First layer center without wipe tower |
| `first_layer_height` | coFloat |  | simple |  | First layer height |
| `first_layer_print_convex_hull` | coPoints |  | simple |  | First layer convex hull |
| `first_layer_print_max` | coFloats |  | simple |  | Top-right corner of the first layer bounding box |
| `first_layer_print_min` | coFloats |  | simple |  | Bottom-left corner of the first layer bounding box |
| `first_layer_print_size` | coFloats |  | simple |  | Size of the first layer bounding box |
| `print_bed_max` | coFloats |  | simple |  | Top-right corner of print bed bounding box |
| `print_bed_min` | coFloats |  | simple |  | Bottom-left corner of print bed bounding box |
| `print_bed_size` | coFloats |  | simple |  | Size of the print bed bounding box |

## Groupe `other:ObjectsInfo` (6 paramètres)

| Clé | Type | Catégorie | Mode | Défaut | Libellé |
|---|---|---|---|---|---|
| `input_filename` | coString |  | simple |  | Full input filename |
| `input_filename_base` | coString |  | simple |  | Input filename without extension |
| `num_instances` | coInt |  | simple |  | Number of instances |
| `num_objects` | coInt |  | simple |  | Number of objects |
| `plate_name` | coString |  | simple |  | Plate name |
| `scale` | coStrings |  | simple |  | Scale per object |

## Groupe `other:OtherPresets` (4 paramètres)

| Clé | Type | Catégorie | Mode | Défaut | Libellé |
|---|---|---|---|---|---|
| `filament_preset` | coString |  | simple |  | Filament preset name |
| `physical_printer_preset` | coString |  | simple |  | Physical printer name |
| `print_preset` | coString |  | simple |  | Print preset name |
| `printer_preset` | coString |  | simple |  | Printer preset name |

## Groupe `other:OtherSlicingStates` (13 paramètres)

| Clé | Type | Catégorie | Mode | Défaut | Libellé |
|---|---|---|---|---|---|
| `current_extruder` | coInt |  | simple |  | Current extruder |
| `current_object_idx` | coInt |  | simple |  | Current object index |
| `has_single_extruder_multi_material_priming` | coBool |  | simple |  | Has single extruder MM priming |
| `has_wipe_tower` | coBool |  | simple |  | Has wipe tower |
| `imex_mode` | coString |  | simple |  | IDEX/IQEX active mode |
| `imex_mode_gcode` | coString |  | simple |  | IDEX/IQEX active mode G-code |
| `imex_mode_index` | coInt |  | simple |  | IDEX/IQEX active mode index |
| `in_head_wrap_detect_zone` | coBool |  | simple |  | In head wrap detect zone |
| `initial_extruder` | coInt |  | simple |  | Initial extruder |
| `initial_no_support_extruder` | coInt |  | simple |  | Initial no support extruder |
| `initial_tool` | coInt |  | simple |  | Initial tool |
| `is_extruder_used` | coBools |  | simple |  | Is extruder used? |
| `num_extruders` | coInt |  | simple |  | Number of extruders |

## Groupe `other:PrintStatistics` (21 paramètres)

| Clé | Type | Catégorie | Mode | Défaut | Libellé |
|---|---|---|---|---|---|
| `extruded_volume` | coFloats |  | simple |  | Volume per extruder |
| `extruded_volume_total` | coFloat |  | simple |  | Total volume |
| `extruded_weight` | coFloats |  | simple |  | Weight per extruder |
| `extruded_weight_total` | coFloat |  | simple |  | Total weight |
| `normal_print_time` | coString |  | simple |  | Print time (normal mode) |
| `print_time` | coString |  | simple |  | Print time (normal mode) |
| `print_time_day` | coString |  | simple |  | Print time (days component) |
| `print_time_hour` | coString |  | simple |  | Print time (hours component) |
| `print_time_minute` | coString |  | simple |  | Print time (minutes component) |
| `print_time_sec` | coString |  | simple |  | Print time (seconds component) |
| `print_time_total_sec` | coString |  | simple |  | Print time (total seconds) |
| `printing_filament_types` | coString |  | simple |  | Used filament types |
| `silent_print_time` | coString |  | simple |  | Print time (silent mode) |
| `total_cost` | coFloat |  | simple |  | Total cost |
| `total_layer_count` | coInt |  | simple |  | Total layer count |
| `total_toolchanges` | coInt |  | simple |  | Total tool changes |
| `total_weight` | coFloat |  | simple |  | Total weight |
| `total_wipe_tower_cost` | coFloat |  | simple |  | Total wipe tower cost |
| `total_wipe_tower_filament` | coFloat |  | simple |  | Total wipe tower filament |
| `used_filament` | coFloat |  | simple |  | Used filament |
| `used_filament_length` | coString |  | simple |  | Filament length (meters) |

## Groupe `other:ReadOnlySlicingStates` (1 paramètres)

| Clé | Type | Catégorie | Mode | Défaut | Libellé |
|---|---|---|---|---|---|
| `zhop` | coFloat |  | simple |  | Current Z-hop |

## Groupe `other:ReadWriteSlicingStates` (4 paramètres)

| Clé | Type | Catégorie | Mode | Défaut | Libellé |
|---|---|---|---|---|---|
| `e_position` | coFloats |  | simple |  | Absolute E position |
| `e_restart_extra` | coFloats |  | simple |  | Extra de-retraction |
| `e_retracted` | coFloats |  | simple |  | Retraction |
| `position` | coFloats |  | simple |  | Position |

## Groupe `other:Temperatures` (6 paramètres)

| Clé | Type | Catégorie | Mode | Défaut | Libellé |
|---|---|---|---|---|---|
| `bed_temperature` | coInts |  | simple |  | Bed temperature |
| `bed_temperature_initial_layer` | coInts |  | simple |  | First layer bed temperature |
| `bed_temperature_initial_layer_single` | coInt |  | simple |  | First layer bed temperature (initial extruder) |
| `first_layer_bed_temperature` | coInts |  | simple |  | First layer bed temperature |
| `first_layer_temperature` | coInts |  | simple |  | First layer temperature |
| `overall_chamber_temperature` | coInt |  | simple |  | Overall chamber temperature |

## Groupe `other:Timestamps` (7 paramètres)

| Clé | Type | Catégorie | Mode | Défaut | Libellé |
|---|---|---|---|---|---|
| `day` | coInt |  | simple |  | Day |
| `hour` | coInt |  | simple |  | Hour |
| `minute` | coInt |  | simple |  | Minute |
| `month` | coInt |  | simple |  | Month |
| `second` | coInt |  | simple |  | Second |
| `timestamp` | coString |  | simple |  | Timestamp |
| `year` | coInt |  | simple |  | Year |
