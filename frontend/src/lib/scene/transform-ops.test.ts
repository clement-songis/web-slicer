// Tests des opérations de transformation (miroir, ajustement au volume).
import { describe, expect, test } from 'vitest';
import { fitScaleFactor, mirrorScale, scaleBy, worldSize } from './transform-ops';
import type { SceneMesh } from './mesh';

/** Pavé centré de dimensions sx × sy × sz (8 sommets suffisent aux bornes). */
function box(sx: number, sy: number, sz: number): SceneMesh {
	const positions: number[] = [];
	for (const x of [-sx / 2, sx / 2])
		for (const y of [-sy / 2, sy / 2]) for (const z of [-sz / 2, sz / 2]) positions.push(x, y, z);
	return {
		positions: new Float32Array(positions),
		normals: new Float32Array(positions.length),
		indices: new Uint32Array([0, 1, 2])
	};
}

describe('transform-ops', () => {
	test('mirrorScale inverse uniquement l’axe demandé, sans muter l’entrée', () => {
		const s: [number, number, number] = [2, 1, 1];
		expect(mirrorScale(s, 0)).toEqual([-2, 1, 1]);
		expect(mirrorScale(s, 1)).toEqual([2, -1, 1]);
		expect(mirrorScale(undefined, 2)).toEqual([1, 1, -1]);
		expect(s).toEqual([2, 1, 1]);
		// Deux miroirs successifs = identité.
		expect(mirrorScale(mirrorScale(s, 0), 0)).toEqual(s);
	});

	test('worldSize tient compte de l’échelle (signe ignoré) et de la rotation', () => {
		expect(worldSize(box(10, 20, 30))).toEqual([10, 20, 30]);
		expect(worldSize(box(10, 20, 30), [0, 0, 0], [-2, 1, 1])).toEqual([20, 20, 30]);
		const r = worldSize(box(10, 20, 30), [0, 0, 90]);
		expect(r[0]).toBeCloseTo(20);
		expect(r[1]).toBeCloseTo(10);
		expect(r[2]).toBeCloseTo(30);
		expect(
			worldSize({
				positions: new Float32Array(),
				normals: new Float32Array(),
				indices: new Uint32Array()
			})
		).toEqual([0, 0, 0]);
	});

	test('fitScaleFactor : plus petit rapport volume/objet, agrandit ou réduit', () => {
		const bed = { width: 200, depth: 200, height: 100 };
		expect(fitScaleFactor([20, 20, 20], bed)).toBe(5);
		expect(fitScaleFactor([400, 100, 10], bed)).toBe(0.5);
		// Objet plat : l'axe nul est ignoré.
		expect(fitScaleFactor([50, 25, 0], bed)).toBe(4);
		expect(fitScaleFactor([0, 0, 0], bed)).toBeNull();
		expect(fitScaleFactor([10, 10, 10], { width: 0, depth: 200, height: 100 })).toBeNull();
	});

	test('scaleBy conserve les signes de miroir', () => {
		expect(scaleBy([-1, 2, 1], 2)).toEqual([-2, 4, 2]);
		expect(scaleBy(undefined, 0.5)).toEqual([0.5, 0.5, 0.5]);
	});
});
