// Audit 46 VLH-a : le profil de hauteur de couche couvre toute la hauteur de
// l'objet transformé et posé (m_object_max_z d'Orca), jamais un repli de 10 mm.
import { describe, expect, test } from 'vitest';
import { layerZMax } from './layer-height';
import { worldMatrix } from './mesh-ops';
import type { SceneMesh } from '../mesh';

// Boîte recentrée 10 × 10 × 20 (comme les maillages servis par le serveur).
function box(h: number): SceneMesh {
	const positions: number[] = [];
	for (const x of [-5, 5])
		for (const y of [-5, 5]) for (const z of [-h / 2, h / 2]) positions.push(x, y, z);
	return {
		positions: new Float32Array(positions),
		indices: new Uint32Array([0, 1, 2])
	} as SceneMesh;
}

describe('layerZMax', () => {
	test('cylindre de 20 mm recentré, posé : 20 (pas 10)', () => {
		const m = worldMatrix({ position: [100, 100, 10], rotation: [0, 0, 0], scale: [1, 1, 1] });
		expect(layerZMax(box(20), m)).toBeCloseTo(20, 5);
	});
	test('mise à l’échelle et rotation prises en compte', () => {
		const scaled = worldMatrix({ position: [0, 0, 15], rotation: [0, 0, 0], scale: [1, 1, 1.5] });
		expect(layerZMax(box(20), scaled)).toBeCloseTo(30, 5);
		// Couchée (rotation X 90°) : la hauteur devient la profondeur (10 mm).
		const lying = worldMatrix({ position: [0, 0, 5], rotation: [90, 0, 0], scale: [1, 1, 1] });
		expect(layerZMax(box(20), lying)).toBeCloseTo(10, 4);
	});
	test('objet enfoncé : seule la partie au-dessus du plateau (layer_profile_matrix)', () => {
		const sunk = worldMatrix({ position: [0, 0, 6], rotation: [0, 0, 0], scale: [1, 1, 1] });
		expect(layerZMax(box(20), sunk)).toBeCloseTo(16, 5);
		// Flottant : posé (min z = 0).
		const floating = worldMatrix({ position: [0, 0, 40], rotation: [0, 0, 0], scale: [1, 1, 1] });
		expect(layerZMax(box(20), floating)).toBeCloseTo(20, 5);
	});
	test('maillage vide : 0', () => {
		expect(
			layerZMax(
				{ positions: new Float32Array(), indices: new Uint32Array() } as SceneMesh,
				worldMatrix({ position: [0, 0, 0], rotation: [0, 0, 0], scale: [1, 1, 1] })
			)
		).toBe(0);
	});
});
