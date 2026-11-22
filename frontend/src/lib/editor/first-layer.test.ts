// Phase 46 (audit 44 PERF-a) : objet nommé dans l'échec « couche initiale vide ».
import { describe, expect, it } from 'vitest';
import { emptyFirstLayerSuspects, isEmptyFirstLayerError, sectionExtent } from './first-layer';
import type { SceneMesh } from '$lib/scene/mesh';

function mesh(points: number[][], tris: number[][]): SceneMesh {
	return {
		positions: new Float32Array(points.flat()),
		normals: new Float32Array(points.length * 3),
		indices: new Uint32Array(tris.flat())
	} as SceneMesh;
}
// Pyramide de base 20 × 20, sommet en haut (z = 10) ou en bas (retournée).
const up = mesh(
	[
		[-10, -10, 0],
		[10, -10, 0],
		[10, 10, 0],
		[-10, 10, 0],
		[0, 0, 10]
	],
	[
		[0, 1, 4],
		[1, 2, 4],
		[2, 3, 4],
		[3, 0, 4],
		[0, 2, 1],
		[0, 3, 2]
	]
);

describe('first-layer', () => {
	it('reconnaît le message (anglais / français)', () => {
		expect(
			isEmptyFirstLayerError(
				"tranchage impossible : One object has an empty first layer and can't be printed."
			)
		).toBe(true);
		expect(isEmptyFirstLayerError('Un objet a une couche initiale vide')).toBe(true);
		expect(isEmptyFirstLayerError('autre')).toBe(false);
	});
	it('section à mi-hauteur de la première couche', () => {
		const s = sectionExtent(up, 0.1)!;
		expect(s.width).toBeCloseTo(19.8, 3);
		const tip = sectionExtent(up, 0.1, [180, 0, 0])!;
		expect(tip.width).toBeCloseTo(0.2, 3);
	});
	it('nomme seulement l’objet posé sur sa pointe', () => {
		expect(
			emptyFirstLayerSuspects(
				[
					{ name: 'pyramide.stl', mesh: up },
					{ name: 'cone_down.stl', mesh: up, rotation: [180, 0, 0] }
				],
				0.2,
				0.42
			)
		).toEqual(['cone_down.stl']);
	});
});
