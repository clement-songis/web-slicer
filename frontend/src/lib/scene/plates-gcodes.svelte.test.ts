import { describe, expect, test } from 'vitest';
import { PlateSet } from './plates.svelte';

describe('PlateSet : G-code par couche (phase 34)', () => {
	test('setCustomGcodes : posé, sérialisé en copie, retiré quand vide', () => {
		const set = new PlateSet();
		const id = set.activeId!;
		const list = [{ printZ: 1.2, type: 'color_change' as const, extruder: 1, color: '#FF0000' }];
		set.setCustomGcodes(id, list);
		list[0].printZ = 9; // la copie interne ne suit pas
		const doc = set.serialize();
		expect(doc.plates[0].customGcodes).toEqual([
			{ printZ: 1.2, type: 'color_change', extruder: 1, color: '#FF0000' }
		]);
		const back = PlateSet.deserialize(JSON.parse(JSON.stringify(doc)));
		expect(back.get(id)?.customGcodes?.[0].printZ).toBe(1.2);
		set.setCustomGcodes(id, []);
		expect('customGcodes' in set.serialize().plates[0]).toBe(false);
	});
});
