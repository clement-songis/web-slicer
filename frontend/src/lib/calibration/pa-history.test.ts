import { describe, expect, it } from 'vitest';
import { paQuery, parseK, sortPa, takeLegacy, toInput, type PaRecord } from './pa-history';

const rec = (id: string, filament: string, updated: string): PaRecord => ({
	id,
	machine_preset: 'Voron',
	filament_preset: filament,
	nozzle_diameter: 0.4,
	extruder_id: 0,
	k_value: 0.04,
	n_coef: 1,
	method: 'pa_line',
	name: filament,
	created_at: updated,
	updated_at: updated
});

describe('historique PA (serveur)', () => {
	it('K : virgule, bornes, arrondi', () => {
		expect(parseK('0,045')).toBe(0.045);
		expect(parseK('3')).toBeNull();
	});
	it('filtres en requête', () => {
		expect(
			paQuery({ machine_preset: 'Voron 2.4', nozzle_diameter: 0.4, filament_preset: '' })
		).toBe('?machine_preset=Voron+2.4&nozzle_diameter=0.4');
		expect(paQuery({})).toBe('');
	});
	it('tri : filament courant puis plus récent', () => {
		const l = sortPa(
			[
				rec('a', 'PLA', '2026-11-01'),
				rec('b', 'PETG', '2026-11-02'),
				rec('c', 'PLA', '2026-11-03')
			],
			'PLA'
		);
		expect(l.map((x) => x.id)).toEqual(['c', 'a', 'b']);
	});
	it('PUT complet, migration du stockage local', () => {
		expect(toInput(rec('a', 'PLA', 'x'), { k_value: 0.05 }).k_value).toBe(0.05);
		const m = new Map([
			[
				'web-slicer.pa-history',
				JSON.stringify([{ printer: 'Voron', nozzle: '0.4', filament: 'PLA', k: 0.04 }])
			]
		]);
		const store = {
			getItem: (k: string) => m.get(k) ?? null,
			removeItem: (k: string) => void m.delete(k)
		};
		expect(takeLegacy(store)).toEqual([
			{
				machine_preset: 'Voron',
				filament_preset: 'PLA',
				nozzle_diameter: 0.4,
				k_value: 0.04,
				method: 'manual',
				name: 'PLA',
				note: null
			}
		]);
		expect(m.size).toBe(0);
	});
});
