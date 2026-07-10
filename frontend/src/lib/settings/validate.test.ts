// Validation bornes/enums (T043) contre des paramètres réels du registre.
import { describe, expect, it } from 'bun:test';

import { PARAMS } from '../../generated/params';
import { validateValue } from './validate';

describe('validateValue', () => {
	it('respecte les bornes d’un entier (wall_loops 0..1000)', () => {
		const def = PARAMS['wall_loops'];
		expect(validateValue(def, 2).ok).toBe(true);
		expect(validateValue(def, -1).ok).toBe(false);
		expect(validateValue(def, 1001).ok).toBe(false);
		expect(validateValue(def, 2.5).ok).toBe(false); // entier attendu
	});

	it('respecte le minimum d’un flottant sans maximum (layer_height ≥ 0)', () => {
		const def = PARAMS['layer_height'];
		expect(validateValue(def, 0.2).ok).toBe(true);
		expect(validateValue(def, -0.1).ok).toBe(false);
		expect(validateValue(def, 999).ok).toBe(true); // pas de max
	});

	it('borne un pourcentage (accel_to_decel_factor 1..100)', () => {
		const def = PARAMS['accel_to_decel_factor'];
		expect(validateValue(def, 50).ok).toBe(true);
		expect(validateValue(def, 0).ok).toBe(false);
		expect(validateValue(def, 150).ok).toBe(false);
	});

	it('exige une valeur connue pour un enum fermé (bed_temperature_formula)', () => {
		const def = PARAMS['bed_temperature_formula'];
		expect(validateValue(def, 'by_first_filament').ok).toBe(true);
		expect(validateValue(def, 'inconnu').ok).toBe(false);
	});

	it('exige un booléen pour un coBool', () => {
		const def = PARAMS['accel_to_decel_enable'];
		expect(validateValue(def, true).ok).toBe(true);
		expect(validateValue(def, 'oui').ok).toBe(false);
	});

	it('laisse passer les chaînes libres', () => {
		const def = PARAMS['machine_start_gcode'];
		expect(validateValue(def, 'G28').ok).toBe(true);
	});
});
