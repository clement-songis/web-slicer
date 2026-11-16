import { describe, expect, it } from 'vitest';
import { flattenPlanes, flattenRotation } from './flatten';
import type { SceneMesh } from '../mesh';

/** Cube de côté `s` centré sur l'origine (8 sommets, 12 triangles). */
function cube(s: number): SceneMesh {
	const h = s / 2;
	const positions: number[] = [];
	for (let i = 0; i < 8; i++) positions.push(i & 1 ? h : -h, i & 2 ? h : -h, i & 4 ? h : -h);
	const quads = [
		[0, 2, 3, 1],
		[4, 5, 7, 6],
		[0, 1, 5, 4],
		[2, 6, 7, 3],
		[0, 4, 6, 2],
		[1, 3, 7, 5]
	];
	const indices = quads.flatMap(([a, b, c, d]) => [a, b, c, a, c, d]);
	return {
		positions: new Float32Array(positions),
		normals: new Float32Array(0),
		indices: new Uint32Array(indices)
	};
}

describe('gizmo Flatten (plans de l’enveloppe convexe)', () => {
	it('cube : 6 faces candidates de 400 mm², normales des axes', () => {
		const planes = flattenPlanes(cube(20));
		expect(planes).toHaveLength(6);
		for (const p of planes) {
			expect(p.area).toBeCloseTo(400, 3);
			expect(Math.max(...p.normal.map(Math.abs))).toBeCloseTo(1, 6);
		}
	});
	it('écarte les faces < 5 mm² (cube de 2 mm)', () => {
		expect(flattenPlanes(cube(2))).toHaveLength(0);
	});
	it('applique l’échelle de l’instance (aires en mm² monde)', () => {
		const areas = flattenPlanes(cube(10), [0, 0, 0], [2, 1, 1]).map((p) => Math.round(p.area));
		expect(areas.sort((a, b) => a - b)).toEqual([100, 100, 200, 200, 200, 200]);
	});
	it('la rotation pose la face choisie à plat (normale → −Z)', () => {
		const planes = flattenPlanes(cube(20), [30, 0, 0]);
		const side = planes.find((p) => Math.abs(p.normal[0]) > 0.99)!;
		const r = flattenRotation([30, 0, 0], side.normal);
		// Après rotation, la face +X/−X est en bas : ré-évaluation des plans.
		const after = flattenPlanes(cube(20), r);
		expect(after.some((p) => p.normal[2] < -0.999999)).toBe(true);
	});
});
