// Tests de l'aide à l'arrangement de plateau (T106).
import { describe, expect, test } from 'vitest';
import { footprint, arrangeItems, applyPlacements } from './arrange';
import type { SceneMesh, SceneObject } from './mesh';

// Maillage minimal (un triangle) aux coordonnées XY données.
function mesh(pts: [number, number, number][]): SceneMesh {
	const positions = new Float32Array(pts.flat());
	return {
		positions,
		normals: new Float32Array(positions.length),
		indices: new Uint32Array([0, 1, 2])
	};
}

describe('arrange', () => {
	test('footprint : étendues XY du maillage', () => {
		const m = mesh([
			[0, 0, 0],
			[10, 4, 2],
			[3, 6, 5]
		]);
		expect(footprint(m)).toEqual({ width: 10, depth: 6 });
	});

	test('footprint : maillage vide → 0×0', () => {
		expect(footprint(mesh([]))).toEqual({ width: 0, depth: 0 });
	});

	test('arrangeItems : id + empreinte par objet', () => {
		const objs: SceneObject[] = [
			{
				id: 'a',
				mesh: mesh([
					[0, 0, 0],
					[2, 0, 0],
					[0, 3, 0]
				])
			},
			{
				id: 'b',
				mesh: mesh([
					[0, 0, 0],
					[5, 0, 0],
					[0, 1, 0]
				])
			}
		];
		expect(arrangeItems(objs)).toEqual([
			{ id: 'a', width: 2, depth: 3 },
			{ id: 'b', width: 5, depth: 1 }
		]);
	});

	test('applyPlacements : position XY appliquée, Z préservée, non-placés inchangés', () => {
		const objs: SceneObject[] = [
			{ id: 'a', mesh: mesh([[0, 0, 0]]), position: [1, 1, 7] },
			{ id: 'b', mesh: mesh([[0, 0, 0]]) }
		];
		const out = applyPlacements(objs, [{ id: 'a', x: 20, y: 30 }]);
		expect(out[0].position).toEqual([20, 30, 7]);
		expect(out[1]).toBe(objs[1]);
	});
});
