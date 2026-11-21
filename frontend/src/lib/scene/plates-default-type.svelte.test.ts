// Phase 44b : type de plaque par défaut = celui de l'imprimante (Orca
// Preset::get_default_bed_type), plateaux « par défaut » qui le suivent.
import { describe, expect, it } from 'vitest';
import { DEFAULT_PLATE_TYPE, PlateSet } from './plates.svelte';

const HOT = 'Smooth High Temp Plate';
const PEI = 'Textured PEI Plate';

describe('PlateSet — type de plaque par défaut', () => {
	it('sans imprimante connue : Cool Plate (comportement historique)', () => {
		const s = new PlateSet();
		expect(s.list()[0].plateType).toBe(DEFAULT_PLATE_TYPE);
		expect(s.defaultPlateType).toBe(DEFAULT_PLATE_TYPE);
	});

	it('un nouveau plateau prend le type de l’imprimante', () => {
		const s = new PlateSet(HOT);
		expect(s.list()[0].plateType).toBe(HOT);
		expect(s.addPlate().plateType).toBe(HOT);
	});

	it('imprimante résolue après coup : les plateaux automatiques suivent, pas les choisis', () => {
		const s = new PlateSet();
		const second = s.addPlate();
		s.setPlateType(second.id, PEI);
		const changed = s.followDefaultPlateType(HOT);
		expect(changed).toEqual([s.list()[0].id]);
		expect(s.list()[0].plateType).toBe(HOT);
		expect(s.get(second.id)?.plateType).toBe(PEI);
		expect(s.addPlate().plateType).toBe(HOT);
	});

	it('changement d’imprimante : les plateaux au défaut précédent suivent le nouveau', () => {
		const s = PlateSet.deserialize({
			activeId: 'plate-1',
			plates: [
				{ id: 'plate-1', name: 'P1', plateType: HOT, objectIds: [] },
				{ id: 'plate-2', name: 'P2', plateType: PEI, objectIds: [] }
			]
		});
		// Chargement d'un projet : types relus conservés.
		expect(s.followDefaultPlateType(DEFAULT_PLATE_TYPE)).toEqual([]);
		expect(s.list().map((p) => p.plateType)).toEqual([HOT, PEI]);
		// L'utilisateur passe d'une imprimante « haute température » à une X1.
		expect(s.followDefaultPlateType(DEFAULT_PLATE_TYPE, HOT)).toEqual(['plate-1']);
		expect(s.list().map((p) => p.plateType)).toEqual([DEFAULT_PLATE_TYPE, PEI]);
	});

	it('désérialisation d’un document vide : plateau automatique au type donné', () => {
		const s = PlateSet.deserialize({ activeId: null, plates: [] }, PEI);
		expect(s.list()[0].plateType).toBe(PEI);
		s.followDefaultPlateType(HOT);
		expect(s.list()[0].plateType).toBe(HOT);
	});
});
