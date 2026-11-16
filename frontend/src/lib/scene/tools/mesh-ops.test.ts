import { describe, expect, it } from 'vitest';
import { meshOpErrorMessage, placeSourcePart, worldMatrix } from './mesh-ops';

describe('placement des pièces découpées', () => {
	it('sans rotation : position source + centre mis à l’échelle', () => {
		const out = placeSourcePart(
			{ position: [100, 100, 10], rotation: [0, 0, 0], scale: [2, 1, 1] },
			[-10, 0, 5]
		);
		expect(out.position).toEqual([80, 100, 15]);
		expect(out.scale).toEqual([2, 1, 1]);
	});

	it('rotation Z de 90° : le décalage tourne avec l’objet', () => {
		const out = placeSourcePart(
			{ position: [0, 0, 0], rotation: [0, 0, 90], scale: [1, 1, 1] },
			[10, 0, 0]
		);
		expect(out.position[0]).toBeCloseTo(0, 6);
		expect(out.position[1]).toBeCloseTo(10, 6);
		expect(out.rotation).toEqual([0, 0, 90]);
	});

	it('matrice monde colonne-major (translation en 12..14)', () => {
		const m = worldMatrix({ position: [1, 2, 3], rotation: [0, 0, 0], scale: [2, 2, 2] });
		expect(m).toHaveLength(16);
		expect(m[0]).toBeCloseTo(2);
		expect([m[12], m[13], m[14]]).toEqual([1, 2, 3]);
	});

	it('messages 409 / 422', () => {
		expect(meshOpErrorMessage('Coupe', 409, 'x')).toContain('conversion');
		expect(meshOpErrorMessage('Coupe', 422, 'plan hors objet')).toContain('plan hors objet');
	});
});
