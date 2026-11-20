import { describe, expect, test } from 'vitest';
import { PlateSet } from './plates.svelte';

describe('« Déplacer le plateau vers l’avant » (move_plate_to_index(i, 0))', () => {
	test('le plateau passe en premier avec ses objets et devient actif', () => {
		const set = new PlateSet();
		const a = set.list()[0];
		const b = set.addPlate();
		const c = set.addPlate();
		set.assign('obj', c.id);
		set.activeId = a.id;
		expect(set.moveToFront(c.id)).toBe(true);
		expect(set.list().map((p) => p.id)).toEqual([c.id, a.id, b.id]);
		expect(set.activeId).toBe(c.id);
		expect(set.plateOf('obj')).toBe(c.id);
		expect(set.plateIndex(c.id)).toBe(0);
		// Déjà premier ou inconnu : rien.
		expect(set.moveToFront(c.id)).toBe(false);
		expect(set.moveToFront('nope')).toBe(false);
	});
});
