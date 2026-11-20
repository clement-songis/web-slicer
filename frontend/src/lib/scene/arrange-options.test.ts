import { describe, expect, it } from 'vitest';
import { ARRANGE_DEFAULTS } from '$lib/settings/preferences';
import {
	clampSpacing,
	effectiveOptions,
	optionsBody,
	resetOptions,
	spacingFloor
} from './arrange-options';
import type { ArrangeOptionsInfo } from '$lib/api/types';

const info = (over: Partial<ArrangeOptionsInfo> = {}): ArrangeOptionsInfo => ({
	defaults: {
		spacing: 0,
		enable_rotation: false,
		allow_multi_materials_on_same_plate: true,
		avoid_extrusion_cali_region: true,
		align_to_y_axis: false
	},
	is_seq_print: false,
	min_spacing: 6,
	show_avoid_extrusion_cali_region: false,
	...over
});

describe('options d’arrangement (lot 34)', () => {
	it('hors séquentiel : 0 = automatique, borne 0–100', () => {
		expect(spacingFloor(info())).toBe(0);
		expect(clampSpacing(0, info())).toBe(0);
		expect(clampSpacing(12.5, info())).toBe(12.5);
		expect(clampSpacing(250, info())).toBe(100);
		expect(clampSpacing(-3, info())).toBe(0);
	});

	it('séquentiel : espacement forcé à min_spacing (slider d’Orca)', () => {
		const seq = info({ is_seq_print: true, min_spacing: 25 });
		expect(spacingFloor(seq)).toBe(25);
		expect(clampSpacing(0, seq)).toBe(25);
		expect(clampSpacing(10, seq)).toBe(25);
		expect(clampSpacing(40, seq)).toBe(40);
	});

	it('Réinitialiser : valeurs du serveur (alignement Y = structure i3)', () => {
		const i3 = info({
			defaults: {
				spacing: 0,
				enable_rotation: false,
				allow_multi_materials_on_same_plate: true,
				avoid_extrusion_cali_region: true,
				align_to_y_axis: true
			}
		});
		const reset = resetOptions(i3);
		expect(reset).toEqual({ ...ARRANGE_DEFAULTS, alignToYAxis: true });
		// En séquentiel, le Reset d'Orca borne aussi l'espacement.
		expect(resetOptions(info({ is_seq_print: true, min_spacing: 25 })).spacing).toBe(25);
	});

	it('corps options : alignement Y éteint si rotation automatique (Orca)', () => {
		expect(
			optionsBody({ ...ARRANGE_DEFAULTS, enableRotation: true, alignToYAxis: true })
		).toMatchObject({ align_to_y_axis: false, enable_rotation: true });
		expect(
			optionsBody({ ...ARRANGE_DEFAULTS, enableRotation: false, alignToYAxis: true })
		).toMatchObject({ align_to_y_axis: true });
	});

	it('options mémorisées ramenées aux bornes du serveur', () => {
		const merged = effectiveOptions(
			{ ...ARRANGE_DEFAULTS, spacing: 2 },
			info({ is_seq_print: true, min_spacing: 25 })
		);
		expect(merged.spacing).toBe(25);
	});
});
