import { describe, expect, it } from 'vitest';
import { IDENTITY, quaternionFromEulerDeg, rotateVectorByEulerDeg } from './transform';
import { placedTransform, plateMoves, transformFromMatrix, unarrangedIds } from './arrange-gui';
import type { Plate } from './plates.svelte';

const plate = (id: string, objectIds: string[]): Plate => ({
	id,
	name: id,
	plateType: '',
	objectIds
});

// Matrice colonne-major 4×4 : translation + rotation Z de 90°.
const ROT_Z90 = [0, 1, 0, 0, -1, 0, 0, 0, 0, 0, 1, 0, 30, 20, 0, 1];

describe('application de l’arrangement GUI (lot 34)', () => {
	it('matrice monde → translation, rotation, échelle', () => {
		const t = transformFromMatrix(ROT_Z90);
		expect(t.position).toEqual([30, 20, 0]);
		// 90° autour de Z : X (1,0,0) → (0,1,0).
		const rotated = rotateVectorByEulerDeg(t.rotation, [1, 0, 0]);
		expect(rotated[0]).toBeCloseTo(0, 6);
		expect(rotated[1]).toBeCloseTo(1, 6);
		expect(t.scale).toEqual([1, 1, 1]);
	});

	it('aller-retour Euler → matrice → Euler', () => {
		const euler: [number, number, number] = [30, -45, 20];
		const [x, y, z, w] = quaternionFromEulerDeg(euler);
		// Reconstruit une matrice de rotation (sans échelle) depuis le quaternion.
		const m = [
			1 - 2 * (y * y + z * z),
			2 * (x * y + w * z),
			2 * (x * z - w * y),
			0,
			2 * (x * y - w * z),
			1 - 2 * (x * x + z * z),
			2 * (y * z + w * x),
			0,
			2 * (x * z + w * y),
			2 * (y * z - w * x),
			1 - 2 * (x * x + y * y),
			0,
			5,
			6,
			7,
			1
		];
		const back = transformFromMatrix(m);
		expect(back.rotation[0]).toBeCloseTo(euler[0], 3);
		expect(back.rotation[1]).toBeCloseTo(euler[1], 3);
		expect(back.rotation[2]).toBeCloseTo(euler[2], 3);
	});

	it('placement sans matrice : position XY seule, Z conservé', () => {
		const t = placedTransform({ ...IDENTITY, position: [1, 2, 8] }, { id: 'a', x: 40, y: -3 });
		expect(t.position).toEqual([40, -3, 8]);
		expect(t.rotation).toEqual([0, 0, 0]);
	});

	it('réaffectations : seuls les objets changés de plateau', () => {
		const plates = [plate('p1', ['a', 'b']), plate('p2', ['c'])];
		const moves = plateMoves(plates, [
			{ id: 'a', x: 0, y: 0, plate_index: 0 },
			{ id: 'b', x: 5, y: 5, plate_index: 1 },
			// Non arrangé (trop grand) : pas de réaffectation.
			{ id: 'c', x: 1, y: 1, plate_index: 2, unarranged: true }
		]);
		expect(moves).toEqual([{ objectId: 'b', plateIndex: 1 }]);
	});

	it('objets trop grands listés à part (notification d’Orca)', () => {
		expect(
			unarrangedIds([
				{ id: 'big', x: 0, y: 0, unarranged: true },
				{ id: 'ok', x: 1, y: 1 }
			])
		).toEqual(['big']);
	});
});
