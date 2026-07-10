// Tests des maths de transformation (T052) : bornes d'échelle, normalisation
// d'angle, et pose à plat (la normale choisie doit finir vers le bas -Z).
import { describe, expect, test } from 'bun:test';
import {
	clampScale,
	normalizeAngle,
	uniformScale,
	layFlatRotation,
	rotateVectorByEulerDeg,
	IDENTITY,
	MIN_SCALE
} from './transform';

type Vec3 = [number, number, number];

function expectClose(a: Vec3, b: Vec3, eps = 1e-4) {
	for (let i = 0; i < 3; i++) expect(Math.abs(a[i] - b[i])).toBeLessThan(eps);
}

describe('clampScale', () => {
	test('borne les valeurs nulles/négatives/non finies', () => {
		expect(clampScale(2)).toBe(2);
		expect(clampScale(0)).toBe(MIN_SCALE);
		expect(clampScale(-1)).toBe(MIN_SCALE);
		expect(clampScale(NaN)).toBe(MIN_SCALE);
	});
});

describe('normalizeAngle', () => {
	test('ramène dans (-180, 180]', () => {
		expect(normalizeAngle(0)).toBe(0);
		expect(normalizeAngle(190)).toBe(-170);
		expect(normalizeAngle(-190)).toBe(170);
		expect(normalizeAngle(360)).toBe(0);
		expect(normalizeAngle(540)).toBe(180);
	});
});

describe('uniformScale', () => {
	test('multiplie les trois axes et borne', () => {
		expect(uniformScale(IDENTITY, 2).scale).toEqual([2, 2, 2]);
		expect(uniformScale(IDENTITY, 0).scale).toEqual([MIN_SCALE, MIN_SCALE, MIN_SCALE]);
	});
});

describe('layFlatRotation', () => {
	test('une normale déjà vers le bas ne tourne pas', () => {
		const r = layFlatRotation([0, 0, -1]);
		expectClose(rotateVectorByEulerDeg(r, [0, 0, -1]), [0, 0, -1]);
	});

	test('amène une normale +Z vers le bas', () => {
		const r = layFlatRotation([0, 0, 1]);
		expectClose(rotateVectorByEulerDeg(r, [0, 0, 1]), [0, 0, -1]);
	});

	test('amène une normale oblique vers le bas', () => {
		const n: Vec3 = [1, 1, 1];
		const r = layFlatRotation(n);
		const len = Math.hypot(...n);
		const unit: Vec3 = [n[0] / len, n[1] / len, n[2] / len];
		expectClose(rotateVectorByEulerDeg(r, unit), [0, 0, -1]);
	});

	test('amène une normale +X vers le bas', () => {
		const r = layFlatRotation([1, 0, 0]);
		expectClose(rotateVectorByEulerDeg(r, [1, 0, 0]), [0, 0, -1]);
	});
});
