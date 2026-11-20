import { describe, expect, test } from 'vitest';
import { DEFAULT_PREFERENCES, clampDracoBits, parsePreferences } from './preferences';

describe('préférences phase 37 (navigateur 3D, axes, Draco)', () => {
	test('défauts d’Orca : navigateur et axes affichés, Draco sans perte', () => {
		expect(DEFAULT_PREFERENCES.show3dNavigator).toBe(true);
		expect(DEFAULT_PREFERENCES.showAxes).toBe(true);
		expect(DEFAULT_PREFERENCES.dracoBits).toBe(0);
	});

	test('bornes de create_item_draco : 0, 8–30', () => {
		expect([-3, 0, 1, 7, 8, 16.4, 30, 99, NaN].map(clampDracoBits)).toEqual([
			0, 0, 8, 8, 8, 16, 30, 30, 0
		]);
	});

	test('lecture tolérante', () => {
		const p = parsePreferences(
			JSON.stringify({ show3dNavigator: false, showAxes: 'x', dracoBits: 5 })
		);
		expect(p.show3dNavigator).toBe(false);
		expect(p.showAxes).toBe(true);
		expect(p.dracoBits).toBe(8);
	});
});
