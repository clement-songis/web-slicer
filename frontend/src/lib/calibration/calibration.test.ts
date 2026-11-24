import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { setLocale } from '#lib/i18n';

// Messages attendus en français (interface traduite).
beforeAll(() => setLocale('fr'));
afterAll(() => setLocale('en'));
import {
	DEFAULTS,
	pressureAdvanceTowerHeight,
	validatePressureAdvance,
	validateTemperature
} from './calibration';

describe('dialogues température / PA', () => {
	it('valeurs par défaut d’Orca valides', () => {
		expect(validateTemperature(DEFAULTS.temperature)).toBeNull();
		expect(validatePressureAdvance(DEFAULTS.pressure_advance)).toBeNull();
	});
	it('hauteur de tour PA : ceil((fin − début)/pas) + 1', () => {
		expect(pressureAdvanceTowerHeight({ start: 0, end: 0.1, step: 0.002 })).toBe(51);
	});
	it('validation des paramètres', () => {
		expect(validateTemperature({ start: 190, end: 230 })).toMatch(/supérieure/);
		expect(validateTemperature({ start: 400, end: 190 })).toMatch(/plage/);
		expect(validatePressureAdvance({ start: 0, end: 1, step: 0.001 })).toMatch(/trop haute/);
		expect(validatePressureAdvance({ start: 0, end: 0.1, step: 0 })).toMatch(/pas/);
	});
});
