// Variantes de buse d'un preset filament (Orca Tab.cpp, TabFilament :
// m_variant_combo, generate_extruder_options, switch_excluder). Sur une
// imprimante multi-buses / multi-débits (Bambu H2D…), un filament porte une
// valeur par variante (`filament_extruder_variant` = ["Direct Drive Standard",
// "Direct Drive High Flow"]) pour les options de `filament_options_with_variant`
// (PrintConfig.cpp) ; le sélecteur choisit l'index édité. Pur.

/** Orca PrintConfig.cpp `filament_options_with_variant` (2.4.1). */
export const FILAMENT_VARIANT_OPTIONS: ReadonlySet<string> = new Set([
	'filament_flow_ratio',
	'filament_max_volumetric_speed',
	'filament_extruder_variant',
	'filament_retraction_length',
	'filament_z_hop',
	'filament_z_hop_types',
	'filament_retract_lift_above',
	'filament_retract_lift_below',
	'filament_retract_lift_enforce',
	'filament_retract_restart_extra',
	'filament_retraction_speed',
	'filament_deretraction_speed',
	'filament_retraction_minimum_travel',
	'filament_retract_when_changing_layer',
	'filament_wipe',
	'filament_wipe_distance',
	'filament_retract_before_wipe',
	'filament_long_retractions_when_cut',
	'filament_retraction_distances_when_cut',
	'long_retractions_when_ec',
	'retraction_distances_when_ec',
	'nozzle_temperature_initial_layer',
	'nozzle_temperature',
	'filament_flush_volumetric_speed',
	'filament_flush_temp',
	'filament_cooling_before_tower',
	'volumetric_speed_coefficients',
	'filament_adaptive_volumetric_speed',
	'filament_ironing_flow',
	'filament_ironing_spacing',
	'filament_ironing_inset',
	'filament_ironing_speed',
	'activate_air_filtration',
	'activate_air_filtration_during_print',
	'activate_air_filtration_on_completion',
	'during_print_exhaust_fan_speed',
	'complete_print_exhaust_fan_speed'
]);

/** Option filament éditée par variante de buse. */
export function isVariantOption(key: string): boolean {
	return FILAMENT_VARIANT_OPTIONS.has(key);
}

// get_nozzle_volume_type_string (Orca) : types de débit reconnus en fin de nom.
const NOZZLE_TYPES: [string, string][] = [
	['High Flow', 'Haut débit'],
	['Standard', 'Standard']
];
const DRIVES: Record<string, string> = {
	'Direct Drive': 'Entraînement direct',
	Bowden: 'Bowden'
};

/**
 * Libellés du sélecteur (generate_extruder_options : « <entraînement> :
 * <débit> » quand le nom finit par un type de débit connu, sinon le nom brut).
 */
export function variantLabels(raw: unknown): string[] {
	if (!Array.isArray(raw)) return [];
	return raw.map((v) => {
		const variant = typeof v === 'string' ? v : String(v ?? '');
		for (const [type, fr] of NOZZLE_TYPES) {
			if (variant.length > type.length + 1 && variant.endsWith(' ' + type)) {
				const drive = variant.slice(0, -type.length - 1);
				return `${DRIVES[drive] ?? drive} : ${fr}`;
			}
		}
		return variant;
	});
}

/**
 * Index édité d'une option : la variante choisie si l'option est par variante
 * et que son vecteur la contient, sinon 0 (première valeur, comportement
 * mono-buse).
 */
export function variantSlot(key: string, value: unknown, variant: number): number {
	return variant > 0 && isVariantOption(key) && Array.isArray(value) && value.length > variant
		? variant
		: 0;
}
