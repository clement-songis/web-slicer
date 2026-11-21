// Phase 44b : formes génériques qui annulent l'échelle de l'objet (Orca
// volume_to_bed_transformation) — échelle uniforme compensée côté requête.
import { describe, expect, it } from 'vitest';
import { genericVolumePlacement, primitiveBedSize, uniformScale, worldVolumeSize } from './volumes';

describe('formes génériques sur un objet mis à l’échelle', () => {
	it('échelle uniforme reconnue, non uniforme refusée', () => {
		expect(uniformScale([2, 2, 2])).toBe(2);
		expect(uniformScale([1, 1, 1])).toBe(1);
		expect(uniformScale([2, 1, 2])).toBeNull();
		expect(uniformScale([0, 0, 0])).toBeNull();
	});

	it('plateau demandé ÷ échelle : côté à l’écran = 10 % du plateau', () => {
		const bed = { width: 350, depth: 300 };
		const [bx, by] = primitiveBedSize(bed, [2, 2, 2]);
		expect([bx, by]).toEqual([175, 150]);
		// côté serveur : 0,1 × max(bx, by) = 17,5 mm local → 35 mm à l'écran.
		expect(0.1 * Math.max(bx, by) * 2).toBeCloseTo(0.1 * 350, 9);
		expect(primitiveBedSize(bed, [2, 1, 1])).toEqual([350, 300]);
	});

	it('placement d’Orca avec l’emprise à l’écran (objet ×2)', () => {
		// Cube unité de 10 mm (±5) mis à l'échelle ×2 au centre (100, 100, 10).
		const positions = [-5, -5, -5, 5, 5, 5];
		const transform = {
			position: [100, 100, 10] as [number, number, number],
			rotation: [0, 0, 0] as [number, number, number],
			scale: [2, 2, 2] as [number, number, number]
		};
		// Forme de 17,5 mm locaux (plateau ÷ 2) → 35 mm à l'écran.
		const world = worldVolumeSize([17.5, 17.5, 17.5], transform.scale);
		expect(world).toEqual([35, 35, 35]);
		const { offset } = genericVolumePlacement(positions, transform, world);
		// Centre à l'écran : (max.x + 17,5 ; min.y + 17,5 ; min.z + 17,5) = (127,5 ; 107,5 ; 17,5)
		// → offset local = (écran − position) / 2.
		expect(offset).toEqual([13.75, 3.75, 3.75]);
	});
});
