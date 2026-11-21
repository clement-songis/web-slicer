import { describe, expect, test } from 'vitest';
import {
	assemblyInfo,
	assemblySectionPlane,
	formatAssemblyInfo,
	objectsBox,
	unionBox,
	volumeWorldBox
} from './assembly-info';
import type { SceneMesh } from './mesh';

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
const r = (v: number[]) => v.map((x) => Math.round(x * 1e6) / 1e6 + 0);

describe('vue d’assemblage : informations et coupe (GLCanvas3D d’Orca)', () => {
	test('informations : boîte de la sélection, volume = produit des dimensions', () => {
		const b = objectsBox([
			{ mesh: box(20, 20, 20), position: [0, 0, 10], rotation: [0, 0, 0], scale: [1, 1, 1] },
			{ mesh: box(10, 10, 10), position: [30, 0, 10], rotation: [0, 0, 0], scale: [1, 1, 2] }
		]);
		expect(b).toEqual({ min: [-10, -10, 0], max: [35, 10, 20] });
		const info = assemblyInfo(b)!;
		expect(info.size).toEqual([45, 20, 20]);
		expect(formatAssemblyInfo(info)).toEqual({ volume: '18000.00', size: '45.00 x 20.00 x 20.00' });
		expect(assemblyInfo(null)).toBeNull();
		expect(unionBox([])).toBeNull();
	});
	test('pièce : pavé tourné puis transformation de l’objet', () => {
		const b = volumeWorldBox(
			{ position: [100, 0, 0], rotation: [0, 0, 90], scale: [2, 2, 2] },
			{ size: [10, 4, 2], offset: [5, 0, 0], rotation: [0, 0, 0] }
		);
		// pavé x 0..10 → ×2 → 0..20 ; rotation 90° autour de Z → y 0..20.
		expect(r(b.min)).toEqual([96, 0, -2]);
		expect(r(b.max)).toEqual([104, 20, 2]);
	});
	test('coupe : normale vers la caméra, balaye la sphère englobante', () => {
		const b = { min: [-10, -10, 0], max: [10, 10, 20] } as const;
		const bb = { min: [...b.min], max: [...b.max] } as {
			min: [number, number, number];
			max: [number, number, number];
		};
		expect(assemblySectionPlane([0, 0, 1], bb, 0)).toBeNull();
		expect(assemblySectionPlane(null, bb, 0.5)).toBeNull();
		const radius = Math.hypot(20, 20, 20) / 2;
		const half = assemblySectionPlane([0, 0, 1], bb, 0.5)!;
		expect(half.offset).toBeCloseTo(10, 9); // passe par le centre
		expect(assemblySectionPlane([0, 0, 1], bb, 1)!.offset).toBeCloseTo(10 - radius, 9);
	});
});
