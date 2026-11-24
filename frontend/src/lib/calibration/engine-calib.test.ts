import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { setLocale } from '#lib/i18n';

// Messages attendus en français (interface traduite).
beforeAll(() => setLocale('fr'));
afterAll(() => setLocale('en'));
import { ENGINE_CALIB_DEFAULTS, parseNumberList, testModels, validateRange } from './engine-calib';

describe('dialogues des calibrations moteur', () => {
	it('valeurs par défaut d’Orca valides', () => {
		for (const [kind, p] of Object.entries(ENGINE_CALIB_DEFAULTS))
			expect(validateRange(p, kind as keyof typeof ENGINE_CALIB_DEFAULTS)).toBeNull();
	});
	it('validation : plage, pas, valeur fixe, plage Y', () => {
		expect(validateRange({ start: 1, end: 1, step: 1 })).not.toBeNull();
		expect(validateRange({ start: 0, end: 1, step: 0 })).not.toBeNull();
		expect(validateRange({ start: 0, end: 2, step: 1, fixed: 30 }, 'input_shaping_damp')).toMatch(
			/entre 0 et 1/
		);
		expect(validateRange({ start: 15, end: 110, step: 1 }, 'input_shaping_freq')).toMatch(/fixe/);
		expect(
			validateRange(
				{ start: 15, end: 110, step: 1, fixed: 0.15, startY: 50, endY: 40 },
				'input_shaping_freq'
			)
		).toMatch(/Y/);
	});
	it('listes de vitesses / accélérations', () => {
		expect(parseNumberList('100, 200;300')).toEqual([100, 200, 300]);
		expect(parseNumberList('100, abc')).toBeNull();
		expect(parseNumberList('')).toEqual([]);
	});
	it('modèles de tour : SCV-V2 pour les virages seulement', () => {
		expect(testModels('input_shaping_freq').map((m) => m.id)).toEqual([0, 1]);
		expect(testModels('cornering').map((m) => m.id)).toEqual([0, 1, 2]);
	});
});
