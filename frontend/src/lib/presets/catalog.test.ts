// Logique des sélecteurs de presets (T046) : arbre imprimante, filtre de
// compatibilité, badge d'héritage.
import { describe, expect, it } from 'bun:test';

import type { PresetSummary } from '../api/types';
import {
	filterByPrinter,
	groupPrinters,
	inheritanceLabel,
	isCompatible,
	isDerived,
	parseNozzle,
	partitionByOrigin,
	printerModel
} from './catalog';

function machine(name: string, vendor = 'Bambu Lab'): PresetSummary {
	return {
		id: name,
		kind: 'machine',
		name,
		origin: 'system',
		vendor,
		instantiation: true
	};
}

describe('nozzle / model parsing', () => {
	it('extrait la buse et le modèle', () => {
		expect(parseNozzle('Bambu Lab A1 mini 0.4 nozzle')).toBe('0.4');
		expect(printerModel('Bambu Lab A1 mini 0.4 nozzle')).toBe('Bambu Lab A1 mini');
		expect(parseNozzle('Custom Printer')).toBeNull();
		expect(printerModel('Custom Printer')).toBe('Custom Printer');
	});
});

describe('groupPrinters', () => {
	it('bâtit un arbre vendeur → modèle → buse trié', () => {
		const tree = groupPrinters([
			machine('Bambu Lab A1 0.4 nozzle'),
			machine('Bambu Lab A1 0.2 nozzle'),
			machine('Bambu Lab A1 mini 0.4 nozzle'),
			machine('Voron 2.4 0.4 nozzle', 'Voron')
		]);
		expect(tree.map((v) => v.vendor)).toEqual(['Bambu Lab', 'Voron']);
		const bambu = tree[0];
		expect(bambu.models.map((m) => m.model)).toEqual(['Bambu Lab A1', 'Bambu Lab A1 mini']);
		// Buses triées au sein du modèle A1.
		expect(bambu.models[0].nozzles.map((n) => n.nozzle)).toEqual(['0.2', '0.4']);
	});

	it('ignore les presets non machine', () => {
		const filament: PresetSummary = {
			id: 'f',
			kind: 'filament',
			name: 'Generic PLA',
			origin: 'system',
			instantiation: true
		};
		expect(groupPrinters([filament])).toEqual([]);
	});
});

describe('isCompatible / filterByPrinter (FR-021)', () => {
	it('universel si la liste est vide/absente', () => {
		expect(isCompatible(null, 'Bambu Lab A1 0.4 nozzle')).toBe(true);
		expect(isCompatible([], 'Bambu Lab A1 0.4 nozzle')).toBe(true);
	});

	it('restreint à l’imprimante active quand une liste est fournie', () => {
		expect(isCompatible(['Bambu Lab A1 0.4 nozzle'], 'Bambu Lab A1 0.4 nozzle')).toBe(true);
		expect(isCompatible(['Bambu Lab A1 0.4 nozzle'], 'Voron 2.4 0.4 nozzle')).toBe(false);
	});

	it('montre tout en l’absence d’imprimante active', () => {
		expect(isCompatible(['Bambu Lab A1 0.4 nozzle'], null)).toBe(true);
	});

	it('filtre une liste de presets par imprimante', () => {
		const items = [
			{ id: 'a', compatiblePrinters: ['Bambu Lab A1 0.4 nozzle'] },
			{ id: 'b', compatiblePrinters: null },
			{ id: 'c', compatiblePrinters: ['Voron 2.4 0.4 nozzle'] }
		];
		const kept = filterByPrinter(items, 'Bambu Lab A1 0.4 nozzle').map((i) => i.id);
		expect(kept).toEqual(['a', 'b']);
	});
});

describe('héritage', () => {
	it('détecte un preset dérivé et son libellé', () => {
		expect(isDerived({ origin: 'user', inherits: 'Generic PLA' })).toBe(true);
		expect(isDerived({ origin: 'user' })).toBe(false);
		expect(isDerived({ origin: 'system', inherits: 'fdm_common' })).toBe(false);
		expect(inheritanceLabel({ origin: 'user', inherits: 'Generic PLA' })).toBe(
			'hérite de Generic PLA'
		);
		expect(inheritanceLabel({ origin: 'user' })).toBeNull();
	});
});

describe('partitionByOrigin', () => {
	it('sépare système et utilisateur en préservant l’ordre', () => {
		const { system, user } = partitionByOrigin([
			{ origin: 'system', id: 's1' },
			{ origin: 'user', id: 'u1' },
			{ origin: 'system', id: 's2' }
		]);
		expect(system.map((p) => p.id)).toEqual(['s1', 's2']);
		expect(user.map((p) => p.id)).toEqual(['u1']);
	});
});
