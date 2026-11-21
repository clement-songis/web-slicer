import { describe, expect, it } from 'vitest';
import {
	allowedKeys,
	addOverride,
	removeOverride,
	searchOverridable,
	overriddenKeys,
	isImproperCategory,
	OBJECT_KEYS,
	REGION_KEYS
} from './object-settings';
import { PARAMS } from '../../generated/params';

describe('réglages par objet (parité SettingsFactory::get_options)', () => {
	it('un objet accepte wall_loops (région) et brim_width (objet)', () => {
		const keys = allowedKeys(false);
		expect(keys).toContain('wall_loops');
		expect(keys).toContain('brim_width');
	});
	it('un volume refuse les clés objet', () => {
		const keys = allowedKeys(true);
		expect(keys).toContain('sparse_infill_density');
		expect(OBJECT_KEYS).toContain('brim_width');
		expect(keys).not.toContain('brim_width');
	});
	it('exclut les clés machine et filament', () => {
		const keys = allowedKeys(false);
		expect(keys).not.toContain('nozzle_diameter');
		expect(keys).not.toContain('filament_type');
	});
	it('la recherche ignore les clés déjà surchargées', () => {
		const hits = searchOverridable('wall_loops', 'develop', false, {});
		expect(hits.map((d) => d.key)).toContain('wall_loops');
		const again = searchOverridable('wall_loops', 'develop', false, { wall_loops: 5 });
		expect(again.map((d) => d.key)).not.toContain('wall_loops');
	});
	it('ajout depuis le process actif puis suppression', () => {
		const s = addOverride({}, 'wall_loops', { wall_loops: 3 });
		expect(s).toEqual({ wall_loops: 3 });
		expect(addOverride(s, 'wall_loops', { wall_loops: 9 })).toBe(s);
		expect(removeOverride(s, 'wall_loops')).toEqual({});
	});
	it('défaut du registre si absent du process', () => {
		const s = addOverride({}, 'wall_loops');
		expect(s.wall_loops).not.toBeUndefined();
	});
	it('REGION_KEYS = PrintRegionConfig seul (pas de MachineEnvelopeConfig)', () => {
		for (const k of [
			'machine_max_acceleration_x',
			'machine_max_jerk_e',
			'input_shaping_freq_x',
			'resonance_avoidance',
			'emit_machine_limits_to_gcode'
		])
			expect(REGION_KEYS).not.toContain(k);
		expect(REGION_KEYS).toContain('zaa_minimize_perimeter_height');
		expect(allowedKeys(false)).not.toContain('machine_max_speed_x');
	});
	it('is_improper_category (GUI_Factories.cpp)', () => {
		expect(isImproperCategory('', 2, false)).toBe(true);
		expect(isImproperCategory('Extruders', 1, false)).toBe(true);
		expect(isImproperCategory('Wipe options', 1, false)).toBe(true);
		expect(isImproperCategory('Extruders', 2, false)).toBe(false);
		expect(isImproperCategory('Support material', 1, true)).toBe(true);
		expect(isImproperCategory('Support material', 1, false)).toBe(false);
		expect(isImproperCategory('Quality', 1, true)).toBe(false);
	});
	it('un seul filament : ni catégorie Extruders ni catégorie vide dans la recherche', () => {
		const hits = searchOverridable('', 'develop', false, {}, 1000, 1);
		expect(hits.length).toBeGreaterThan(0);
		expect(hits.some((d) => d.category === 'Extruders' || !d.category)).toBe(false);
		expect(hits.some((d) => d.key.startsWith('machine_'))).toBe(false);
		const multi = searchOverridable('', 'develop', false, {}, 1000, 2);
		const extruderKeys = allowedKeys(false).filter((k) => PARAMS[k]?.category === 'Extruders');
		if (extruderKeys.length) expect(multi.some((d) => d.category === 'Extruders')).toBe(true);
	});
	it('tri stable des clés surchargées', () => {
		expect(overriddenKeys({ wall_loops: 1, brim_width: 2 })).toHaveLength(2);
	});
});

// Audit 43 P-a : les options à liste (ConfigOptionEnum) de PrintRegionConfig /
// PrintObjectConfig (PrintConfig.hpp, Orca 2.4.1) sont réglables par objet.
describe('réglages par objet : listes de choix (audit 43 P-a)', () => {
	const REGION_ENUMS = [
		'ensure_vertical_shell_thickness',
		'top_surface_pattern',
		'bottom_surface_pattern',
		'internal_solid_infill_pattern',
		'sparse_infill_pattern',
		'fuzzy_skin',
		'fuzzy_skin_noise_type',
		'fuzzy_skin_mode',
		'ironing_type',
		'ironing_pattern',
		'counterbore_hole_bridging',
		'wall_sequence',
		'wall_direction',
		'seam_slope_type'
	];
	const OBJECT_ENUMS = [
		'brim_type',
		'seam_position',
		'slicing_mode',
		'support_type',
		'support_base_pattern',
		'support_interface_pattern',
		'support_style',
		'dont_filter_internal_bridges',
		'enable_extra_bridge_layer',
		'support_ironing_pattern',
		'wall_generator',
		'gap_fill_target'
	];
	it('toutes les listes de région sont proposées pour un objet et une pièce', () => {
		for (const k of REGION_ENUMS) {
			expect(PARAMS[k]?.type, k).toBe('coEnum');
			expect(allowedKeys(false), k).toContain(k);
			expect(allowedKeys(true), k).toContain(k);
		}
	});
	it('les listes de PrintObjectConfig : objet seulement', () => {
		for (const k of OBJECT_ENUMS) {
			expect(PARAMS[k]?.type, k).toBe('coEnum');
			expect(allowedKeys(false), k).toContain(k);
			expect(allowedKeys(true), k).not.toContain(k);
		}
	});
	it('recherche « ironing_type » et « fuzzy_skin » (repro p5.mjs)', () => {
		expect(searchOverridable('ironing_type', 'develop', true, {}).map((d) => d.key)).toContain(
			'ironing_type'
		);
		const fuzzy = searchOverridable('fuzzy_skin', 'develop', false, {}).map((d) => d.key);
		expect(fuzzy).toEqual(
			expect.arrayContaining(['fuzzy_skin', 'fuzzy_skin_mode', 'fuzzy_skin_noise_type'])
		);
	});
	it('ajout d’une liste : valeur du process actif', () => {
		expect(addOverride({}, 'sparse_infill_pattern', { sparse_infill_pattern: 'gyroid' })).toEqual({
			sparse_infill_pattern: 'gyroid'
		});
	});
	it('aucune clé machine (MachineEnvelopeConfig) ni doublon', () => {
		const keys = allowedKeys(false);
		expect(new Set(keys).size).toBe(keys.length);
		expect(keys.some((k) => k.startsWith('machine_') || k.startsWith('input_shaping'))).toBe(false);
		expect(REGION_KEYS.length + OBJECT_KEYS.length).toBe(155 + 128);
	});
});
