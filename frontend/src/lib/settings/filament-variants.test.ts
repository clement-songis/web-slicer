import { afterAll, beforeAll, describe, expect, test } from 'vitest';
import { setLocale } from '$lib/i18n';

beforeAll(() => setLocale('fr'));
afterAll(() => setLocale('en'));
import { isVariantOption, variantLabels, variantSlot } from './filament-variants';

describe('variantes de buse du filament (Orca TabFilament m_variant_combo)', () => {
	test('libellés « entraînement : débit » (generate_extruder_options)', () => {
		expect(variantLabels(['Direct Drive Standard', 'Direct Drive High Flow'])).toEqual([
			'Entraînement direct : Standard',
			'Entraînement direct : Haut débit'
		]);
		expect(variantLabels(['Bowden Standard', 'Hybrid'])).toEqual(['Bowden : Standard', 'Hybrid']);
		expect(variantLabels(undefined)).toEqual([]);
	});
	test('options par variante = filament_options_with_variant', () => {
		expect(isVariantOption('nozzle_temperature')).toBe(true);
		expect(isVariantOption('filament_max_volumetric_speed')).toBe(true);
		expect(isVariantOption('filament_type')).toBe(false);
	});
	test('index édité : variante si l’option en a une valeur, sinon 0', () => {
		expect(variantSlot('nozzle_temperature', [220, 230], 1)).toBe(1);
		expect(variantSlot('nozzle_temperature', [220], 1)).toBe(0);
		expect(variantSlot('filament_type', ['PLA', 'PLA'], 1)).toBe(0);
		expect(variantSlot('nozzle_temperature', [220, 230], 0)).toBe(0);
	});
});
