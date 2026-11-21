import { describe, expect, test } from 'vitest';
import { clampFpsCap, DEFAULT_PREFERENCES, parsePreferences } from './preferences';

describe('préférences phase 34 (presets filament groupés, graphismes)', () => {
	test('défauts : regroupement « Aucun » (Orca group_filament_presets = 1), MSAA actif, sans plafond', () => {
		expect(DEFAULT_PREFERENCES.groupFilamentPresets).toBe('none');
		expect(parsePreferences(null).graphics).toMatchObject({
			antialias: true,
			fpsCap: 0,
			showFps: false
		});
	});
	test('lecture tolérante et bornes du plafond d’images/s', () => {
		const p = parsePreferences(
			JSON.stringify({
				groupFilamentPresets: 'vendor',
				graphics: { antialias: false, fpsCap: 5, showFps: true }
			})
		);
		expect(p.groupFilamentPresets).toBe('vendor');
		expect(p.graphics).toMatchObject({ antialias: false, fpsCap: 10, showFps: true });
		expect(
			parsePreferences(JSON.stringify({ groupFilamentPresets: 'x', graphics: 3 })).graphics.fpsCap
		).toBe(0);
		expect(
			parsePreferences(JSON.stringify({ groupFilamentPresets: 'x' })).groupFilamentPresets
		).toBe('none');
		expect(clampFpsCap(-1)).toBe(0);
		expect(clampFpsCap(1000)).toBe(240);
		expect(clampFpsCap(59.6)).toBe(60);
	});
});
