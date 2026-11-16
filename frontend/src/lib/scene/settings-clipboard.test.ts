import { describe, expect, it } from 'vitest';
import { pasteSettings } from './settings-clipboard.svelte';
import { addOverride } from './object-settings';

describe('copier/coller des réglages d’objet', () => {
	it('colle les clés autorisées et garde les autres surcharges', () => {
		const r = pasteSettings({ wall_loops: 2 }, { wall_loops: 5, sparse_infill_density: 30 }, false);
		expect(r.settings).toEqual({ wall_loops: 5, sparse_infill_density: 30 });
		expect(r.pasted).toBe(2);
	});
	it('volume : les clés d’objet sont ignorées', () => {
		const r = pasteSettings({}, { enable_support: true, wall_loops: 4 }, true);
		expect(r.settings).toEqual({ wall_loops: 4 });
		expect(r.skipped).toEqual(['enable_support']);
	});
	it('surcharge ajoutée sans valeur de base : défaut Orca au format des valeurs', () => {
		expect(addOverride({}, 'precise_outer_wall', {}).precise_outer_wall).toBe(true);
	});
});
