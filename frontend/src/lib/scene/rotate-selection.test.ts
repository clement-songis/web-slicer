import { describe, expect, test } from 'vitest';
import { rotateSelectionZ } from './rotate-selection';
import { rotateEulerAboutWorldZ, rotateVectorByEulerDeg } from './transform';

const close = (a: number[], b: number[]) => a.forEach((v, i) => expect(v).toBeCloseTo(b[i], 5));

describe('rotation ±45° de la sélection (Page préc./suiv.)', () => {
	test('orientation à plat : seule la composante Z change', () => {
		expect(rotateEulerAboutWorldZ([0, 0, 0], 45)).toEqual([0, 0, 45]);
		expect(rotateEulerAboutWorldZ([0, 0, 170], 45)[2]).toBeCloseTo(-145, 5);
	});

	test('objet basculé : rotation autour du Z du plateau, pas de son Z local', () => {
		const start: [number, number, number] = [90, 0, 0];
		const next = rotateEulerAboutWorldZ(start, 45);
		// Un vecteur quelconque tourné par R' = Rz(45°)·R.
		const v: [number, number, number] = [1, 2, 3];
		const before = rotateVectorByEulerDeg(start, v);
		const a = Math.PI / 4;
		const expected = [
			before[0] * Math.cos(a) - before[1] * Math.sin(a),
			before[0] * Math.sin(a) + before[1] * Math.cos(a),
			before[2]
		];
		close(rotateVectorByEulerDeg(next, v), expected);
	});

	test('objet seul : tourne sur place ; plusieurs : autour du centre de la sélection', () => {
		const one = rotateSelectionZ([{ id: 'a', position: [10, 20, 5] }], -45);
		expect(one.get('a')!.position).toEqual([10, 20, 5]);
		expect(one.get('a')!.rotation).toEqual([0, 0, -45]);
		const two = rotateSelectionZ(
			[
				{ id: 'a', position: [0, 0, 1] },
				{ id: 'b', position: [20, 0, 1] }
			],
			90
		);
		close(two.get('a')!.position, [10, -10, 1]);
		close(two.get('b')!.position, [10, 10, 1]);
		expect(rotateSelectionZ([], 45).size).toBe(0);
	});
});
