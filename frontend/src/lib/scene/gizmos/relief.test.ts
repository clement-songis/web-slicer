// Tests des gizmos mesure / oreilles de bord / relief (T057, analyse G1).
import { afterAll, beforeAll, describe, expect, test } from 'vitest';
import { setLocale } from '$lib/i18n';
import { distance, angleDeg, angleBetweenNormals, measurePoints } from './measure';
import { BrimEars } from './brim-ears';
import { defaultEmbossParams, validateEmboss, type EmbossParams } from './emboss';

beforeAll(() => setLocale('fr'));
afterAll(() => setLocale('en'));

describe('measure', () => {
	test('distance et composantes entre deux points', () => {
		expect(distance([0, 0, 0], [3, 4, 0])).toBeCloseTo(5);
		const m = measurePoints([1, 2, 3], [4, 6, 3]);
		expect(m.distance).toBeCloseTo(5);
		expect(m.delta).toEqual([3, 4, 0]);
	});

	test('angle entre vecteurs et normales', () => {
		expect(angleDeg([1, 0, 0], [0, 1, 0])).toBeCloseTo(90);
		expect(angleDeg([1, 0, 0], [1, 0, 0])).toBeCloseTo(0);
		expect(angleBetweenNormals([0, 0, 1], [0, 0, -1])).toBeCloseTo(180);
	});

	test('angle nul pour un vecteur nul', () => {
		expect(angleDeg([0, 0, 0], [1, 0, 0])).toBe(0);
	});
});

describe('BrimEars', () => {
	test('ajoute, supprime et compte les points', () => {
		const ears = new BrimEars();
		ears.add(1, 2);
		ears.add(3, 4);
		expect(ears.count).toBe(2);
		ears.removeAt(0);
		expect(ears.list()[0]).toMatchObject({ x: 3, y: 4 });
		ears.removeAt(99); // hors limites : ignoré
		expect(ears.count).toBe(1);
	});

	test('round-trip de sérialisation', () => {
		const ears = new BrimEars();
		ears.add(1.5, -2.5);
		const doc = ears.serialize();
		// Phase 47 (EARS-a) : oreilles dans le repère objet + rayon de tête.
		expect(doc).toEqual({ points: [[1.5, -2.5, 8]], ears: [[1.5, -2.5, 0, 4]] });
		const back = BrimEars.deserialize(doc);
		expect(back.list()[0]).toEqual({ x: 1.5, y: -2.5, z: 0, r: 4 });
	});
});

describe('emboss', () => {
	test('paramètres par défaut valides', () => {
		expect(validateEmboss(defaultEmbossParams())).toBeNull();
	});

	test('rejette texte vide, SVG vide, profondeur nulle', () => {
		const base = defaultEmbossParams();
		expect(validateEmboss({ ...base, text: '  ' })).toContain('texte');
		expect(validateEmboss({ ...base, depth: 0 })).toContain('profondeur');
		const svg: EmbossParams = { ...base, source: 'svg', svg: '' };
		expect(validateEmboss(svg)).toContain('SVG');
	});
});
