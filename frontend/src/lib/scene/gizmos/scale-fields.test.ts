// Audit 40 B3 : échelle en % et taille en mm, verrou de proportions (Orca).
import { describe, expect, test } from 'vitest';
import { scaleFromPercent, scaleFromSize, scalePercent, scaledSize } from './scale-fields';

describe('échelle en % / taille en mm', () => {
	test('affichage : 1 → 100 %, miroir → -100 %, taille = base × |échelle|', () => {
		expect(scalePercent(1)).toBe(100);
		expect(scalePercent(-1)).toBe(-100);
		expect(scalePercent(1.23456)).toBe(123.46);
		expect(scaledSize([20, 10, 5], [1.5, -1, 2])).toEqual([30, 10, 10]);
	});
	test('150 % sur X, verrou fermé : les trois axes à 150 %', () => {
		expect(scaleFromPercent([1, 1, 1], 0, 150, true)).toEqual([1.5, 1.5, 1.5]);
	});
	test('verrou ouvert : seul l’axe change ; le miroir garde son signe', () => {
		expect(scaleFromPercent([1, -1, 1], 1, 50, false)).toEqual([1, -0.5, 1]);
		expect(scaleFromPercent([2, -1, 1], 0, 100, true)).toEqual([1, -0.5, 0.5]);
	});
	test('taille en mm : 40 mm sur un cube de 20 → 200 % (proportions gardées)', () => {
		expect(scaleFromSize([1, 1, 1], [20, 20, 20], 0, 40, true)).toEqual([2, 2, 2]);
		expect(scaleFromSize([1, 1, 1], [20, 10, 20], 1, 5, false)).toEqual([1, 0.5, 1]);
	});
	test('saisies invalides ignorées', () => {
		expect(scaleFromPercent([1, 1, 1], 0, 0, true)).toBeNull();
		expect(scaleFromPercent([1, 1, 1], 0, Number.NaN, true)).toBeNull();
		expect(scaleFromSize([1, 1, 1], [0, 1, 1], 0, 10, true)).toBeNull();
	});
});
