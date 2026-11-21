import { describe, expect, test } from 'vitest';
import type { SceneMesh, SceneObject } from './mesh';
import {
	convexHull2D,
	footprintPolygon,
	overlapDepth,
	overlappingCount
} from './footprint-overlap';

/** Pavé centré (8 sommets, sans triangles : seule l'empreinte compte). */
function box(sx: number, sy: number, sz: number): SceneMesh {
	const pts: number[] = [];
	for (const x of [-sx / 2, sx / 2])
		for (const y of [-sy / 2, sy / 2]) for (const z of [-sz / 2, sz / 2]) pts.push(x, y, z);
	return {
		positions: new Float32Array(pts),
		normals: new Float32Array(0),
		indices: new Uint32Array(0)
	} as unknown as SceneMesh;
}

const obj = (
	id: string,
	mesh: SceneMesh,
	position: [number, number, number],
	rotation: [number, number, number] = [0, 0, 0]
): SceneObject => ({ id, mesh, position, rotation });

describe('alerte de superposition : empreintes réelles (phase 38)', () => {
	test('enveloppe 2D et profondeur de recouvrement', () => {
		const sq = convexHull2D([
			[0, 0],
			[1, 0],
			[1, 1],
			[0, 1],
			[0.5, 0.5]
		]);
		expect(sq).toHaveLength(4);
		const shifted = sq.map(([x, y]) => [x + 0.75, y] as [number, number]);
		expect(overlapDepth(sq, shifted)).toBeCloseTo(0.25);
		const apart = sq.map(([x, y]) => [x + 2, y] as [number, number]);
		expect(overlapDepth(sq, apart)).toBeLessThanOrEqual(0);
	});

	test('pièce longue tournée de 90° à 2 mm d’un cube : plus de faux positif', () => {
		// Boîtes XY sans rotation (ancienne alerte) : 40 × 10 → recouvrement de 13 mm ;
		// empreinte réelle 10 × 40 → 2 mm d'écart.
		const a = obj('a', box(40, 10, 10), [100, 100, 0], [0, 0, 90]);
		const b = obj('b', box(20, 20, 20), [117, 100, 0]);
		expect(footprintPolygon(a, [0, 0])).toHaveLength(4);
		expect(overlappingCount([a, b], [0, 0])).toBe(0);
		// Vrai chevauchement toujours signalé.
		const c = obj('c', box(20, 20, 20), [110, 100, 0]);
		expect(overlappingCount([a, c], [0, 0])).toBe(2);
	});

	test('rotation à 45° qui élargit l’empreinte : chevauchement détecté', () => {
		const m = box(20, 20, 20);
		const a = obj('a', m, [100, 100, 0], [0, 0, 45]);
		// À 22 mm : boîtes sans rotation disjointes (10 + 10 < 22) mais losange
		// (14,14) + carré (10) > 22 → recouvrement réel.
		const b = obj('b', m, [122, 100, 0]);
		expect(overlappingCount([a, b], [0, 0])).toBe(2);
	});
});
