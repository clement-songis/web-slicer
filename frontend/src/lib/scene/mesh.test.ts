// Tests du recentrage de maillage (fix gizmo : origine locale = centre visuel).
import { describe, expect, test } from 'vitest';
import { centerMesh, positionForImport, restingZ, type SceneMesh } from './mesh';
import { primitiveMesh } from './primitives';

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

describe('positionForImport', () => {
	test('place un nouvel import au centre XY du plateau sans changer son Z', () => {
		expect(positionForImport([110, 120, 5], [128, 128])).toEqual([128, 128, 5]);
	});

	test('conserve exactement la position sauvegardée lors de la restauration', () => {
		expect(positionForImport([110, 120, 5], [128, 128], [12, 34, 0])).toEqual([12, 34, 0]);
	});
});

describe('restingZ', () => {
	// Boîte centrée de 20 × 10 × 6 mm (demi-tailles 10, 5, 3).
	const box = mesh([-10, -5, -3, 10, 5, 3, -10, 5, 3]);

	test('un maillage centré se pose à sa demi-hauteur, pas à z = 0', () => {
		expect(restingZ(box)).toBe(3);
	});

	test('un maillage dont la base est déjà à z = 0 reste à 0 (pas de -0)', () => {
		expect(Object.is(restingZ(mesh([0, 0, 0, 1, 1, 2, 1, 0, 0])), 0)).toBe(true);
	});

	test('tient compte de l’échelle', () => {
		expect(restingZ(box, [0, 0, 0], [1, 1, 2])).toBe(6);
	});

	test('tient compte de la rotation (Euler XYZ, degrés)', () => {
		// 90° autour de Y : l’étendue X (demi-taille 10) devient verticale.
		expect(restingZ(box, [0, 90, 0])).toBeCloseTo(10, 6);
		// 90° autour de X : l’étendue Y (demi-taille 5) devient verticale.
		expect(restingZ(box, [90, 0, 0])).toBeCloseTo(5, 6);
	});

	test('maillage vide → 0', () => {
		expect(restingZ(mesh([]))).toBe(0);
	});

	test('primitive recentrée : base exactement sur le plateau', () => {
		const { mesh: cube } = centerMesh(primitiveMesh('cube'));
		const z = restingZ(cube);
		expect(z).toBeCloseTo(10, 6); // cube de 20 mm
		let minZ = Infinity;
		for (let i = 2; i < cube.positions.length; i += 3) minZ = Math.min(minZ, cube.positions[i] + z);
		expect(minZ).toBeCloseTo(0, 6);
	});
});
