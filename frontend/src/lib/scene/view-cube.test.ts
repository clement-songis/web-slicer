import { describe, expect, test } from 'vitest';
import { CUBE_FACES, cameraBasis, cubeTransform, faceTransform, facingFace } from './view-cube';

type V = [number, number, number];
const dot = (a: V, b: V) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];

describe('navigateur 3D (cube de vues)', () => {
	test('repère caméra : vue de face (regarde +Y) → droite +X, haut +Z', () => {
		const b = cameraBasis([0, 1, 0]);
		expect(b.right.map((n) => Math.round(n))).toEqual([1, 0, 0]);
		expect(b.up.map((n) => Math.round(n))).toEqual([0, 0, 1]);
	});

	test('vues de dessus / dessous : repli sur +X, haut ±Y', () => {
		expect(cameraBasis([0, 0, -1]).up.map((n) => Math.round(n) + 0)).toEqual([0, 1, 0]);
		expect(cameraBasis([0, 0, 1]).up.map((n) => Math.round(n) + 0)).toEqual([0, -1, 0]);
	});

	test('chaque face, vue de face, montre son texte à l’endroit (droite → écran droite, bas → écran bas)', () => {
		const views: Record<string, V> = {
			front: [0, 1, 0],
			rear: [0, -1, 0],
			left: [1, 0, 0],
			right: [-1, 0, 0],
			top: [0, 0, -1],
			bottom: [0, 0, 1]
		};
		for (const face of CUBE_FACES) {
			const b = cameraBasis(views[face.view]);
			expect(dot(face.right, b.right)).toBeCloseTo(1);
			expect(dot(face.down, b.up)).toBeCloseTo(-1);
			expect(facingFace(views[face.view]).view).toBe(face.view);
		}
	});

	test('matrices CSS sérialisées (colonnes) sans bruit flottant', () => {
		expect(cubeTransform([0, 1, 0])).toBe(
			'matrix3d(1, 0, 0, 0, 0, 0, -1, 0, 0, -1, 0, 0, 0, 0, 0, 1)'
		);
		expect(faceTransform(CUBE_FACES[4], 28)).toBe(
			'matrix3d(1, 0, 0, 0, 0, -1, 0, 0, 0, 0, 1, 0, 0, 0, 28, 1)'
		);
	});
});
