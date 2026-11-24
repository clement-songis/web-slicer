// Réglages par objet / par volume (parité Orca : SettingsFactory::get_options,
// GUI_Factories.cpp). Un objet accepte les clés de PrintRegionConfig +
// PrintObjectConfig, une pièce/un modificateur seulement PrintRegionConfig.
// Listes extraites de PRINT_CONFIG_CLASS_DEFINE (libslic3r/PrintConfig.hpp,
// Orca 2.4.1). Fonctions pures → testables sans Svelte.

import { PARAMS, type ParamDef } from '../../generated/params';
import { orcaDefault } from '#lib/settings/editing';
import { optionVisible, queryRank, type DisplayMode } from '#lib/settings/filter';

/** Clés de PrintRegionConfig (réglables par objet et par volume). */
export const REGION_KEYS: readonly string[] = [
	'print_extruder_id',
	'print_extruder_variant',
	'bottom_shell_layers',
	'bottom_shell_thickness',
	'bridge_angle',
	'internal_bridge_angle',
	'relative_bridge_angle',
	'bridge_flow',
	'bridge_line_width',
	'internal_bridge_flow',
	'bridge_speed',
	'internal_bridge_speed',
	'ensure_vertical_shell_thickness',
	'top_surface_density',
	'bottom_surface_density',
	'top_surface_pattern',
	'bottom_surface_pattern',
	'internal_solid_infill_pattern',
	'outer_wall_line_width',
	'outer_wall_speed',
	'infill_direction',
	'solid_infill_direction',
	'solid_infill_rotate_template',
	'symmetric_infill_y_axis',
	'infill_shift_step',
	'sparse_infill_rotate_template',
	'sparse_infill_density',
	'sparse_infill_pattern',
	'lateral_lattice_angle_1',
	'lateral_lattice_angle_2',
	'infill_overhang_angle',
	'lightning_overhang_angle',
	'lightning_prune_angle',
	'lightning_straightening_angle',
	'align_infill_direction_to_model',
	'extra_solid_infills',
	'fuzzy_skin',
	'fuzzy_skin_thickness',
	'fuzzy_skin_point_distance',
	'fuzzy_skin_first_layer',
	'fuzzy_skin_noise_type',
	'fuzzy_skin_mode',
	'fuzzy_skin_scale',
	'fuzzy_skin_octaves',
	'fuzzy_skin_persistence',
	'fuzzy_skin_ripples_per_layer',
	'fuzzy_skin_ripple_offset',
	'fuzzy_skin_layers_between_ripple_offset',
	'gap_infill_speed',
	'sparse_infill_filament_id',
	'sparse_infill_line_width',
	'infill_wall_overlap',
	'top_bottom_infill_wall_overlap',
	'sparse_infill_speed',
	'skeleton_infill_density',
	'skin_infill_density',
	'infill_lock_depth',
	'skin_infill_depth',
	'skin_infill_line_width',
	'skeleton_infill_line_width',
	'infill_combination',
	'infill_combination_max_layer_height',
	'fill_multiline',
	'gyroid_optimized',
	'ironing_type',
	'ironing_pattern',
	'ironing_flow',
	'ironing_spacing',
	'ironing_inset',
	'ironing_direction',
	'ironing_speed',
	'ironing_angle',
	'ironing_angle_fixed',
	'filament_ironing_flow',
	'filament_ironing_spacing',
	'filament_ironing_inset',
	'filament_ironing_speed',
	'detect_overhang_wall',
	'outer_wall_filament_id',
	'inner_wall_filament_id',
	'inner_wall_line_width',
	'inner_wall_speed',
	'wall_loops',
	'alternate_extra_wall',
	'minimum_sparse_infill_area',
	'internal_solid_filament_id',
	'top_surface_filament_id',
	'bottom_surface_filament_id',
	'internal_solid_infill_line_width',
	'internal_solid_infill_speed',
	'detect_thin_wall',
	'top_surface_line_width',
	'top_shell_layers',
	'top_shell_thickness',
	'top_surface_speed',
	'enable_overhang_speed',
	'overhang_1_4_speed',
	'overhang_2_4_speed',
	'overhang_3_4_speed',
	'overhang_4_4_speed',
	'only_one_wall_top',
	'min_width_top_surface',
	'only_one_wall_first_layer',
	'print_flow_ratio',
	'seam_gap',
	'role_based_wipe_speed',
	'wipe_speed',
	'wipe_on_loops',
	'wipe_before_external_loop',
	'wall_infill_order',
	'precise_outer_wall',
	'bridge_density',
	'filter_out_gap_fill',
	'small_perimeter_speed',
	'small_perimeter_threshold',
	'top_solid_infill_flow_ratio',
	'bottom_solid_infill_flow_ratio',
	'infill_anchor',
	'infill_anchor_max',
	'make_overhang_printable',
	'extra_perimeters_on_overhangs',
	'slowdown_for_curled_perimeters',
	'hole_to_polyhole',
	'hole_to_polyhole_threshold',
	'hole_to_polyhole_twisted',
	'overhang_reverse',
	'overhang_reverse_internal_only',
	'overhang_reverse_threshold',
	'counterbore_hole_bridging',
	'wall_sequence',
	'is_infill_first',
	'small_area_infill_flow_compensation',
	'wall_direction',
	'first_layer_flow_ratio',
	'outer_wall_flow_ratio',
	'inner_wall_flow_ratio',
	'overhang_flow_ratio',
	'sparse_infill_flow_ratio',
	'internal_solid_infill_flow_ratio',
	'gap_fill_flow_ratio',
	'seam_slope_type',
	'seam_slope_conditional',
	'scarf_angle_threshold',
	'seam_slope_start_height',
	'seam_slope_entire_loop',
	'seam_slope_min_length',
	'seam_slope_steps',
	'seam_slope_inner_walls',
	'scarf_joint_speed',
	'scarf_joint_flow_ratio',
	'scarf_overhang_threshold',
	'zaa_enabled',
	'zaa_dont_alternate_fill_direction',
	'zaa_min_z',
	'zaa_minimize_perimeter_height'
];

/** Clés de PrintObjectConfig (réglables par objet seulement). */
export const OBJECT_KEYS: readonly string[] = [
	'brim_object_gap',
	'brim_flow_ratio',
	'brim_use_efc_outline',
	'brim_type',
	'brim_width',
	'brim_ears_detection_length',
	'brim_ears_max_angle',
	'skirt_start_angle',
	'bridge_no_support',
	'elefant_foot_compensation',
	'elefant_foot_compensation_layers',
	'elefant_foot_layers_density',
	'max_bridge_length',
	'line_width',
	'interface_shells',
	'layer_height',
	'mmu_segmented_region_max_width',
	'mmu_segmented_region_interlocking_depth',
	'raft_contact_distance',
	'raft_expansion',
	'raft_first_layer_density',
	'raft_first_layer_expansion',
	'raft_layers',
	'seam_position',
	'staggered_inner_seams',
	'slice_closing_radius',
	'slicing_mode',
	'enable_support',
	'support_type',
	'support_angle',
	'support_on_build_plate_only',
	'support_critical_regions_only',
	'support_remove_small_overhang',
	'support_top_z_distance',
	'support_bottom_z_distance',
	'enforce_support_layers',
	'support_filament',
	'support_line_width',
	'support_interface_not_for_body',
	'support_interface_loop_pattern',
	'support_interface_filament',
	'support_interface_top_layers',
	'support_interface_bottom_layers',
	'support_interface_spacing',
	'support_interface_speed',
	'support_base_pattern',
	'support_interface_pattern',
	'support_base_pattern_spacing',
	'support_expansion',
	'support_speed',
	'support_style',
	'set_other_flow_ratios',
	'support_flow_ratio',
	'support_interface_flow_ratio',
	'independent_support_layer_height',
	'thick_bridges',
	'thick_internal_bridges',
	'dont_filter_internal_bridges',
	'enable_extra_bridge_layer',
	'internal_bridge_density',
	'support_threshold_angle',
	'support_threshold_overlap',
	'support_object_xy_distance',
	'support_object_first_layer_gap',
	'support_ironing',
	'support_ironing_pattern',
	'support_ironing_flow',
	'support_ironing_spacing',
	'xy_hole_compensation',
	'xy_contour_compensation',
	'flush_into_objects',
	'flush_into_infill',
	'flush_into_support',
	'tree_support_branch_distance',
	'tree_support_tip_diameter',
	'tree_support_branch_diameter',
	'tree_support_branch_angle',
	'tree_support_branch_diameter_angle',
	'tree_support_angle_slow',
	'tree_support_wall_count',
	'tree_support_auto_brim',
	'tree_support_brim_width',
	'detect_narrow_internal_solid_infill',
	'adaptive_layer_height',
	'support_bottom_interface_spacing',
	'wall_generator',
	'wall_transition_length',
	'wall_transition_filter_deviation',
	'wall_transition_angle',
	'wall_distribution_count',
	'min_feature_size',
	'initial_layer_min_bead_width',
	'min_bead_width',
	'wall_maximum_resolution',
	'wall_maximum_deviation',
	'make_overhang_printable_angle',
	'make_overhang_printable_hole_size',
	'tree_support_branch_distance_organic',
	'tree_support_top_rate',
	'tree_support_branch_diameter_organic',
	'tree_support_branch_angle_organic',
	'gap_fill_target',
	'min_length_factor',
	'default_acceleration',
	'outer_wall_acceleration',
	'inner_wall_acceleration',
	'top_surface_acceleration',
	'initial_layer_acceleration',
	'bridge_acceleration',
	'travel_acceleration',
	'sparse_infill_acceleration',
	'internal_solid_infill_acceleration',
	'default_jerk',
	'outer_wall_jerk',
	'inner_wall_jerk',
	'infill_jerk',
	'top_surface_jerk',
	'initial_layer_jerk',
	'travel_jerk',
	'precise_z_height',
	'default_junction_deviation',
	'interlocking_beam',
	'interlocking_beam_width',
	'interlocking_orientation',
	'interlocking_beam_layer_count',
	'interlocking_depth',
	'interlocking_boundary_avoidance',
	'calib_flowrate_topinfill_special_order'
];

// Les clés de MachineEnvelopeConfig (machine_max_*, input_shaping_*,
// resonance_avoidance…) ne sont PAS des clés de région : Orca ne les propose
// pas par objet (elles appartiennent au profil imprimante).

/**
 * Catégories écartées de la liste « Ajouter des réglages » (`is_improper_category`,
 * GUI_Factories.cpp) : catégorie vide ; « Extruders »/« Wipe options » avec un
 * seul filament ; « Support material » pour une pièce/un modificateur.
 */
export function isImproperCategory(
	category: string,
	filamentCount: number,
	isPart: boolean
): boolean {
	return (
		!category ||
		(filamentCount <= 1 && (category === 'Extruders' || category === 'Wipe options')) ||
		(isPart && category === 'Support material')
	);
}

/** Clés surchargeables pour un objet (`isPart=false`) ou un volume. */
export function allowedKeys(isPart: boolean): string[] {
	const keys = isPart ? [...REGION_KEYS] : [...REGION_KEYS, ...OBJECT_KEYS];
	return [...new Set(keys)].filter((k) => PARAMS[k] !== undefined);
}

/** Définitions candidates à l'ajout : autorisées, non surchargées, visibles. */
export function searchOverridable(
	query: string,
	mode: DisplayMode,
	isPart: boolean,
	existing: Record<string, unknown>,
	limit = 30,
	filamentCount = 1
): ParamDef[] {
	return (
		allowedKeys(isPart)
			.filter((k) => !(k in existing))
			.map((k) => PARAMS[k])
			.filter((def) => !isImproperCategory(def.category, filamentCount, isPart))
			.filter((def) => optionVisible(def, mode, query))
			.map((def) => ({ def, rank: queryRank(def, query) }))
			// Avec une requête : les plus pertinents d'abord (libellé avant infobulle),
			// puis catégorie/clé ; les groupes suivent l'ordre du meilleur résultat.
			.sort(
				(a, b) =>
					a.rank - b.rank ||
					a.def.category.localeCompare(b.def.category) ||
					a.def.key.localeCompare(b.def.key)
			)
			.map(({ def }) => def)
			.slice(0, limit)
	);
}

/**
 * Ajoute une surcharge : valeur initiale = valeur du process actif (comme
 * Orca, qui copie la valeur courante), sinon le défaut du registre.
 */
export function addOverride(
	settings: Record<string, unknown>,
	key: string,
	base: Record<string, unknown> = {}
): Record<string, unknown> {
	if (key in settings) return settings;
	// Défaut au format des valeurs (pas la forme brute du registre).
	const initial = key in base ? base[key] : orcaDefault(key);
	return { ...settings, [key]: structuredClone(initial ?? '') };
}

/** Retire une surcharge (le preset reprend la main). */
export function removeOverride(
	settings: Record<string, unknown>,
	key: string
): Record<string, unknown> {
	const next = { ...settings };
	delete next[key];
	return next;
}

/** Clés surchargées triées par catégorie puis clé (affichage stable). */
export function overriddenKeys(settings: Record<string, unknown>): string[] {
	return Object.keys(settings).sort((a, b) => {
		const ca = PARAMS[a]?.category ?? '';
		const cb = PARAMS[b]?.category ?? '';
		return ca.localeCompare(cb) || a.localeCompare(b);
	});
}

/** Regroupe des définitions par catégorie (ordre d'entrée conservé). */
export function groupByCategory(defs: ParamDef[]): { category: string; defs: ParamDef[] }[] {
	const groups = new Map<string, ParamDef[]>();
	for (const def of defs) {
		const list = groups.get(def.category) ?? [];
		list.push(def);
		groups.set(def.category, list);
	}
	return [...groups].map(([category, list]) => ({ category, defs: list }));
}
