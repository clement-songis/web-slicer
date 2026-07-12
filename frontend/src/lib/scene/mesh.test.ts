// Tests du recentrage de maillage (fix gizmo : origine locale = centre visuel).
import { describe, expect, test } from 'vitest';
import { centerMesh, type SceneMesh } from './mesh';

function mesh(positions: number[]): SceneMesh {
	return {
		positions: new Float32Array(positions),
		normals: new Float32Array(),
		indices: new Uint32Array(positions.length / 3)
	};
}

describe('centerMesh', () => {
	test('recentre sur le centre de la boîte englobante et renvoie ce centre', () => {
		// Boîte de (100,100,0) à (120,140,10) → centre (110,120,5).
		const { mesh: out, center } = centerMesh(mesh([100, 100, 0, 120, 140, 10, 100, 140, 0]));
		expect(center).toEqual([110, 120, 5]);
		// Chaque sommet est translaté de -center.
		expect([...out.positions.slice(0, 3)]).toEqual([-10, -20, -5]);
		expect([...out.positions.slice(3, 6)]).toEqual([10, 20, 5]);
	});

	test('un maillage déjà centré reste inchangé (centre nul)', () => {
		const { center } = centerMesh(mesh([-5, -5, -5, 5, 5, 5, -5, 5, -5]));
		expect(center).toEqual([0, 0, 0]);
	});

	test('un maillage vide renvoie un centre nul sans lever', () => {
		expect(centerMesh(mesh([])).center).toEqual([0, 0, 0]);
	});

	test('préserve normales et indices (mêmes références)', () => {
		const src = mesh([0, 0, 0, 2, 0, 0, 0, 2, 0]);
		const { mesh: out } = centerMesh(src);
		expect(out.normals).toBe(src.normals);
		expect(out.indices).toBe(src.indices);
	});
});
