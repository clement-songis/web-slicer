// Tests du modèle multi-plateaux (T059).
import { describe, expect, test } from 'bun:test';
import { PlateSet, DEFAULT_PLATE_TYPE } from './plates';

describe('PlateSet', () => {
	test('démarre avec un plateau actif', () => {
		const s = new PlateSet();
		expect(s.list().length).toBe(1);
		expect(s.activeId).toBe(s.list()[0].id);
		expect(s.list()[0].plateType).toBe(DEFAULT_PLATE_TYPE);
	});

	test('assigne un objet à un plateau (exclusif)', () => {
		const s = new PlateSet();
		const p1 = s.list()[0].id;
		const p2 = s.addPlate().id;
		s.assign('obj', p1);
		expect(s.plateOf('obj')).toBe(p1);
		s.assign('obj', p2); // déplacé
		expect(s.plateOf('obj')).toBe(p2);
		expect(s.get(p1)!.objectIds).toEqual([]);
	});

	test('supprime un plateau et migre ses objets', () => {
		const s = new PlateSet();
		const p1 = s.list()[0].id;
		const p2 = s.addPlate().id;
		s.assign('obj', p2);
		expect(s.removePlate(p2)).toBe(true);
		expect(s.plateOf('obj')).toBe(p1);
	});

	test('refuse de supprimer le dernier plateau', () => {
		const s = new PlateSet();
		expect(s.removePlate(s.list()[0].id)).toBe(false);
	});

	test('répartit les objets par paquets, créant les plateaux', () => {
		const s = new PlateSet();
		s.distribute(['a', 'b', 'c', 'd', 'e'], 2);
		expect(s.list().length).toBe(3);
		expect(s.list()[0].objectIds).toEqual(['a', 'b']);
		expect(s.list()[1].objectIds).toEqual(['c', 'd']);
		expect(s.list()[2].objectIds).toEqual(['e']);
	});

	test('plateIndex reflète l’ordre (cible du tranchage P5)', () => {
		const s = new PlateSet();
		const p2 = s.addPlate().id;
		expect(s.plateIndex(p2)).toBe(1);
	});

	test('type de plaque paramétrable et round-trip', () => {
		const s = new PlateSet();
		const id = s.list()[0].id;
		s.setPlateType(id, 'Engineering Plate');
		s.assign('obj', id);
		const doc = s.serialize();
		const back = PlateSet.deserialize(doc);
		expect(back.get(id)!.plateType).toBe('Engineering Plate');
		expect(back.plateOf('obj')).toBe(id);
	});
});
