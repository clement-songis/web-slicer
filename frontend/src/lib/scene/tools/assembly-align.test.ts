import { describe, expect, it } from 'vitest';
import {
	alignFaces,
	alignPoints,
	quatFromEulerDeg,
	rotate,
	worldFaceCenter,
	worldTriangleNormal
} from './assembly-align';
import type { SceneMesh } from '../mesh';

/** Cube de côté 20 centré à l'origine ; `split` : sommets dupliqués par triangle. */
function cube(split = false): SceneMesh {
	const v = [
		[-10, -10, -10],
		[10, -10, -10],
		[10, 10, -10],
		[-10, 10, -10],
		[-10, -10, 10],
		[10, -10, 10],
		[10, 10, 10],
		[-10, 10, 10]
	];
	const f = [
		[0, 2, 1],
		[0, 3, 2],
		[4, 5, 6],
		[4, 6, 7],
		[0, 1, 5],
		[0, 5, 4],
		[1, 2, 6],
		[1, 6, 5],
		[2, 3, 7],
		[2, 7, 6],
		[3, 0, 4],
		[3, 4, 7]
	];
	const tris = split ? f.flatMap((t) => t.map((i) => v[i])) : v;
	const indices = split ? f.flatMap((_, t) => [t * 3, t * 3 + 1, t * 3 + 2]) : f.flat();
	const positions = new Float32Array(tris.flat());
	return {
		positions,
		normals: new Float32Array(positions.length),
		indices: new Uint32Array(indices)
	};
}

describe('Assembly (face à face / point à point)', () => {
	it('point à point : translation seule', () => {
		expect(alignPoints([10, 0, 5], [12, 1, 5], [40, 1, 5])).toEqual([38, 0, 5]);
	});
	it('face à face : la face +X de B vient contre la face +X de A, normales opposées', () => {
		// A : face +X en x = 10, normale +X. B (centre 50,0,0) : face +X cliquée en x = 55.
		const out = alignFaces(
			{ position: [50, 0, 0], rotation: [0, 0, 0] },
			[55, 0, 0],
			[1, 0, 0],
			[10, 0, 0],
			[1, 0, 0]
		);
		// B tourné de 180° (normale → −X) et son point cliqué posé en (10, 0, 0).
		const n = rotate(quatFromEulerDeg(out.rotation), [1, 0, 0]);
		expect(n[0]).toBeCloseTo(-1, 6);
		const picked = rotate(quatFromEulerDeg(out.rotation), [5, 0, 0]);
		expect(out.position[0] + picked[0]).toBeCloseTo(10, 6);
		expect(out.position[1] + picked[1]).toBeCloseTo(0, 6);
	});
	it('centre de face : face plane entière, pas le triangle ni le point cliqué', () => {
		for (const split of [false, true]) {
			const m = cube(split);
			// Triangle 3 : face +Z ; objet en (50, 0, 10).
			const c = worldFaceCenter(m, 3, [50, 0, 10], [0, 0, 0], [1, 1, 1]);
			expect(c[0]).toBeCloseTo(50, 6);
			expect(c[1]).toBeCloseTo(0, 6);
			expect(c[2]).toBeCloseTo(20, 6);
		}
	});
	it('face à face sur les centres de faces : cubes de 20 mm en contact, centres à 20 mm', () => {
		const m = cube(true);
		const a = {
			position: [0, 0, 10] as [number, number, number],
			rotation: [0, 0, 0] as [number, number, number]
		};
		const b = {
			position: [40, 15, 10] as [number, number, number],
			rotation: [0, 0, 30] as [number, number, number]
		};
		const one: [number, number, number] = [1, 1, 1];
		// A : face +Z (tri 2) ; B : face +X (tri 6).
		const out = alignFaces(
			b,
			worldFaceCenter(m, 6, b.position, b.rotation, one),
			worldTriangleNormal(m, 6, b.rotation, one),
			worldFaceCenter(m, 2, a.position, a.rotation, one),
			worldTriangleNormal(m, 2, a.rotation, one)
		);
		expect(Math.hypot(out.position[0], out.position[1], out.position[2] - 10)).toBeCloseTo(20, 4);
		expect(out.position[2]).toBeCloseTo(30, 4);
	});
	it('quatFromEulerDeg suit la convention XYZ de three.js', () => {
		const v = rotate(quatFromEulerDeg([0, 0, 90]), [1, 0, 0]);
		expect(v[1]).toBeCloseTo(1, 6);
	});
});
