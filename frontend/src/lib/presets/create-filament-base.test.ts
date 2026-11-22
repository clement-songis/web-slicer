// Phase 46 (audit 44 PR1-c) : « Créer un filament » — filament de départ
// filtré par type comme Orca (CreatePresetsDialog.cpp:1210-1381).
import { describe, expect, it } from 'vitest';
import { baseFilamentChoices, defaultBaseFilament, nameType } from './create-custom';

const presets = [
	{ id: 'pla', name: 'Generic PLA @System', origin: 'system' },
	{ id: 'plasilk', name: 'Generic PLA Silk @System', origin: 'system' },
	{ id: 'petg', name: 'Generic PETG @System', origin: 'system' },
	{ id: 'petghf', name: 'Generic PETG HF @System', origin: 'system' },
	{ id: 'bpetg', name: 'Bambu PETG Basic @BBL X1C', origin: 'system' },
	{ id: 'user', name: 'Atelier PETG Mat', origin: 'user', inherits: 'Generic PLA @System' }
];
const types: Record<string, string> = {
	pla: 'PLA',
	plasilk: 'PLA',
	petg: 'PETG',
	petghf: 'PETG',
	bpetg: 'PETG'
};
const typeOf = (p: { id: string }) => types[p.id];

describe('baseFilamentChoices', () => {
	it('ne propose que les filaments système du type choisi', () => {
		expect(baseFilamentChoices(presets, 'PETG', typeOf).map((p) => p.id)).toEqual([
			'petg',
			'petghf',
			'bpetg'
		]);
		expect(baseFilamentChoices(presets, 'PLA', typeOf).map((p) => p.id)).toEqual([
			'pla',
			'plasilk'
		]);
		expect(baseFilamentChoices(presets, 'ABS', typeOf)).toEqual([]);
	});
	it('repli sur le nom sans gabarits serveur', () => {
		expect(baseFilamentChoices(presets, 'PETG', () => null).map((p) => p.id)).toEqual([
			'petg',
			'petghf',
			'bpetg'
		]);
	});
});

describe('defaultBaseFilament', () => {
	const petg = baseFilamentChoices(presets, 'PETG', typeOf);
	it('garde le filament courant s’il est du type', () => {
		expect(defaultBaseFilament(petg, 'bpetg', 'PETG')).toBe('bpetg');
	});
	it('sinon le Generic du type (jamais le PLA courant)', () => {
		expect(defaultBaseFilament(petg, 'pla', 'PETG')).toBe('petg');
	});
	it('sinon le premier, ou rien', () => {
		expect(defaultBaseFilament([presets[4]], 'pla', 'PETG')).toBe('bpetg');
		expect(defaultBaseFilament([], 'pla', 'ABS')).toBeNull();
	});
});

describe('nameType', () => {
	it('mot-type le plus long, hors suffixe @imprimante', () => {
		expect(nameType('Generic PETG HF @System')).toBe('PETG');
		expect(nameType('Generic PLA+ @System')).toBe('PLA+');
		expect(nameType('Polymaker PLA @PA printer')).toBe('PLA');
		expect(nameType('Mystery')).toBeNull();
	});
});
