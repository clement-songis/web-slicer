# Annexe A — Registre exhaustif des paramètres (normatif)

Source : `audit/parameters.json` (846 paramètres, généré depuis
`vendor/OrcaSlicer/src/libslic3r/PrintConfig.cpp`). Chaque ligne est une
exigence : le paramètre DOIT être exposé par l'application (stockage,
API, UI selon son groupe) ou inscrit au registre d'exclusions avec
justification.

Portée par groupe : `fff`/`common` → exposés dans l'UI de réglages ;
`sla` → registre + API (pas d'UI, OrcaSlicer n'expose pas d'onglet SLA) ;
`cli:*` → équivalents des actions serveur (tranchage, transformations) ;
`other:*` → placeholders G-code et états de slicing (moteur de templates).

## Groupe `common` (31 paramètres)

| Clé | Type | Catégorie | Mode | Défaut | Libellé |
|---|---|---|---|---|---|
| `bbl_use_printhost` | coBool |  | advanced | False | Use 3rd-party print host |
| `bed_custom_model` | coString |  | advanced |  | Bed custom model |
| `bed_custom_texture` | coString |  | advanced |  | Bed custom texture |
| `bed_exclude_area` | coPoints |  | advanced | 0, 0 | Bed exclude area |
| `elefant_foot_compensation` | coFloat | Quality | advanced | 0.0 | Elephant foot compensation |
| `elefant_foot_compensation_layers` | coInt | Quality | advanced | 1 | Elephant foot compensation layers |
| `elefant_foot_layers_density` | coPercent | Quality | expert | 100 | Elephant foot layers density |
| `extruder_printable_area` | coPointsGroups |  | advanced |  | Extruder printable area |
| `extruder_printable_height` | coFloats |  | advanced |  | Extruder printable height |
| `flashforge_serial_number` | coString |  | advanced |  | Serial Number |
| `layer_height` | coFloat | Quality | simple | 0.2 | Layer height |
| `parallel_printheads_bed_exclude_areas` | coStrings |  | advanced |  | Parallel printheads bed exclude areas |
| `parallel_printheads_count` | coInt |  | advanced |  | Parallel printheads count |
| `preferred_orientation` | coFloat |  | advanced | 0.0 | Preferred orientation |
| `preset_name` | coString |  | simple |  |  |
| `preset_names` | coStrings |  | advanced |  | Printer preset names |
| `print_host` | coString |  | advanced |  | Hostname, IP or URL |
| `print_host_webui` | coString |  | advanced |  | Device UI |
| `printable_area` | coPoints |  | advanced | 0, 0 | Printable area |
| `printable_height` | coFloat |  | simple | 100.0 | Printable height |
| `printer_agent` | coString |  | advanced |  | Printer Agent |
| `printer_technology` | coEnum |  | simple | ptFFF | Printer technology |
| `printhost_apikey` | coString |  | advanced |  | API Key / Password |
| `printhost_authorization_type` | coEnum |  | advanced | atKeyPassword | Authorization Type |
| `printhost_cafile` | coString |  | advanced |  | HTTPS CA File |
| `printhost_password` | coString |  | advanced |  | Password |
| `printhost_port` | coString |  | advanced |  | Printer |
| `printhost_ssl_ignore_revoke` | coBool |  | advanced | False | Ignore HTTPS certificate revocation checks |
| `printhost_user` | coString |  | advanced |  | User |
| `support_parallel_printheads` | coBool |  | advanced |  | Support parallel printheads |
| `use_3mf` | coBool |  | advanced | False | Use 3MF instead of G-code |

## Groupe `fff` (632 paramètres)

| Clé | Type | Catégorie | Mode | Défaut | Libellé |
|---|---|---|---|---|---|
| `accel_to_decel_enable` | coBool | Speed | advanced | True | Enable accel_to_decel |
| `accel_to_decel_factor` | coPercent | Speed | advanced | 50 | accel_to_decel |
| `activate_air_filtration` | coBools |  | simple |  | Activate air filtration |
| `activate_air_filtration_during_print` | coBools |  | simple |  |  |
| `activate_air_filtration_on_completion` | coBools |  | simple |  |  |
| `activate_chamber_temp_control` | coBools |  | simple |  | Activate temperature control |
| `adaptive_bed_mesh_margin` | coFloat |  | advanced | 0 | Mesh margin |
| `adaptive_pressure_advance` | coBools |  | advanced |  | Enable adaptive pressure advance (beta) |
| `adaptive_pressure_advance_bridges` | coFloats |  | advanced |  | Pressure advance for bridges |
| `adaptive_pressure_advance_model` | coStrings |  | advanced |  | Adaptive pressure advance measurements (beta) |
| `adaptive_pressure_advance_overhangs` | coBools |  | advanced |  | Enable adaptive pressure advance for overhangs (beta) |
| `additional_cooling_fan_speed` | coInts |  | simple |  | Fan speed |
| `additional_fan_full_speed_layer` | coInts |  | simple |  | Full fan speed at layer |
| `align_infill_direction_to_model` | coBool | Strength | advanced | False | Align infill direction to model |
| `alternate_extra_wall` | coBool | Strength | advanced | False | Alternate extra wall |
| `auxiliary_fan` | coBool |  | advanced | False | Auxiliary part cooling fan |
| `bbl_calib_mark_logo` | coBool |  | advanced | True | Show auto-calibration marks |
| `bed_mesh_max` | coPoint |  | advanced | Vec2d(99999, 99999) | Bed mesh max |
| `bed_mesh_min` | coPoint |  | advanced | Vec2d(-99999, -99999) | Bed mesh min |
| `bed_mesh_probe_distance` | coPoint |  | advanced | Vec2d(50, 50) | Probe point distance |
| `bed_temperature_formula` | coEnum |  | advanced | BedTempFormula::btfHighestTemp | Bed temperature type |
| `before_layer_change_gcode` | coString |  | advanced |  | Before layer change G-code |
| `best_object_pos` | coPoint |  | advanced | Vec2d(0.5, 0.5) | Best object position |
| `bottom_shell_layers` | coInt | Strength | simple | 3 | Bottom shell layers |
| `bottom_shell_thickness` | coFloat | Strength | simple | 0.0 | Bottom shell thickness |
| `bottom_solid_infill_flow_ratio` | coFloat | Advanced | advanced | 1 | Bottom surface flow ratio |
| `bottom_surface_density` | coPercent | Strength | simple | 100 | Bottom surface density |
| `bottom_surface_filament_id` | coInt | Extruders | advanced | 0 | Bottom surface |
| `bottom_surface_pattern` | coEnum | Strength | simple | ipMonotonic | Bottom surface pattern |
| `bridge_acceleration` | coFloatOrPercent | Speed | advanced | {"value": 50.0, "percent": true} | Bridge |
| `bridge_angle` | coFloat | Strength | advanced | 0.0 | External bridge infill direction |
| `bridge_density` | coPercent | Strength | advanced | 100 | External bridge density |
| `bridge_flow` | coFloat | Quality | advanced | 1 | Bridge flow ratio |
| `bridge_line_width` | coFloatOrPercent | Quality | advanced | {"value": 100.0, "percent": true} | Bridge |
| `bridge_no_support` | coBool | Support | advanced | False | Don't support bridges |
| `bridge_speed` | coFloat | Speed | advanced | 25 | External |
| `brim_ears` | coBool | Support | advanced | False | Brim ears |
| `brim_ears_detection_length` | coFloat | Support | advanced | 1 | Brim ear detection radius |
| `brim_ears_max_angle` | coFloat | Support | advanced | 125 | Brim ear max angle |
| `brim_flow_ratio` | coFloat | Support | advanced | 1 | Brim flow ratio |
| `brim_object_gap` | coFloat | Support | advanced | 0.0 | Brim-object gap |
| `brim_type` | coEnum | Support | simple | btAutoBrim | Brim type |
| `brim_use_efc_outline` | coBool | Support | advanced | False | Brim follows compensated outline |
| `brim_width` | coFloat | Support | simple | 0.0 | Brim width |
| `calib_flowrate_topinfill_special_order` | coBool |  | develop | False |  |
| `chamber_minimal_temperature` | coInts |  | simple |  | Minimal |
| `chamber_temperature` | coInts |  | simple |  | Chamber temperature |
| `change_extrusion_role_gcode` | coString |  | advanced |  | Change extrusion role G-code |
| `change_filament_gcode` | coString |  | advanced |  | Change filament G-code |
| `close_additional_fan_first_x_layers` | coInts |  | simple |  | For the first |
| `close_fan_the_first_x_layers` | coInts |  | simple |  | No cooling for the first |
| `combine_brims` | coBool | Support | advanced | False | Combine brims |
| `compatible_machine_expression_group` | coStrings |  | simple |  |  |
| `compatible_printers` | coStrings |  | advanced |  | Select printers |
| `compatible_printers_condition` | coString |  | advanced |  | Condition |
| `compatible_prints` | coStrings |  | advanced |  | Select profiles |
| `compatible_prints_condition` | coString |  | advanced |  | Condition |
| `compatible_process_expression_group` | coStrings |  | simple |  |  |
| `complete_print_exhaust_fan_speed` | coInts |  | simple |  | Fan speed |
| `cool_plate_temp` | coInts |  | simple |  | Other layers |
| `cool_plate_temp_initial_layer` | coInts |  | simple |  | First layer |
| `cooling_tube_length` | coFloat |  | advanced | 5.0 | Cooling tube length |
| `cooling_tube_retraction` | coFloat |  | advanced | 91.5 | Cooling tube position |
| `counterbore_hole_bridging` | coEnum | Quality | advanced | chbNone | Bridge counterbore holes |
| `curr_bed_type` | coEnum |  | simple | btPC | Bed type |
| `default_acceleration` | coFloat | Speed | advanced | 500.0 | Normal printing |
| `default_bed_type` | coString |  | advanced |  | Default bed type |
| `default_filament_colour` | coStrings |  | advanced |  | Default color |
| `default_filament_profile` | coStrings |  | simple |  | Default filament profile |
| `default_jerk` | coFloat | Speed | advanced | 0 | Default |
| `default_junction_deviation` | coFloat | Speed | advanced | 0.f | Junction Deviation |
| `default_nozzle_volume_type` | coEnums |  | develop |  | Default Nozzle Volume Type. |
| `default_print_profile` | coString |  | simple |  | Default process profile |
| `deretraction_speed` | coFloats |  | advanced |  | De-retraction Speed |
| `detect_narrow_internal_solid_infill` | coBool | Strength | advanced | True | Detect narrow internal solid infills |
| `detect_overhang_wall` | coBool | Quality | advanced | True | Detect overhang walls |
| `detect_thin_wall` | coBool | Strength | advanced | False | Detect thin walls |
| `different_settings_to_system` | coStrings |  | simple |  |  |
| `disable_m73` | coBool |  | advanced | False | Disable set remaining print time |
| `dont_filter_internal_bridges` | coEnum | Quality | advanced | ibfDisabled | Filter out small internal bridges |
| `dont_slow_down_outer_wall` | coBools |  | simple |  | Don't slow down outer walls |
| `draft_shield` | coEnum |  | advanced | dsDisabled | Draft shield |
| `during_print_exhaust_fan_speed` | coInts |  | simple |  | Fan speed |
| `emit_machine_limits_to_gcode` | coBool | Machine limits | advanced | True | Emit limits to G-code |
| `enable_arc_fitting` | coBool |  | advanced | 0 | Arc fitting |
| `enable_extra_bridge_layer` | coEnum | Quality | advanced | eblDisabled | Extra bridge layers (beta) |
| `enable_filament_dynamic_map` | coBool |  | develop | False | Enable filament dynamic map |
| `enable_filament_ramming` | coBool |  | advanced | True | Enable filament ramming |
| `enable_long_retraction_when_cut` | coInt |  | develop |  |  |
| `enable_overhang_bridge_fan` | coBools |  | simple |  | Force cooling for overhangs and bridges |
| `enable_overhang_speed` | coBool | Speed | advanced |  | Slow down for overhang |
| `enable_power_loss_recovery` | coEnum |  | advanced | PowerLossRecoveryMode::PrinterConfiguration | Power Loss Recovery |
| `enable_pressure_advance` | coBools |  | simple |  | Enable pressure advance |
| `enable_prime_tower` | coBool |  | simple | False | Enable |
| `enable_support` | coBool | Support | simple | False | Enable support |
| `enable_tower_interface_cooldown_during_tower` | coBool |  | advanced | False | Cool down from interface boost during prime tower |
| `enable_tower_interface_features` | coBool |  | advanced | False | Enable tower interface features |
| `enable_wrapping_detection` | coBool |  | advanced | False | Enable clumping detection |
| `enforce_support_layers` | coInt | Support | develop | 0 |  |
| `eng_plate_temp` | coInts |  | simple |  | Other layers |
| `eng_plate_temp_initial_layer` | coInts |  | simple |  | First layer |
| `ensure_vertical_shell_thickness` | coEnum | Strength | advanced | EnsureVerticalShellThickness::evstAll | Ensure vertical shell thickness |
| `exclude_object` | coBool |  | advanced | False | Exclude objects |
| `extra_loading_move` | coFloat |  | advanced | -2.0 | Extra loading distance |
| `extra_perimeters_on_overhangs` | coBool | Quality | advanced | False | Extra perimeters on overhangs |
| `extra_solid_infills` | coString | Strength | advanced |  | Insert solid layers |
| `extruder` | coInt | Extruders | advanced |  | Extruder |
| `extruder_ams_count` | coStrings |  | simple |  | Extruder AMS count |
| `extruder_clearance_height_to_lid` | coFloat |  | advanced | 120 | Height to lid |
| `extruder_clearance_height_to_rod` | coFloat |  | advanced | 40 | Height to rod |
| `extruder_clearance_radius` | coFloat |  | advanced | 40 | Radius |
| `extruder_colour` | coStrings |  | advanced |  | Extruder Color |
| `extruder_offset` | coPoints |  | advanced | 0,0 | Extruder offset |
| `extruder_type` | coEnums |  | advanced |  | Type |
| `extruder_variant_list` | coStrings |  | simple |  | Extruder variant list |
| `extrusion_rate_smoothing_external_perimeter_only` | coBool |  | advanced | False | Apply only on external features |
| `fan_cooling_layer_time` | coFloats |  | simple |  | Layer time |
| `fan_kickstart` | coFloat |  | advanced | 0 | Fan kick-start time |
| `fan_max_speed` | coFloats |  | simple |  | Fan speed |
| `fan_min_speed` | coFloats |  | simple |  | Fan speed |
| `fan_speedup_overhangs` | coBool |  | advanced | True | Only overhangs |
| `fan_speedup_time` | coFloat |  | advanced | 0 |  |
| `filament_adaptive_volumetric_speed` | coBools |  | develop |  | Adaptive volumetric speed |
| `filament_adhesiveness_category` | coInts |  | develop |  | Adhesiveness Category |
| `filament_change_extrusion_role_gcode` | coStrings |  | advanced |  | Change extrusion role G-code (filament) |
| `filament_change_length` | coFloats |  | advanced |  | Filament ramming length |
| `filament_colour` | coStrings |  | advanced |  | Color |
| `filament_colour_type` | coStrings |  | simple |  |  |
| `filament_cooling_before_tower` | coFloats |  | develop |  | Wipe tower cooling |
| `filament_cooling_final_speed` | coFloats |  | advanced |  | Speed of the last cooling move |
| `filament_cooling_initial_speed` | coFloats |  | advanced |  | Speed of the first cooling move |
| `filament_cooling_moves` | coInts |  | advanced |  | Number of cooling moves |
| `filament_cost` | coFloats |  | advanced |  | Price |
| `filament_density` | coFloats |  | advanced |  | Density |
| `filament_deretraction_speed` | coFloatsNullable |  | advanced |  | De-retraction Speed |
| `filament_diameter` | coFloats |  | simple |  | Diameter |
| `filament_end_gcode` | coStrings |  | advanced |  | End G-code |
| `filament_extruder_variant` | coStrings |  | simple |  | Filament's extruder variant |
| `filament_flow_ratio` | coFloats |  | advanced |  | Flow ratio |
| `filament_flush_temp` | coInts |  | advanced |  | Flush temperature |
| `filament_flush_volumetric_speed` | coFloats |  | advanced |  | Flush volumetric speed |
| `filament_ids` | coStrings |  | simple |  |  |
| `filament_ironing_flow` | coPercents | Quality | advanced |  | Ironing flow |
| `filament_ironing_inset` | coFloats | Quality | advanced |  | Ironing inset |
| `filament_ironing_spacing` | coFloats | Quality | advanced |  | Ironing line spacing |
| `filament_ironing_speed` | coFloats | Speed | advanced |  | Ironing speed |
| `filament_is_support` | coBools |  | advanced |  | Support material |
| `filament_loading_speed` | coFloats |  | advanced |  | Loading speed |
| `filament_loading_speed_start` | coFloats |  | advanced |  | Loading speed at the start |
| `filament_long_retractions_when_cut` | coBoolsNullable |  | develop |  | Long retraction when cut (beta) |
| `filament_map` | coInts |  | develop |  | Filament map to extruder |
| `filament_map_mode` | coEnum |  | advanced | fmmAutoForFlush | filament mapping mode |
| `filament_max_volumetric_speed` | coFloats |  | advanced |  | Max volumetric speed |
| `filament_minimal_purge_on_wipe_tower` | coFloats |  | advanced |  | Minimal purge on wipe tower |
| `filament_multi_colour` | coStrings |  | simple |  |  |
| `filament_multitool_ramming` | coBools |  | advanced |  | Enable ramming for multi-tool setups |
| `filament_multitool_ramming_flow` | coFloats |  | advanced |  | Multi-tool ramming flow |
| `filament_multitool_ramming_volume` | coFloats |  | advanced |  | Multi-tool ramming volume |
| `filament_notes` | coStrings |  | advanced |  | Filament notes |
| `filament_printable` | coInts |  | develop |  | Filament printable |
| `filament_ramming_parameters` | coStrings |  | advanced |  | Ramming parameters |
| `filament_retract_before_wipe` | coPercentsNullable |  | advanced |  | Retract amount before wipe |
| `filament_retract_lift_above` | coFloatsNullable |  | advanced |  | Only lift Z above |
| `filament_retract_lift_below` | coFloatsNullable |  | advanced |  | Only lift Z below |
| `filament_retract_lift_enforce` | coEnumsNullable |  | advanced |  | On surfaces |
| `filament_retract_restart_extra` | coFloatsNullable |  | advanced |  | Extra length on restart |
| `filament_retract_when_changing_layer` | coBoolsNullable |  | advanced |  | Retract when change layer |
| `filament_retraction_distances_when_cut` | coFloatsNullable |  | develop |  | Retraction distance when cut |
| `filament_retraction_length` | coFloatsNullable |  | simple |  | Length |
| `filament_retraction_minimum_travel` | coFloatsNullable |  | advanced |  | Travel distance threshold |
| `filament_retraction_speed` | coFloatsNullable |  | advanced |  | Retraction Speed |
| `filament_self_index` | coInts |  | simple |  | Filament self index |
| `filament_settings_id` | coStrings |  | simple |  |  |
| `filament_shrink` | coPercents |  | advanced |  | Shrinkage (XY) |
| `filament_shrinkage_compensation_z` | coPercents |  | advanced |  | Shrinkage (Z) |
| `filament_soluble` | coBools |  | advanced |  | Soluble material |
| `filament_stamping_distance` | coFloats |  | advanced |  | Stamping distance measured from the center of the cooling tube |
| `filament_stamping_loading_speed` | coFloats |  | advanced |  | Stamping loading speed |
| `filament_start_gcode` | coStrings |  | advanced |  | Start G-code |
| `filament_toolchange_delay` | coFloats |  | advanced |  | Delay after unloading |
| `filament_tower_interface_pre_extrusion_dist` | coFloats |  | advanced |  | Interface layer pre-extrusion distance |
| `filament_tower_interface_pre_extrusion_length` | coFloats |  | advanced |  | Interface layer pre-extrusion length |
| `filament_tower_interface_print_temp` | coInts |  | advanced |  | Interface layer print temperature |
| `filament_tower_interface_purge_volume` | coFloats |  | advanced |  | Interface layer purge length |
| `filament_tower_ironing_area` | coFloats |  | advanced |  | Tower ironing area |
| `filament_type` | coStrings |  | simple |  | Type |
| `filament_unloading_speed` | coFloats |  | advanced |  | Unloading speed |
| `filament_unloading_speed_start` | coFloats |  | advanced |  | Unloading speed at the start |
| `filament_vendor` | coStrings |  | advanced | ["(Undefined)"] | Vendor |
| `filament_wipe` | coBoolsNullable |  | advanced |  | Wipe while retracting |
| `filament_wipe_distance` | coFloatsNullable |  | advanced |  | Wipe Distance |
| `filament_z_hop` | coFloatsNullable |  | simple |  | Z-hop height |
| `filament_z_hop_types` | coEnumsNullable |  | advanced |  | Z-hop type |
| `file_start_gcode` | coString |  | advanced |  | File header G-code |
| `filename_format` | coString |  | advanced | {input_filename_base}_{filament_type[initial_tool]}_{print_time}.gcode | Filename format |
| `fill_multiline` | coInt | Strength | simple | 1 | Fill Multiline |
| `filter_out_gap_fill` | coFloat | Layers and Perimeters | advanced | 0 | Filter out tiny gaps |
| `first_layer_flow_ratio` | coFloat | Advanced | advanced | 1 | First layer flow ratio |
| `first_layer_print_sequence` | coInts |  | simple |  | First layer print sequence |
| `first_layer_sequence_choice` | coEnum | Quality | simple | flsAuto | First layer filament sequence |
| `first_x_layer_fan_speed` | coFloats |  | simple |  | Fan speed |
| `flush_into_infill` | coBool | Flush options | simple | False | Flush into objects' infill |
| `flush_into_objects` | coBool | Flush options | simple | False | Flush into this object |
| `flush_into_support` | coBool | Flush options | simple | True | Flush into objects' support |
| `flush_multiplier` | coFloats |  | simple |  | Flush multiplier |
| `flush_volumes_matrix` | coFloats |  | simple |  | Purging volumes |
| `flush_volumes_vector` | coFloats |  | simple |  | Purging volumes - load/unload volumes |
| `full_fan_speed_layer` | coInts |  | advanced |  | Full fan speed at layer |
| `fuzzy_skin` | coEnum | Others | simple | FuzzySkinType::Disabled_fuzzy | Fuzzy Skin |
| `fuzzy_skin_first_layer` | coBool | Others | simple | 0 | Apply fuzzy skin to first layer |
| `fuzzy_skin_layers_between_ripple_offset` | coInt | Others | advanced | 1 | Layers between ripple offset |
| `fuzzy_skin_mode` | coEnum | Others | simple | FuzzySkinMode::Displacement | Fuzzy skin generator mode |
| `fuzzy_skin_noise_type` | coEnum | Others | simple | NoiseType::Classic | Fuzzy skin noise type |
| `fuzzy_skin_octaves` | coInt | Others | advanced | 4 | Fuzzy Skin Noise Octaves |
| `fuzzy_skin_persistence` | coFloat | Others | advanced | 0.5 | Fuzzy skin noise persistence |
| `fuzzy_skin_point_distance` | coFloat | Others | simple | 0.3f | Fuzzy skin point distance |
| `fuzzy_skin_ripple_offset` | coPercent | Others | advanced | 50 | Ripple offset |
| `fuzzy_skin_ripples_per_layer` | coInt | Others | advanced | 15 | Number of ripples per layer |
| `fuzzy_skin_scale` | coFloat | Others | advanced | 1.0 | Fuzzy skin feature size |
| `fuzzy_skin_thickness` | coFloat | Others | simple | 0.2 | Fuzzy skin thickness |
| `gap_fill_flow_ratio` | coFloat | Advanced | advanced | 1 | Gap fill flow ratio |
| `gap_fill_target` | coEnum | Strength | advanced | gftNowhere | Apply gap fill |
| `gap_infill_speed` | coFloat | Speed | advanced | 30 | Gap infill |
| `gcode_add_line_number` | coBool |  | develop | 0 | Add line number |
| `gcode_comments` | coBool |  | advanced | 0 | Verbose G-code |
| `gcode_flavor` | coEnum |  | advanced | gcfMarlinLegacy | G-code flavor |
| `gcode_label_objects` | coBool |  | advanced | 1 | Label objects |
| `grab_length` | coFloats |  | develop | [0] | Grab length |
| `gyroid_optimized` | coBool | Strength | simple | False | Z-buckling bias optimization (experimental) |
| `has_filament_switcher` | coBool |  | develop | False | Has filament switcher |
| `has_scarf_joint_seam` | coBool | Machine limits | advanced | False |  |
| `head_wrap_detect_zone` | coPoints |  | develop |  | Head wrap detect zone |
| `high_current_on_filament_swap` | coBool |  | advanced | 0 | High extruder current on filament swap |
| `hole_to_polyhole` | coBool | Quality | advanced | False | Convert holes to polyholes |
| `hole_to_polyhole_threshold` | coFloatOrPercent | Quality | advanced | {"value": 0.01, "percent": false} | Polyhole detection margin |
| `hole_to_polyhole_twisted` | coBool | Quality | advanced | True | Polyhole twist |
| `host_type` | coEnum |  | advanced | htOctoPrint | Host Type |
| `hot_plate_temp` | coInts |  | simple |  | Other layers |
| `hot_plate_temp_initial_layer` | coInts |  | simple |  | First layer |
| `idle_temperature` | coInts |  | simple |  | Idle temperature |
| `independent_support_layer_height` | coBool | Support | advanced | True | Independent support layer height |
| `infill_anchor` | coFloatOrPercent | Strength | advanced | {"value": 400.0, "percent": true} | Sparse infill anchor length |
| `infill_anchor_max` | coFloatOrPercent | Strength | advanced | {"value": 20.0, "percent": false} | Maximum length of the infill anchor |
| `infill_combination` | coBool | Strength | advanced | False | Infill combination |
| `infill_combination_max_layer_height` | coFloatOrPercent | Strength | advanced | {"value": 100.0, "percent": true} | Infill combination - Max layer height |
| `infill_direction` | coFloat | Strength | advanced | 45 | Sparse infill direction |
| `infill_jerk` | coFloat | Speed | advanced | 9 | Infill |
| `infill_lock_depth` | coFloat | Strength | advanced | 1.0 | Infill lock depth |
| `infill_overhang_angle` | coFloat | Strength | advanced | 60 | Infill overhang angle |
| `infill_shift_step` | coFloat | Strength | advanced | 0.4 | Infill shift step |
| `infill_wall_overlap` | coPercent | Strength | advanced | 15 | Infill/Wall overlap |
| `inherits` | coString |  | simple |  | Inherits profile |
| `inherits_group` | coStrings |  | simple |  |  |
| `initial_layer_acceleration` | coFloat | Speed | advanced | 300 | First layer |
| `initial_layer_infill_speed` | coFloat |  | advanced | 60.0 | First layer infill |
| `initial_layer_jerk` | coFloat | Speed | advanced | 9 | First layer |
| `initial_layer_line_width` | coFloatOrPercent | Quality | advanced | {"value": 0.0, "percent": false} | First layer |
| `initial_layer_min_bead_width` | coPercent | Quality | advanced | 85 | First layer minimum wall width |
| `initial_layer_print_height` | coFloat | Quality | simple | 0.2 | First layer height |
| `initial_layer_speed` | coFloat |  | advanced | 30 | First layer |
| `initial_layer_travel_acceleration` | coFloatOrPercent |  | advanced | {"value": 100.0, "percent": true} | First layer travel |
| `initial_layer_travel_jerk` | coFloatOrPercent |  | advanced | {"value": 100.0, "percent": true} | First layer travel |
| `initial_layer_travel_speed` | coFloatOrPercent | Speed | advanced | {"value": 100.0, "percent": true} | First layer travel speed |
| `inner_wall_acceleration` | coFloat | Speed | advanced | 10000 | Inner wall |
| `inner_wall_filament_id` | coInt | Extruders | advanced | 0 | Inner walls |
| `inner_wall_flow_ratio` | coFloat | Advanced | advanced | 1 | Inner wall flow ratio |
| `inner_wall_jerk` | coFloat | Speed | advanced | 9 | Inner wall |
| `inner_wall_line_width` | coFloatOrPercent | Quality | advanced | {"value": 0.0, "percent": false} | Inner wall |
| `inner_wall_speed` | coFloat | Speed | advanced | 60 | Inner wall |
| `input_shaping_damp_x` | coFloat |  | expert | 0.1 | X |
| `input_shaping_damp_y` | coFloat |  | expert | 0.1 | Y |
| `input_shaping_emit` | coBool |  | expert | False | Emit input shaping |
| `input_shaping_freq_x` | coFloat |  | expert | 0 | X |
| `input_shaping_freq_y` | coFloat |  | expert | 0 | Y |
| `input_shaping_type` | coEnum |  | expert | InputShaperType::Default | Input shaper type |
| `interface_shells` | coBool | Quality | advanced | False | Interface shells |
| `interlocking_beam` | coBool | Advanced | advanced | False | Use beam interlocking |
| `interlocking_beam_layer_count` | coInt | Advanced | advanced | 2 | Interlocking beam layers |
| `interlocking_beam_width` | coFloat | Advanced | advanced | 0.8 | Interlocking beam width |
| `interlocking_boundary_avoidance` | coInt | Advanced | advanced | 2 | Interlocking boundary avoidance |
| `interlocking_depth` | coInt | Advanced | advanced | 2 | Interlocking depth |
| `interlocking_orientation` | coFloat | Advanced | advanced | 22.5 | Interlocking direction |
| `internal_bridge_angle` | coFloat | Strength | advanced | 0.0 | Internal bridge infill direction |
| `internal_bridge_density` | coPercent | Strength | advanced | 100 | Internal bridge density |
| `internal_bridge_fan_speed` | coInts |  | advanced |  | Internal bridges fan speed |
| `internal_bridge_flow` | coFloat | Quality | advanced | 1 | Internal bridge flow ratio |
| `internal_bridge_speed` | coFloatOrPercent | Speed | advanced | {"value": 150.0, "percent": true} | Internal |
| `internal_solid_filament_id` | coInt | Extruders | advanced | 0 | Internal solid infill |
| `internal_solid_infill_acceleration` | coFloatOrPercent | Speed | advanced | {"value": 100.0, "percent": true} | Internal solid infill |
| `internal_solid_infill_flow_ratio` | coFloat | Advanced | advanced | 1 | Internal solid infill flow ratio |
| `internal_solid_infill_line_width` | coFloatOrPercent | Quality | advanced | {"value": 0.0, "percent": false} | Internal solid infill |
| `internal_solid_infill_pattern` | coEnum | Strength | simple | ipMonotonic | Internal solid infill pattern |
| `internal_solid_infill_speed` | coFloat | Speed | advanced | 100 | Internal solid infill |
| `ironing_angle` | coFloat | Quality | advanced | 0 | Ironing angle offset |
| `ironing_angle_fixed` | coBool | Quality | advanced | False | Fixed ironing angle |
| `ironing_expansion` | coFloat | Quality | expert | 0 | Ironing expansion |
| `ironing_fan_speed` | coInts |  | advanced |  | Ironing fan speed |
| `ironing_flow` | coPercent | Quality | advanced | 10 | Ironing flow |
| `ironing_inset` | coFloat | Quality | advanced | 0 | Ironing inset |
| `ironing_pattern` | coEnum | Quality | advanced | ipRectilinear | Ironing Pattern |
| `ironing_spacing` | coFloat | Quality | advanced | 0.1 | Ironing line spacing |
| `ironing_speed` | coFloat | Quality | advanced | 20 | Ironing speed |
| `ironing_type` | coEnum | Quality | advanced | IroningType::NoIroning | Ironing Type |
| `is_infill_first` | coBool | Quality | advanced |  | Print infill first |
| `lateral_lattice_angle_1` | coFloat | Strength | advanced | -45 | Lateral lattice angle 1 |
| `lateral_lattice_angle_2` | coFloat | Strength | advanced | 45 | Lateral lattice angle 2 |
| `layer_change_gcode` | coString |  | advanced |  | Layer change G-code |
| `lightning_overhang_angle` | coFloat | Strength | expert | 45 | Lightning overhang angle |
| `lightning_prune_angle` | coFloat | Strength | expert | 45 | Prune angle |
| `lightning_straightening_angle` | coFloat | Strength | expert | 45 | Straightening angle |
| `line_width` | coFloatOrPercent | Quality | advanced | {"value": 0.0, "percent": false} | Default |
| `long_retractions_when_cut` | coBools |  | develop |  | Long retraction when cut (beta) |
| `long_retractions_when_ec` | coBools |  | advanced |  | Long retraction when extruder change |
| `machine_end_gcode` | coString |  | advanced | M104 S0 ; turn off temperature G28 X0  ; home X axis M84     ; disable motors  | End G-code |
| `machine_load_filament_time` | coFloat |  | advanced | 0.0 | Filament load time |
| `machine_max_acceleration_extruding` | coFloats | Machine limits | simple |  |  |
| `machine_max_acceleration_retracting` | coFloats | Machine limits | simple |  |  |
| `machine_max_acceleration_travel` | coFloats | Machine limits | advanced |  |  |
| `machine_max_junction_deviation` | coFloats | Machine limits | advanced |  |  |
| `machine_min_extruding_rate` | coFloats | Machine limits | develop |  |  |
| `machine_min_travel_rate` | coFloats | Machine limits | develop |  |  |
| `machine_pause_gcode` | coString |  | advanced |  | Pause G-code |
| `machine_start_gcode` | coString |  | advanced | G28 ; home all axes G1 Z5 F5000 ; lift nozzle  | Start G-code |
| `machine_tool_change_time` | coFloat |  | advanced |  | Tool change time |
| `machine_unload_filament_time` | coFloat |  | advanced | 0.0 | Filament unload time |
| `make_overhang_printable` | coBool | Quality | advanced | False | Make overhangs printable |
| `make_overhang_printable_angle` | coFloat | Quality | advanced | 55.0 | Make overhangs printable - Maximum angle |
| `make_overhang_printable_hole_size` | coFloat | Quality | advanced | 0.0 | Make overhangs printable - Hole area |
| `manual_filament_change` | coBool |  | advanced | False | Manual Filament Change |
| `master_extruder_id` | coInt |  | simple |  | Master extruder id |
| `max_bridge_length` | coFloat | Support | advanced | 10 | Max bridge length |
| `max_layer_height` | coFloats |  | advanced |  | Max |
| `max_resonance_avoidance_speed` | coFloat |  | advanced | 120 | Max |
| `max_travel_detour_distance` | coFloatOrPercent | Quality | advanced | {"value": 0.0, "percent": false} | Avoid crossing walls - Max detour length |
| `max_volumetric_extrusion_rate_slope` | coFloat |  | advanced | 0 | Extrusion rate smoothing |
| `max_volumetric_extrusion_rate_slope_segment_length` | coFloat |  | advanced | 3.0 | Smoothing segment length |
| `min_bead_width` | coPercent | Quality | advanced | 85 | Minimum wall width |
| `min_feature_size` | coPercent | Quality | advanced | 25 | Minimum feature size |
| `min_layer_height` | coFloats |  | advanced |  | Min |
| `min_length_factor` | coFloat | Quality | advanced | 0.5 | Minimum wall length |
| `min_resonance_avoidance_speed` | coFloat |  | advanced | 70 | Min |
| `min_skirt_length` | coFloat |  | advanced | 0.0 | Skirt minimum extrusion length |
| `min_width_top_surface` | coFloatOrPercent | Quality | advanced | {"value": 300.0, "percent": true} | One wall threshold |
| `minimum_sparse_infill_area` | coFloat | Strength | advanced | 15 | Minimum sparse infill threshold |
| `mmu_segmented_region_interlocking_depth` | coFloat | Advanced | advanced | 0.0 | Interlocking depth of a segmented region |
| `mmu_segmented_region_max_width` | coFloat | Advanced | advanced | 0.0 | Maximum width of a segmented region |
| `notes` | coString |  | advanced |  | Configuration notes |
| `nozzle_diameter` | coFloats |  | advanced |  | Nozzle diameter |
| `nozzle_flush_dataset` | coInts |  | simple |  |  |
| `nozzle_height` | coFloat |  | develop | 2.5 | Nozzle height |
| `nozzle_hrc` | coInt |  | develop |  | Nozzle HRC |
| `nozzle_temperature` | coInts |  | simple |  | Other layers |
| `nozzle_temperature_initial_layer` | coInts |  | simple |  | First layer |
| `nozzle_temperature_range_high` | coInts |  | simple |  | Max |
| `nozzle_temperature_range_low` | coInts |  | simple |  | Min |
| `nozzle_type` | coEnums |  | advanced | { ntUndefine } | Nozzle type |
| `nozzle_volume` | coFloats |  | advanced |  | Nozzle volume |
| `nozzle_volume_type` | coEnums |  | simple |  | Nozzle Volume Type |
| `only_one_wall_first_layer` | coBool | Quality | simple | False | Only one wall on first layer |
| `only_one_wall_top` | coBool | Quality | simple | False | Only one wall on top surfaces |
| `ooze_prevention` | coBool |  | advanced | False | Enable |
| `other_layers_print_sequence` | coInts |  | simple |  | Other layers print sequence |
| `other_layers_print_sequence_nums` | coInt |  | simple |  | The number of other layers print sequence |
| `other_layers_sequence_choice` | coEnum | Quality | simple | flsAuto | Other layers filament sequence |
| `outer_wall_acceleration` | coFloat | Speed | advanced | 500 | Outer wall |
| `outer_wall_filament_id` | coInt | Extruders | advanced | 0 | Outer walls |
| `outer_wall_flow_ratio` | coFloat | Advanced | advanced | 1 | Outer wall flow ratio |
| `outer_wall_jerk` | coFloat | Speed | advanced | 9 | Outer wall |
| `outer_wall_line_width` | coFloatOrPercent | Quality | advanced | {"value": 0.0, "percent": false} | Outer wall |
| `outer_wall_speed` | coFloat | Speed | advanced | 60 | Outer wall |
| `overhang_1_4_speed` | coFloatOrPercent | Speed | advanced | {"value": 0.0, "percent": false} | 10% |
| `overhang_2_4_speed` | coFloatOrPercent | Speed | advanced | {"value": 0.0, "percent": false} | 25% |
| `overhang_3_4_speed` | coFloatOrPercent | Speed | advanced | {"value": 0.0, "percent": false} | 50% |
| `overhang_4_4_speed` | coFloatOrPercent | Speed | advanced | {"value": 0.0, "percent": false} | 75% |
| `overhang_fan_speed` | coInts |  | advanced |  | Overhangs and external bridges fan speed |
| `overhang_fan_threshold` | coEnums |  | advanced | int | Overhang cooling activation threshold |
| `overhang_flow_ratio` | coFloat | Advanced | advanced | 1 | Overhang flow ratio |
| `overhang_reverse` | coBool | Quality | advanced | False | Reverse on even |
| `overhang_reverse_internal_only` | coBool | Quality | advanced | False | Reverse only internal perimeters |
| `overhang_reverse_threshold` | coFloatOrPercent | Quality | advanced | {"value": 50.0, "percent": true} | Reverse threshold |
| `parking_pos_retraction` | coFloat |  | advanced | 92.0 | Filament parking position |
| `part_cooling_fan_min_pwm` | coInt |  | advanced | 0 | Minimum non-zero part cooling fan speed |
| `pellet_flow_coefficient` | coFloats |  | simple |  | Pellet flow coefficient |
| `pellet_modded_printer` | coBool |  | simple | False | Pellet Modded Printer |
| `physical_extruder_map` | coInts |  | develop |  | Map the logical extruder to physical extruder |
| `post_process` | coStrings |  | advanced |  | Post-processing Scripts |
| `precise_outer_wall` | coBool | Quality | simple |  | Precise wall |
| `precise_z_height` | coBool | Quality | advanced | 0 | Precise Z height |
| `preheat_steps` | coInt |  | develop | 1 | Preheat steps |
| `preheat_time` | coFloat |  | advanced | 30.0 | Preheat time |
| `pressure_advance` | coFloats |  | advanced |  | Pressure advance |
| `prime_tower_brim_width` | coFloat |  | advanced | 3.0 | Brim width |
| `prime_tower_enable_framework` | coBool |  | advanced | False | Internal ribs |
| `prime_tower_flat_ironing` | coBool |  | advanced | False |  |
| `prime_tower_infill_gap` | coPercent |  | advanced | 150 | Infill gap |
| `prime_tower_skip_points` | coBool |  | advanced | True | Skip points |
| `prime_tower_width` | coFloat |  | simple | 60.0 | Width |
| `prime_volume` | coFloat |  | simple | 45.0 | Prime volume |
| `print_compatible_printers` | coStrings |  | simple |  |  |
| `print_extruder_id` | coInts |  | simple |  | Print extruder id |
| `print_extruder_variant` | coStrings |  | simple |  | Print's extruder variant |
| `print_flow_ratio` | coFloat | Quality | advanced | 1.f | Flow ratio |
| `print_order` | coEnum |  | advanced | PrintOrder::Default | Intra-layer order |
| `print_sequence` | coEnum |  | simple | PrintSequence::ByLayer | Print sequence |
| `print_settings_id` | coString |  | simple |  |  |
| `printer_extruder_id` | coInts |  | simple |  | Printer extruder id |
| `printer_extruder_variant` | coStrings |  | simple |  | Printer's extruder variant |
| `printer_model` | coString |  | simple |  | Printer type |
| `printer_notes` | coString |  | advanced |  | Printer notes |
| `printer_settings_id` | coString |  | simple |  |  |
| `printer_structure` | coEnum |  | develop | psUndefine | Printer structure |
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
| `reduce_fan_stop_start_freq` | coBools |  | simple |  | Keep fan always on |
| `reduce_infill_retraction` | coBool |  | advanced | False | Reduce infill retraction |
| `relative_bridge_angle` | coBool | Strength | advanced | False | Relative bridge angle |
| `required_nozzle_HRC` | coInts |  | develop |  | Required nozzle HRC |
| `resolution` | coFloat |  | advanced | 0.01 | Resolution |
| `resonance_avoidance` | coBool |  | advanced | False | Resonance avoidance |
| `retract_before_wipe` | coPercents |  | advanced |  | Retract amount before wipe |
| `retract_length_toolchange` | coFloats |  | advanced |  | Length |
| `retract_lift_above` | coFloats |  | advanced |  | Only lift Z above |
| `retract_lift_below` | coFloats |  | advanced |  | Only lift Z below |
| `retract_lift_enforce` | coEnums |  | advanced |  | On surfaces |
| `retract_restart_extra` | coFloats |  | advanced |  | Extra length on restart |
| `retract_restart_extra_toolchange` | coFloats |  | advanced |  | Extra length on restart |
| `retract_when_changing_layer` | coBools |  | advanced |  | Retract when change layer |
| `retraction_distances_when_cut` | coFloats |  | develop |  | Retraction distance when cut |
| `retraction_distances_when_ec` | coFloats |  | advanced |  | Retraction distance when extruder change |
| `retraction_length` | coFloats |  | simple |  | Length |
| `retraction_minimum_travel` | coFloats |  | advanced |  | Travel distance threshold |
| `retraction_speed` | coFloats |  | advanced |  | Retraction Speed |
| `role_based_wipe_speed` | coBool | Speed | advanced | True | Role base wipe speed |
| `scan_first_layer` | coBool |  | advanced | False | Scan first layer |
| `scarf_angle_threshold` | coInt | Quality | advanced | 155 | Conditional angle threshold |
| `scarf_joint_flow_ratio` | coFloat | Quality | expert | 1 | Scarf joint flow ratio |
| `scarf_joint_speed` | coFloatOrPercent | Quality | advanced | {"value": 100.0, "percent": true} | Scarf joint speed |
| `scarf_overhang_threshold` | coPercent | Quality | advanced | 40 | Conditional overhang threshold |
| `seam_gap` | coFloatOrPercent | Quality | advanced | {"value": 10.0, "percent": true} | Seam gap |
| `seam_position` | coEnum | Quality | simple | spAligned | Seam position |
| `seam_slope_conditional` | coBool | Quality | advanced | False | Conditional scarf joint |
| `seam_slope_entire_loop` | coBool | Quality | advanced | False | Scarf around entire wall |
| `seam_slope_inner_walls` | coBool | Quality | advanced | False | Scarf joint for inner walls |
| `seam_slope_min_length` | coFloat | Quality | advanced | 20 | Scarf length |
| `seam_slope_start_height` | coFloatOrPercent | Quality | advanced | {"value": 0.0, "percent": false} | Scarf start height |
| `seam_slope_steps` | coInt | Quality | advanced | 10 | Scarf steps |
| `seam_slope_type` | coEnum | Quality | advanced | SeamScarfType::None | Scarf joint seam (beta) |
| `set_other_flow_ratios` | coBool | Advanced | advanced | False | Set other flow ratios |
| `silent_mode` | coBool |  | develop | False | Supports silent mode |
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
| `skirt_type` | coEnum |  | advanced | stCombined | Skirt type |
| `slice_closing_radius` | coFloat | Quality | advanced | 0.049 | Slice gap closing radius |
| `slicing_mode` | coEnum | Other | advanced | SlicingMode::Regular | Slicing Mode |
| `slow_down_for_layer_cooling` | coBools |  | simple |  | Slow printing down for better layer cooling |
| `slow_down_layer_time` | coFloats |  | simple |  | Layer time |
| `slow_down_layers` | coInt | Speed | advanced | 0 | Number of slow layers |
| `slow_down_min_speed` | coFloats |  | advanced |  | Min print speed |
| `slowdown_for_curled_perimeters` | coBool | Speed | advanced |  | Slow down for curled perimeters |
| `small_area_infill_flow_compensation` | coBool | Quality | advanced | False | Small area flow compensation (beta) |
| `small_area_infill_flow_compensation_model` | coStrings |  | advanced |  | Flow Compensation Model |
| `small_perimeter_speed` | coFloatOrPercent | Speed | advanced | {"value": 50.0, "percent": true} | Small perimeters |
| `small_perimeter_threshold` | coFloat | Speed | advanced | 0 | Small perimeters threshold |
| `solid_infill_direction` | coFloat | Strength | advanced | 45 | Solid infill direction |
| `solid_infill_rotate_template` | coString | Strength | advanced |  | Solid infill rotation template |
| `sparse_infill_acceleration` | coFloatOrPercent | Speed | advanced | {"value": 100.0, "percent": true} | Sparse infill |
| `sparse_infill_density` | coPercent | Strength | simple | 20 | Sparse infill density |
| `sparse_infill_filament_id` | coInt | Extruders | advanced | 0 | Infill |
| `sparse_infill_flow_ratio` | coFloat | Advanced | advanced | 1 | Sparse infill flow ratio |
| `sparse_infill_line_width` | coFloatOrPercent | Quality | advanced | {"value": 0.0, "percent": false} | Sparse infill |
| `sparse_infill_pattern` | coEnum | Strength | simple | ipCrossHatch | Sparse infill pattern |
| `sparse_infill_rotate_template` | coString | Strength | advanced |  | Sparse infill rotation template |
| `sparse_infill_speed` | coFloat | Speed | advanced | 100 | Sparse infill |
| `spiral_finishing_flow_ratio` | coFloat |  | advanced | 0 | Spiral finishing flow ratio |
| `spiral_mode` | coBool |  | simple | False | Spiral vase |
| `spiral_mode_max_xy_smoothing` | coFloatOrPercent |  | advanced | {"value": 200.0, "percent": true} | Max XY Smoothing |
| `spiral_mode_smooth` | coBool |  | simple | False | Smooth Spiral |
| `spiral_starting_flow_ratio` | coFloat |  | advanced | 0 | Spiral starting flow ratio |
| `staggered_inner_seams` | coBool | Quality | advanced | False | Staggered inner seams |
| `standby_temperature_delta` | coInt |  | advanced | -5 | Temperature variation |
| `start_end_points` | coPoints |  | develop | 30, -3 | Start end points |
| `supertack_plate_temp` | coInts |  | simple |  | Other layers |
| `supertack_plate_temp_initial_layer` | coInts |  | simple |  | First layer |
| `support_air_filtration` | coBool |  | develop | True | Support air filtration |
| `support_angle` | coFloat | Support | advanced | 0 | Pattern angle |
| `support_base_pattern` | coEnum | Support | advanced | smpDefault | Base pattern |
| `support_base_pattern_spacing` | coFloat | Support | advanced | 2.5 | Base pattern spacing |
| `support_bottom_interface_spacing` | coFloat | Support | advanced | 0.5 | Bottom interface spacing |
| `support_bottom_z_distance` | coFloat | Support | advanced | 0.2 | Bottom Z distance |
| `support_chamber_temp_control` | coBool |  | advanced | True | Support control chamber temperature |
| `support_critical_regions_only` | coBool | Support | advanced | False | Support critical regions only |
| `support_expansion` | coFloat | Support | advanced | 0 | Normal Support expansion |
| `support_filament` | coInt | Support | simple | 0 | Support/raft base |
| `support_flow_ratio` | coFloat | Advanced | advanced | 1 | Support flow ratio |
| `support_interface_bottom_layers` | coInt | Support | advanced | 0 | Bottom interface layers |
| `support_interface_filament` | coInt | Support | simple | 0 | Support/raft interface |
| `support_interface_flow_ratio` | coFloat | Advanced | advanced | 1 | Support interface flow ratio |
| `support_interface_loop_pattern` | coBool | Support | advanced | False | Interface use loop pattern |
| `support_interface_not_for_body` | coBool | Support | simple | True | Avoid interface filament for base |
| `support_interface_pattern` | coEnum | Support | advanced | smipAuto | Interface pattern |
| `support_interface_spacing` | coFloat | Support | advanced | 0.5 | Top interface spacing |
| `support_interface_speed` | coFloat | Speed | advanced | 80 | Support interface |
| `support_interface_top_layers` | coInt | Support | advanced | 3 | Top interface layers |
| `support_ironing` | coBool | Support | advanced | False | Ironing Support Interface |
| `support_ironing_flow` | coPercent | Support | advanced | 10 | Support Ironing flow |
| `support_ironing_pattern` | coEnum | Support | advanced | ipRectilinear | Support Ironing Pattern |
| `support_ironing_spacing` | coFloat | Support | advanced | 0.1 | Support Ironing line spacing |
| `support_line_width` | coFloatOrPercent | Quality | advanced | {"value": 0.0, "percent": false} | Support |
| `support_material_interface_fan_speed` | coInts |  | advanced |  | Support interface fan speed |
| `support_multi_bed_types` | coBool |  | simple | False | Support multi bed types |
| `support_object_first_layer_gap` | coFloat | Support | advanced | 0.2 | Support/object first layer gap |
| `support_object_skip_flush` | coBool |  | simple | False |  |
| `support_object_xy_distance` | coFloat | Support | advanced | 0.35 | Support/object XY distance |
| `support_on_build_plate_only` | coBool | Support | simple | False | On build plate only |
| `support_remove_small_overhang` | coBool | Support | advanced | True | Ignore small overhangs |
| `support_speed` | coFloat | Speed | advanced | 80 | Support |
| `support_style` | coEnum | Support | advanced | smsDefault | Style |
| `support_threshold_angle` | coInt | Support | simple | 30 | Threshold angle |
| `support_threshold_overlap` | coFloatOrPercent | Support | simple | {"value": 50.0, "percent": true} | Threshold overlap |
| `support_top_z_distance` | coFloat | Support | advanced | 0.2 | Top Z distance |
| `support_type` | coEnum | Support | simple | stNormalAuto | Type |
| `symmetric_infill_y_axis` | coBool | Strength | advanced | False | Symmetric infill Y axis |
| `temperature_vitrification` | coInts |  | simple |  | Softening temperature |
| `template_custom_gcode` | coString |  | advanced |  | Custom G-code |
| `textured_cool_plate_temp` | coInts |  | simple |  | Other layers |
| `textured_cool_plate_temp_initial_layer` | coInts |  | simple |  | First layer |
| `textured_plate_temp` | coInts |  | simple |  | Other layers |
| `textured_plate_temp_initial_layer` | coInts |  | simple |  | First layer |
| `thick_bridges` | coBool | Quality | advanced | False | Thick external bridges |
| `thick_internal_bridges` | coBool | Quality | advanced | True | Thick internal bridges |
| `thumbnails` | coString |  | advanced | 48x48/PNG,300x300/PNG | G-code thumbnails |
| `thumbnails_format` | coEnum |  | advanced | GCodeThumbnailsFormat::PNG | Format of G-code thumbnails |
| `time_cost` | coFloat |  | advanced | 0 | Time cost |
| `time_lapse_gcode` | coString |  | advanced |  | Timelapse G-code |
| `timelapse_type` | coEnum |  | simple | tlTraditional | Timelapse |
| `tool_change_on_wipe_tower` | coBool |  | advanced | False | Tool change on wipe tower |
| `top_bottom_infill_wall_overlap` | coPercent | Strength | advanced | 25 | Top/Bottom solid infill/wall overlap |
| `top_shell_layers` | coInt | Strength | simple | 4 | Top shell layers |
| `top_shell_thickness` | coFloat | Strength | simple | 0.6 | Top shell thickness |
| `top_solid_infill_flow_ratio` | coFloat | Advanced | advanced | 1 | Top surface flow ratio |
| `top_surface_acceleration` | coFloat | Speed | advanced | 500 | Top surface |
| `top_surface_density` | coPercent | Strength | simple | 100 | Top surface density |
| `top_surface_filament_id` | coInt | Extruders | advanced | 0 | Top surface |
| `top_surface_jerk` | coFloat | Speed | advanced | 9 | Top surface |
| `top_surface_line_width` | coFloatOrPercent | Quality | advanced | {"value": 0.0, "percent": false} | Top surface |
| `top_surface_pattern` | coEnum | Strength | simple | ipMonotonicLine | Top surface pattern |
| `top_surface_speed` | coFloat | Speed | advanced | 100 | Top surface |
| `travel_acceleration` | coFloat | Speed | advanced | 10000 | Travel |
| `travel_jerk` | coFloat | Speed | advanced | 12 | Travel |
| `travel_slope` | coFloats |  | advanced |  | Traveling angle |
| `travel_speed` | coFloat |  | advanced | 120 | Travel |
| `travel_speed_z` | coFloat |  | develop | 0.0 |  |
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
| `upward_compatible_machine` | coStrings |  | advanced |  | upward compatible machine |
| `use_firmware_retraction` | coBool |  | advanced | False | Use firmware retraction |
| `use_relative_e_distances` | coBool |  | advanced | True | Use relative E distances |
| `volumetric_speed_coefficients` | coStrings |  | simple |  | Max volumetric speed multinomial coefficients |
| `wall_direction` | coEnum | Quality | advanced | WallDirection::CounterClockwise | Wall loop direction |
| `wall_distribution_count` | coInt | Quality | advanced | 1 | Wall distribution count |
| `wall_generator` | coEnum | Quality | advanced | PerimeterGeneratorType::Arachne | Wall generator |
| `wall_loops` | coInt | Strength | simple | 2 | Wall loops |
| `wall_maximum_deviation` | coFloat | Quality | expert | 0.025f | Maximum wall deviation |
| `wall_maximum_resolution` | coFloat | Quality | expert | 0.5f | Maximum wall resolution |
| `wall_sequence` | coEnum | Quality | advanced | WallSequence::InnerOuter | Walls printing order |
| `wall_transition_angle` | coFloat | Quality | advanced | 10.0 | Wall transitioning threshold angle |
| `wall_transition_filter_deviation` | coPercent | Quality | advanced | 25 | Wall transitioning filter margin |
| `wall_transition_length` | coPercent | Quality | advanced | 100 | Wall transition length |
| `wipe` | coBools |  | advanced |  | Wipe while retracting |
| `wipe_before_external_loop` | coBool | Quality | advanced | False | Wipe before external loop |
| `wipe_distance` | coFloats |  | advanced |  | Wipe Distance |
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
| `wipe_tower_type` | coEnum |  | advanced | WipeTowerType::Type2 | Wipe tower type |
| `wipe_tower_wall_type` | coEnum |  | advanced | wtwRib | Wall type |
| `wipe_tower_x` | coFloats |  | develop |  |  |
| `wipe_tower_y` | coFloats |  | develop |  |  |
| `wiping_volumes_extruders` | coFloats |  | simple |  | Purging volumes - load/unload volumes |
| `wrapping_detection_gcode` | coString |  | advanced |  | Clumping detection G-code |
| `wrapping_detection_layers` | coInt |  | develop | 20 | Clumping detection layers |
| `wrapping_exclude_area` | coPoints |  | advanced |  | Probing exclude area of clumping |
| `xy_contour_compensation` | coFloat | Quality | advanced | 0 | X-Y contour compensation |
| `xy_hole_compensation` | coFloat | Quality | advanced | 0 | X-Y hole compensation |
| `z_hop` | coFloats |  | simple |  | Z-hop height |
| `z_hop_types` | coEnums |  | advanced |  | Z-hop type |
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
| `display_orientation` | coEnum |  | advanced | sladoPortrait |  |
| `display_pixels_x` | coInt |  | simple | 2560 | X |
| `display_pixels_y` | coInt |  | simple | 1440 | Y |
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
| `material_print_speed` | coEnum |  | advanced | slamsFast |  |
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
| `support_pillar_connection_mode` | coEnum |  | advanced | slapcmDynamic |  |
| `support_pillar_diameter` | coFloat |  | simple | 1.0 |  |
| `support_pillar_widening_factor` | coFloat |  | advanced | 0.0 |  |
| `support_points_density_relative` | coInt |  | simple | 100 |  |
| `support_points_minimal_distance` | coFloat |  | simple | 1.0 |  |
| `support_small_pillar_diameter_percent` | coPercent |  | advanced | 50 |  |
| `supports_enable` | coBool |  | simple | True |  |

## Groupe `cli:CLIActions` (17 paramètres)

| Clé | Type | Catégorie | Mode | Défaut | Libellé |
|---|---|---|---|---|---|
| `export_3mf` | coString |  | simple | output.3mf | Export 3MF |
| `export_settings` | coString |  | simple | output.json | Export Settings |
| `export_slicedata` | coString |  | simple | cached_data | Export slicing data |
| `export_stl` | coBool |  | simple | False | Export STL |
| `export_stls` | coString |  | simple | stl_path | Export multiple STLs |
| `help` | coBool |  | simple | False | Help |
| `info` | coBool |  | simple | False | Output Model Info |
| `load_defaultfila` | coBool |  | simple | False | Load default filaments |
| `load_slicedata` | coStrings |  | simple | cached_data | Load slicing data |
| `min_save` | coBool |  | simple | False | Minimum save |
| `mstpp` | coInt |  | simple | 300 | mstpp |
| `mtcpp` | coInt |  | simple | 1000000 | mtcpp |
| `no_check` | coBool |  | simple | False | No check |
| `normative_check` | coBool |  | simple | True | Normative check |
| `pipe` | coString |  | simple |  | Send progress to pipe |
| `slice` | coInt |  | simple | 0 | Slice |
| `uptodate` | coBool |  | simple | False | UpToDate |

## Groupe `cli:CLIMisc` (26 paramètres)

| Clé | Type | Catégorie | Mode | Défaut | Libellé |
|---|---|---|---|---|---|
| `allow_mix_temp` | coBool |  | simple | False | Allow filaments with high/low temperature to be printed together |
| `allow_multicolor_oneplate` | coBool |  | simple | True | Allow multiple colors on one plate |
| `allow_newer_file` | coBool |  | simple | False | Allow 3MF with newer version to be sliced |
| `allow_rotations` | coBool |  | simple | True | Allow rotation when arranging |
| `avoid_extrusion_cali_region` | coBool |  | simple | False | Avoid extrusion calibrate region when arranging |
| `clone_objects` | coInts |  | simple |  | Clone Objects |
| `datadir` | coString |  | simple |  | Data directory |
| `debug` | coInt |  | simple | 1 | Debug level |
| `downward_check` | coBool |  | simple | False | Downward machines check |
| `downward_settings` | coStrings |  | simple |  | Downward machines settings |
| `enable_timelapse` | coBool |  | simple | False | Enable timelapse for print |
| `load_assemble_list` | coString |  | simple |  | Load assemble list |
| `load_custom_gcodes` | coString |  | simple |  | Load custom G-code |
| `load_filament_ids` | coInts |  | simple |  | Load filament IDs |
| `load_filaments` | coStrings |  | simple |  | Load Filament Settings |
| `load_settings` | coStrings |  | simple |  | Load General Settings |
| `logfile` | coInt |  | simple |  | Log file |
| `makerlab_name` | coString |  | simple |  | MakerLab name |
| `makerlab_version` | coString |  | simple |  | MakerLab version |
| `metadata_name` | coStrings |  | simple |  | Metadata name list |
| `metadata_value` | coStrings |  | simple |  | Metadata value list |
| `outputdir` | coString |  | simple |  | Output directory |
| `skip_modified_gcodes` | coBool |  | simple | False | Skip modified G-code in 3MF |
| `skip_objects` | coInts |  | simple |  | Skip Objects |
| `uptodate_filaments` | coStrings |  | simple |  | Load uptodate filament settings when using uptodate |
| `uptodate_settings` | coStrings |  | simple |  | Load uptodate process/machine settings when using uptodate |

## Groupe `cli:CLITransform` (9 paramètres)

| Clé | Type | Catégorie | Mode | Défaut | Libellé |
|---|---|---|---|---|---|
| `arrange` | coInt |  | simple | 0 | Arrange Options |
| `assemble` | coBool |  | simple | False | Assemble |
| `convert_unit` | coBool |  | simple | False | Convert Unit |
| `ensure_on_bed` | coBool |  | simple | False | Ensure on bed |
| `orient` | coInt |  | simple | 0 | Orient Options |
| `repetitions` | coInt |  | simple | 1 | Repetition count |
| `rotate` | coFloat |  | simple | 0 | Rotate |
| `rotate_x` | coFloat |  | simple | 0 | Rotate around X |
| `rotate_y` | coFloat |  | simple | 0 | Rotate around Y |

## Groupe `other:CustomGcodeSpecific` (4 paramètres)

| Clé | Type | Catégorie | Mode | Défaut | Libellé |
|---|---|---|---|---|---|
| `filament_extruder_id` | coInt |  | simple |  | Filament extruder ID |
| `layer_num` | coInt |  | simple |  | Layer number |
| `layer_z` | coFloat |  | simple |  | Layer Z |
| `max_layer_z` | coFloat |  | simple |  | Maximal layer Z |

## Groupe `other:Dimensions` (7 paramètres)

| Clé | Type | Catégorie | Mode | Défaut | Libellé |
|---|---|---|---|---|---|
| `first_layer_print_convex_hull` | coPoints |  | simple |  | First layer convex hull |
| `first_layer_print_max` | coFloats |  | simple |  | Top-right corner of the first layer bounding box |
| `first_layer_print_min` | coFloats |  | simple |  | Bottom-left corner of the first layer bounding box |
| `first_layer_print_size` | coFloats |  | simple |  | Size of the first layer bounding box |
| `print_bed_max` | coFloats |  | simple |  | Top-right corner of print bed bounding box |
| `print_bed_min` | coFloats |  | simple |  | Bottom-left corner of print bed bounding box |
| `print_bed_size` | coFloats |  | simple |  | Size of the print bed bounding box |

## Groupe `other:ObjectsInfo` (4 paramètres)

| Clé | Type | Catégorie | Mode | Défaut | Libellé |
|---|---|---|---|---|---|
| `input_filename_base` | coString |  | simple |  | Input filename without extension |
| `num_instances` | coInt |  | simple |  | Number of instances |
| `num_objects` | coInt |  | simple |  | Number of objects |
| `scale` | coStrings |  | simple |  | Scale per object |

## Groupe `other:OtherPresets` (4 paramètres)

| Clé | Type | Catégorie | Mode | Défaut | Libellé |
|---|---|---|---|---|---|
| `filament_preset` | coString |  | simple |  | Filament preset name |
| `physical_printer_preset` | coString |  | simple |  | Physical printer name |
| `print_preset` | coString |  | simple |  | Print preset name |
| `printer_preset` | coString |  | simple |  | Printer preset name |

## Groupe `other:OtherSlicingStates` (8 paramètres)

| Clé | Type | Catégorie | Mode | Défaut | Libellé |
|---|---|---|---|---|---|
| `current_extruder` | coInt |  | simple |  | Current extruder |
| `current_object_idx` | coInt |  | simple |  | Current object index |
| `has_single_extruder_multi_material_priming` | coBool |  | simple |  | Has single extruder MM priming |
| `has_wipe_tower` | coBool |  | simple |  | Has wipe tower |
| `initial_extruder` | coInt |  | simple |  | Initial extruder |
| `initial_tool` | coInt |  | simple |  | Initial tool |
| `is_extruder_used` | coBools |  | simple |  | Is extruder used? |
| `num_extruders` | coInt |  | simple |  | Number of extruders |

## Groupe `other:PrintStatistics` (16 paramètres)

| Clé | Type | Catégorie | Mode | Défaut | Libellé |
|---|---|---|---|---|---|
| `extruded_volume` | coFloats |  | simple |  | Volume per extruder |
| `extruded_volume_total` | coFloat |  | simple |  | Total volume |
| `extruded_weight` | coFloats |  | simple |  | Weight per extruder |
| `extruded_weight_total` | coFloat |  | simple |  | Total weight |
| `normal_print_time` | coString |  | simple |  | Print time (normal mode) |
| `print_time` | coString |  | simple |  | Print time (normal mode) |
| `print_time_sec` | coString |  | simple |  | Print time (seconds) |
| `silent_print_time` | coString |  | simple |  | Print time (silent mode) |
| `total_cost` | coFloat |  | simple |  | Total cost |
| `total_layer_count` | coInt |  | simple |  | Total layer count |
| `total_toolchanges` | coInt |  | simple |  | Total tool changes |
| `total_weight` | coFloat |  | simple |  | Total weight |
| `total_wipe_tower_cost` | coFloat |  | simple |  | Total wipe tower cost |
| `total_wipe_tower_filament` | coFloat |  | simple |  | Wipe tower volume |
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
