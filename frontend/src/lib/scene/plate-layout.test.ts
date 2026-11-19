import { describe, expect, it } from 'vitest';
import type { BedShape } from './bed';
import {
	dropPlateIndex,
	plateColumns,
	plateOffsets,
	plateOrigin,
	plateRect,
	showDropHint
} from './plate-layout';

const bed = {
	polygon: [],
	width: 250,
	depth: 200,
	height: 250,
	origin: { x: 0, y: 0 },
	center: { x: 125, y: 100 },
	rectangular: true,
	customModel: null,
	customTexture: null
} as BedShape;

describe('plate-layout (PartPlateList d’Orca)', () => {
	it('compute_colum_count', () => {
		expect([1, 2, 3, 4, 5, 6, 7, 9, 10].map(plateColumns)).toEqual([1, 2, 2, 2, 3, 3, 3, 3, 4]);
	});
	it('origines : pas largeur × 1,2 en X, −profondeur × 1,2 en Y', () => {
		expect(plateOrigin(0, 4, bed)).toEqual([0, -0]);
		expect(plateOrigin(1, 4, bed)).toEqual([300, -0]);
		expect(plateOrigin(2, 4, bed)).toEqual([0, -240]);
		expect(plateOrigin(3, 4, bed)).toEqual([300, -240]);
		expect(plateOrigin(2, 3, bed)).toEqual([0, -240]);
	});
	it('décalages relatifs au plateau actif (actif à l’origine)', () => {
		const o = plateOffsets(3, 1, bed);
		expect(o[1]).toEqual([0, 0]);
		expect(o[0]).toEqual([-300, 0]);
		expect(o[2]).toEqual([-300, -240]);
		expect(plateRect(bed, o[0])).toEqual({ minX: -300, maxX: -50, minY: 0, maxY: 200 });
	});
	it('dépôt : reste tant qu’il touche son plateau, sinon premier plateau touché', () => {
		const box = (x: number, y: number) => ({
			minX: x - 10,
			maxX: x + 10,
			minY: y - 10,
			maxY: y + 10
		});
		expect(dropPlateIndex(box(125, 100), bed, 2, 0)).toBeNull();
		// À cheval sur le bord du plateau actif : reste (intersect_instance).
		expect(dropPlateIndex(box(255, 100), bed, 2, 0)).toBeNull();
		// Dans l'espace entre plateaux : hors de tout plateau, ne change pas.
		expect(dropPlateIndex(box(275, 100), bed, 2, 0)).toBeNull();
		expect(dropPlateIndex(box(400, 100), bed, 2, 0)).toBe(1);
		// Depuis le plateau 2 (actif) vers le plateau 1 à gauche.
		expect(dropPlateIndex(box(-150, 100), bed, 2, 1)).toBe(0);
		// Rangée suivante (−Y).
		expect(dropPlateIndex(box(125, -140), bed, 3, 0)).toBe(2);
	});
	it('indication « Glissez un modèle » masquée dès qu’un objet existe, même en échec', () => {
		expect(showDropHint(0, [], [])).toBe(true);
		expect(showDropHint(1, [], ['a'])).toBe(false);
		expect(showDropHint(0, [{ objectId: 'a', status: 'failed' }], ['a'])).toBe(false);
		// Échec sur un autre plateau : le plateau actif est bien vide.
		expect(showDropHint(0, [{ objectId: 'a', status: 'failed' }], ['b'])).toBe(true);
		expect(showDropHint(0, [{ objectId: 'a', status: 'ready' }], ['a'])).toBe(true);
	});
});
