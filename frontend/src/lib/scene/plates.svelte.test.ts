// Tests du modèle multi-plateaux (T059).
import { describe, expect, test } from 'vitest';
import { PlateSet, DEFAULT_PLATE_TYPE } from './plates.svelte';

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

	test('deserialize ne recrée pas un id existant après suppression', () => {
		const s = new PlateSet();
		s.addPlate(); // plate-2
		s.addPlate(); // plate-3
		s.removePlate('plate-1');
		const back = PlateSet.deserialize(s.serialize());
		expect(back.list().map((p) => p.id)).toEqual(['plate-2', 'plate-3']);
		const added = back.addPlate();
		expect(added.id).toBe('plate-4');
		expect(new Set(back.list().map((p) => p.id)).size).toBe(3);
	});

	test('deserialize choisit un nom de plateau libre', () => {
		const back = PlateSet.deserialize({
			plates: [
				{ id: 'plate-2', name: 'Plateau 2', plateType: DEFAULT_PLATE_TYPE, objectIds: [] },
				{ id: 'plate-3', name: 'Plateau 3', plateType: DEFAULT_PLATE_TYPE, objectIds: [] }
			],
			activeId: 'plate-3'
		});
		expect(back.addPlate().name).toBe('Plateau 4');
		expect(back.activeId).toBe('plate-3');
	});

	test('deserialize tolère des ids non numériques (copie de projet) sans collision', () => {
		const back = PlateSet.deserialize({
			plates: [
				{ id: 'a1b2', name: 'Plateau 1', plateType: DEFAULT_PLATE_TYPE, objectIds: [] },
				{ id: 'plate-1', name: 'Plateau 2', plateType: DEFAULT_PLATE_TYPE, objectIds: [] }
			],
			activeId: 'inconnu'
		});
		// Plateau actif inconnu → premier plateau.
		expect(back.activeId).toBe('a1b2');
		const added = back.addPlate();
		expect(added.id).toBe('plate-2');
		expect(added.name).toBe('Plateau 3');
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
	test('duplicatePlate copie les objets sur un nouveau plateau unique et l’active', () => {
		const s = new PlateSet();
		const src = s.activeId!;
		s.setPlateType(src, 'Engineering Plate');
		s.assign('a', src);
		s.assign('b', src);
		const other = s.addPlate();
		s.assign('z', other.id);
		const calls: [string, string][] = [];
		const copy = s.duplicatePlate(src, (objectId, plateId) => {
			calls.push([objectId, plateId]);
			return `${objectId}-copy`;
		});
		expect(copy).not.toBeNull();
		expect(copy!.id).not.toBe(src);
		expect(new Set(s.list().map((p) => p.id)).size).toBe(s.list().length);
		expect(copy!.name).toBe('Plateau 1 (copie)');
		expect(copy!.plateType).toBe('Engineering Plate');
		expect(calls).toEqual([
			['a', copy!.id],
			['b', copy!.id]
		]);
		expect(s.get(copy!.id)!.objectIds).toEqual(['a-copy', 'b-copy']);
		// Source intacte, copie insérée juste après, activée.
		expect(s.get(src)!.objectIds).toEqual(['a', 'b']);
		expect(s.plateIndex(copy!.id)).toBe(s.plateIndex(src) + 1);
		expect(s.activeId).toBe(copy!.id);
		expect(s.plateOf('z')).toBe(other.id);
	});

	test('duplicatePlate : noms uniques, objets non copiables ignorés, source inconnue', () => {
		const s = new PlateSet();
		const src = s.activeId!;
		s.assign('a', src);
		s.assign('b', src);
		const first = s.duplicatePlate(src, () => null)!;
		expect(first.objectIds).toEqual([]);
		const second = s.duplicatePlate(src, (id) => (id === 'a' ? 'a2' : null))!;
		expect(second.name).toBe('Plateau 1 (copie 2)');
		expect(second.objectIds).toEqual(['a2']);
		expect(s.duplicatePlate('nope', () => 'x')).toBeNull();
		expect(s.activeId).toBe(second.id);
	});

	test('duplicatePlate survit au round-trip de sérialisation', () => {
		const s = new PlateSet();
		const src = s.activeId!;
		s.assign('a', src);
		const copy = s.duplicatePlate(src, () => 'a-copy')!;
		const back = PlateSet.deserialize(s.serialize());
		expect(back.activeId).toBe(copy.id);
		expect(back.plateOf('a-copy')).toBe(copy.id);
		expect(back.plateOf('a')).toBe(src);
		expect(back.addPlate().id).not.toBe(copy.id);
	});
});

describe('réglages par plateau', () => {
	test('définit, efface et sérialise plates[].settings', () => {
		const set = new PlateSet();
		const id = set.list()[0].id;
		set.setPlateSetting(id, 'print_sequence', 'by object');
		set.setPlateSetting(id, 'spiral_mode', '1');
		const doc = set.serialize();
		expect(doc.plates[0].settings).toEqual({ print_sequence: 'by object', spiral_mode: '1' });
		// Copie profonde : modifier le document ne touche pas le jeu.
		doc.plates[0].settings!.print_sequence = 'by layer';
		expect(set.get(id)?.settings?.print_sequence).toBe('by object');
		const back = PlateSet.deserialize(set.serialize());
		expect(back.get(id)?.settings).toEqual({ print_sequence: 'by object', spiral_mode: '1' });
		set.setPlateSetting(id, 'print_sequence', undefined);
		set.setPlateSetting(id, 'spiral_mode', undefined);
		expect(set.get(id)?.settings).toBeUndefined();
	});
});
