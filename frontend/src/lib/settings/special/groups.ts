// Structure des pages spéciales de réglages (T044), miroir de l'Annexe B.
// Ces pages ne suivent pas le layout générique `ui-layout.ts` : elles sont
// construites à la main par OrcaSlicer (`add_filament_overrides_page`,
// `TabPrinter::build_fff` Machine G-code, `TabFilament::build` Multimaterial).
// Données pures + logique de surcharge « N/A » → testables sans rendu.

import type { ParamDef } from '../../../generated/params';
import { resolveWidgetKind } from '../widgets/resolve';

/** Un groupe d'options d'une page spéciale (clés → `PARAMS`). */
export interface SpecialGroup {
	title: string;
	options: string[];
}

/** Surcharges filament (`TabFilament::add_filament_overrides_page`, Annexe B).
 *  Chaque option porte une case « activer » : décochée = N/A (le filament
 *  n'écrase pas la valeur de l'imprimante/process → valeur `null`). */
export const FILAMENT_OVERRIDES: SpecialGroup[] = [
	{
		title: 'Retraction',
		options: [
			'filament_retraction_length',
			'filament_z_hop',
			'filament_z_hop_types',
			'filament_retract_lift_above',
			'filament_retract_lift_below',
			'filament_retract_lift_enforce',
			'filament_retraction_speed',
			'filament_deretraction_speed',
			'filament_retract_restart_extra',
			'filament_retraction_minimum_travel',
			'filament_retract_when_changing_layer',
			'filament_wipe',
			'filament_wipe_distance',
			'filament_retract_before_wipe',
			'filament_long_retractions_when_cut',
			'filament_retraction_distances_when_cut'
		]
	},
	{
		title: 'Ironing',
		options: [
			'filament_ironing_flow',
			'filament_ironing_spacing',
			'filament_ironing_inset',
			'filament_ironing_speed'
		]
	}
];

/** G-code machine (`TabPrinter::build_fff`, Annexe B) : éditeurs multilignes. */
export const MACHINE_GCODE: SpecialGroup[] = [
	{ title: 'File header G-code', options: ['file_start_gcode'] },
	{ title: 'Machine start G-code', options: ['machine_start_gcode'] },
	{ title: 'Machine end G-code', options: ['machine_end_gcode'] },
	{ title: 'Printing by object G-code', options: ['printing_by_object_gcode'] },
	{ title: 'Before layer change G-code', options: ['before_layer_change_gcode'] },
	{ title: 'Layer change G-code', options: ['layer_change_gcode'] },
	{ title: 'Timelapse G-code', options: ['time_lapse_gcode'] },
	{ title: 'Clumping Detection G-code', options: ['wrapping_detection_gcode'] },
	{ title: 'Change filament G-code', options: ['change_filament_gcode'] },
	{ title: 'Change extrusion role G-code', options: ['change_extrusion_role_gcode'] },
	{ title: 'Pause G-code', options: ['machine_pause_gcode'] },
	{ title: 'Template Custom G-code', options: ['template_custom_gcode'] }
];

/** Multimaterial filament (`TabFilament::build`, Annexe B) : tables de purge et
 *  de changement d'outil. */
export const FILAMENT_MULTIMATERIAL: SpecialGroup[] = [
	{
		title: 'Wipe tower parameters',
		options: [
			'filament_minimal_purge_on_wipe_tower',
			'filament_tower_interface_pre_extrusion_dist',
			'filament_tower_interface_pre_extrusion_length',
			'filament_tower_ironing_area',
			'filament_tower_interface_purge_volume',
			'filament_tower_interface_print_temp'
		]
	},
	{
		title: 'Multi Filament',
		options: ['long_retractions_when_ec', 'retraction_distances_when_ec']
	},
	{
		title: 'Tool change parameters with single extruder MM printers',
		options: [
			'filament_loading_speed_start',
			'filament_loading_speed',
			'filament_unloading_speed_start',
			'filament_unloading_speed',
			'filament_toolchange_delay',
			'filament_cooling_moves',
			'filament_cooling_initial_speed',
			'filament_cooling_final_speed',
			'filament_stamping_loading_speed',
			'filament_stamping_distance'
		]
	},
	{
		title: 'Tool change parameters with multi extruder MM printers',
		options: [
			'filament_multitool_ramming',
			'filament_multitool_ramming_volume',
			'filament_multitool_ramming_flow'
		]
	}
];

/** Une surcharge filament est active si sa valeur n'est pas « N/A » (`null`). */
export function isOverrideActive(value: unknown): boolean {
	return value !== null && value !== undefined;
}

/** Valeur d'amorçage quand on active une surcharge N/A : défaut du registre,
 *  sinon un neutre adapté au widget. */
export function defaultFor(def: ParamDef): unknown {
	if (def.default !== null && def.default !== undefined) return def.default;
	switch (resolveWidgetKind(def)) {
		case 'bool':
			return false;
		case 'int':
		case 'float':
		case 'percent':
			return 0;
		case 'floatOrPercent':
			return '0';
		case 'enum':
			return def.enumValues[0] ?? '';
		case 'point':
			return [0, 0];
		default:
			return '';
	}
}
