import { describe, expect, it } from 'vitest';
import {
	canTransformObject,
	movableObjectsOnPlate,
	objectsOnPlate,
	viewportObjects
} from './interactions';

const objects = [{ id: 'a' }, { id: 'b' }, { id: 'c' }];
const plate = { id: 'p1', objectIds: ['a', 'b'] };

describe('interactions objet/plateau', () => {
	it('filtre les objets hors plateau actif', () => {
		expect(objectsOnPlate(objects, plate).map((o) => o.id)).toEqual(['a', 'b']);
		expect(objectsOnPlate(objects, null)).toEqual([]);
	});

	it('filtre aussi les objets masqués du viewport, sans les retirer du plateau', () => {
		expect(viewportObjects(objects, plate, (id) => id === 'a').map((o) => o.id)).toEqual(['b']);
		expect(plate.objectIds).toEqual(['a', 'b']);
	});

	it('refuse les transformations hors plateau et sur objets verrouillés', () => {
		const locked = (id: string) => id === 'a';
		expect(canTransformObject('a', plate, locked)).toBe(false);
		expect(canTransformObject('b', plate, locked)).toBe(true);
		expect(canTransformObject('c', plate, locked)).toBe(false);
		expect(canTransformObject('b', null, locked)).toBe(false);
	});

	it("n'arrange que les objets mobiles du plateau actif", () => {
		expect(movableObjectsOnPlate(objects, plate, (id) => id === 'a').map((o) => o.id)).toEqual([
			'b'
		]);
	});
});
