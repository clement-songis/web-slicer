import { describe, expect, it } from 'vitest';
import { nearestFreeCell } from './arrange';

const bed = { x: 0, y: 0, width: 256, depth: 256 };
describe('nearestFreeCell (imports successifs, Orca get_nearest_empty_cell)', () => {
	it('plateau vide : centre', () => {
		expect(nearestFreeCell({ width: 20, depth: 20 }, [], bed)).toEqual([128, 128]);
	});
	it('à côté du premier objet, sans chevauchement (écart 6 mm), cellule la plus proche', () => {
		const [x, y] = nearestFreeCell(
			{ width: 20, depth: 20 },
			[{ x: 128, y: 128, width: 20, depth: 20 }],
			bed
		);
		const gap = Math.max(Math.abs(x - 128), Math.abs(y - 128));
		expect(gap).toBeGreaterThanOrEqual(26);
		expect(Math.hypot(x - 128, y - 128)).toBeLessThanOrEqual(30 + 1e-9);
	});
	it('reste dans le plateau ; repli d’Orca si plein', () => {
		const full = [{ x: 128, y: 128, width: 256, depth: 256 }];
		expect(nearestFreeCell({ width: 20, depth: 20 }, full, bed)).toEqual([128 + 12.8, 128 + 12.8]);
	});
});
