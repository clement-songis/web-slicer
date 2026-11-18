import { describe, expect, it } from 'vitest';
import { encodeFacet } from './facets';
import { decodeFacetTriangles, hexToRgb, paintOverlayGeometry } from './paint-overlay';

type V = [number, number, number];
const A: V = [0, 0, 0];
const B: V = [4, 0, 0];
const C: V = [0, 4, 0];

describe('decodeFacetTriangles (TriangleSelector::deserialize)', () => {
	it('triangle entier : une feuille, état 1/2 et états MMU ≥ 3', () => {
		expect(decodeFacetTriangles(encodeFacet(1), A, B, C)).toEqual([
			{ vertices: [A, B, C], state: 1 }
		]);
		expect(decodeFacetTriangles(encodeFacet(5), A, B, C)[0].state).toBe(5);
		expect(decodeFacetTriangles('', A, B, C)).toEqual([]);
		expect(decodeFacetTriangles('zz', A, B, C)).toEqual([]);
	});
	it('scission en 3 (2 côtés) : enfants lus du dernier au premier, comme Orca', () => {
		// Model.cpp (_2_SAME_1_DIFF_COLOR, iso 0) : result0 + result1 + result1 + "2".
		const leaves = decodeFacetTriangles(
			encodeFacet(1) + encodeFacet(2) + encodeFacet(2) + '2',
			A,
			B,
			C
		);
		expect(leaves).toHaveLength(3);
		// Enfant 0 (sommet A) porte result0.
		const withA = leaves.filter((l) => l.vertices.some((v) => v.join() === A.join()));
		expect(withA.map((l) => l.state)).toEqual([1]);
		expect(leaves.filter((l) => l.state === 2)).toHaveLength(2);
	});
	it('scission en 4 : aire totale conservée, feuilles non peintes omises', () => {
		// Racine 3 côtés (code 3), enfants (ordre inverse) : 0, 1, 0, 1.
		const hex = encodeFacet(1) + '0' + encodeFacet(1) + '0' + '3';
		const leaves = decodeFacetTriangles(hex, A, B, C);
		expect(leaves).toHaveLength(2);
		const area = (t: [V, V, V]) =>
			Math.abs(
				(t[1][0] - t[0][0]) * (t[2][1] - t[0][1]) - (t[2][0] - t[0][0]) * (t[1][1] - t[0][1])
			) / 2;
		expect(leaves.reduce((s, l) => s + area(l.vertices), 0)).toBeCloseTo(4);
	});
});

describe('paintOverlayGeometry', () => {
	const mesh = {
		positions: new Float32Array([...A, ...B, ...C]),
		indices: new Uint32Array([0, 1, 2])
	};
	it('un triangle coloré par état, rien pour un état sans couleur', () => {
		const out = paintOverlayGeometry(mesh, { '0': encodeFacet(2) }, (s) =>
			s === 2 ? '#ff0000' : null
		);
		expect(out?.positions).toHaveLength(9);
		expect([...out!.colors.slice(0, 3)]).toEqual([1, 0, 0]);
		expect(paintOverlayGeometry(mesh, { '0': encodeFacet(1) }, () => null)).toBeNull();
		expect(paintOverlayGeometry(mesh, { '7': encodeFacet(1) }, () => '#00ff00')).toBeNull();
		expect(paintOverlayGeometry(mesh, undefined, () => '#00ff00')).toBeNull();
	});
	it('hexToRgb', () => {
		expect(hexToRgb('#00ff00')).toEqual([0, 1, 0]);
		expect(hexToRgb('vert')).toBeNull();
	});
});
