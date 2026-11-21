import { describe, expect, test } from 'vitest';
import { groupCutScale, pieceWorldBox, selectionCenter, uniformFactor } from './cut-group-scale';
import type { SceneMesh } from './mesh';

/** Pavé centré de dimensions w × d × h. */
function box(w: number, d: number, h: number): SceneMesh {
	const p: number[] = [];
	for (const x of [-w / 2, w / 2])
		for (const y of [-d / 2, d / 2]) for (const z of [-h / 2, h / 2]) p.push(x, y, z);
	return {
		positions: new Float32Array(p),
		normals: new Float32Array(p.length),
		indices: new Uint32Array()
	};
}

// Cube de 20 mm coupé à z = 10 : bas (z 0..10) et haut retourné posé à côté.
const low = {
	id: 'low',
	mesh: box(20, 20, 10),
	position: [0, 0, 5] as [number, number, number],
	rotation: [0, 0, 0] as [number, number, number],
	scale: [1, 1, 1] as [number, number, number]
};
const up = {
	id: 'up',
	mesh: box(20, 20, 10),
	position: [30, 0, 5] as [number, number, number],
	rotation: [180, 0, 0] as [number, number, number],
	scale: [1, 1, 1] as [number, number, number]
};

describe('échelle groupée des pièces coupées (Selection::scale_and_translate)', () => {
	test('boîte monde et centre de la sélection', () => {
		expect(pieceWorldBox(low)).toEqual({ min: [-10, -10, 0], max: [10, 10, 10] });
		const b = pieceWorldBox(up);
		expect(b.min.map((v) => Math.round(v * 1e6) / 1e6 + 0)).toEqual([20, -10, 0]);
		expect(selectionCenter([low, up])!.map((v) => Math.round(v * 1e6) / 1e6 + 0)).toEqual([
			15, 0, 5
		]);
	});
	test('facteur uniforme : axe le plus modifié', () => {
		expect(uniformFactor([1, 1, 1], [2, 1, 1])).toBe(2);
		expect(uniformFactor([2, 2, 2], [2, 2, 1])).toBe(0.5);
		expect(uniformFactor([1, 1, 1], [1, 1, 1])).toBe(1);
	});
	test('k = 2 autour du centre commun, puis reposées sur le plateau', () => {
		const out = groupCutScale([low, up], 2);
		const r = (v: number[]) => v.map((x) => Math.round(x * 1e6) / 1e6 + 0);
		expect(out.map((o) => o.id)).toEqual(['low', 'up']);
		expect(out[0].scale).toEqual([2, 2, 2]);
		expect(out[1].scale).toEqual([2, 2, 2]);
		// Boîte commune x −10…40, centre 15 : 15 + 2·(0 − 15) = −15 ;
		// 15 + 2·(30 − 15) = 45 → écart entre pièces doublé.
		expect(r(out[0].position)).toEqual([-15, 0, 10]);
		expect(r(out[1].position)).toEqual([45, 0, 10]);
		// Sans pose : z = 5 + 2·(5 − 5).
		expect(r(groupCutScale([low, up], 2, { dropToBed: false })[0].position)).toEqual([-15, 0, 5]);
	});
	test('facteur invalide ou sélection vide → aucun changement', () => {
		expect(groupCutScale([low], 0)).toEqual([]);
		expect(groupCutScale([], 2)).toEqual([]);
	});
});
