import { describe, expect, test } from 'vitest';
import {
	defaultSelection,
	filamentChoices,
	filamentType,
	filterChoices,
	orderChoices,
	preferredFilaments,
	presetNames
} from './wizard-filaments';
import type { PresetSummary } from '$lib/api/types';

const p = (name: string, origin = 'system', instantiation = true): PresetSummary => ({
	id: name,
	kind: 'filament',
	name,
	origin,
	instantiation
});

describe('assistant : page Filaments (phase 25)', () => {
	test('type déduit du nom', () => {
		expect(filamentType('Bambu PLA Basic @BBL X1C')).toBe('PLA');
		expect(filamentType('Generic PETG-CF @System')).toBe('PETG-CF');
		expect(filamentType('Polymaker PolyLite ASA')).toBe('ASA');
		expect(filamentType('Generic PC')).toBe('PC');
		expect(filamentType('Fiberon PA6-GF')).toBe('PA');
		expect(filamentType('Support W')).toBe('Autre');
	});
	test('union des filaments compatibles, système instanciables, triés', () => {
		const choices = filamentChoices([
			[p('Generic PLA @System'), p('Bambu PLA Basic @BBL X1C'), p('fdm_filament_pla', 'system', false)],
			[p('Bambu PLA Basic @BBL X1C'), p('Mon PLA', 'user')]
		]);
		expect(choices.map((c) => c.name)).toEqual(['Bambu PLA Basic @BBL X1C', 'Generic PLA @System']);
		expect(choices[0]).toMatchObject({ vendor: 'Bambu', type: 'PLA' });
	});
	test('pré-sélection = filaments par défaut des imprimantes (default_filament_profile)', () => {
		const choices = filamentChoices([[p('Bambu PLA Basic @BBL X1C'), p('Generic PLA @System')]]);
		expect(presetNames('"a";b')).toEqual(['a', 'b']);
		expect(
			defaultSelection(choices, [['Bambu PLA Basic @BBL X1C'], 'Inconnu', 'Bambu PLA Basic @BBL X1C'])
		).toEqual(['Bambu PLA Basic @BBL X1C']);
	});
	test('ordre : cochés, puis leur marque et Generic, puis le reste', () => {
		const choices = filamentChoices([
			[p('addnorth PLA'), p('Bambu PETG HF @BBL X1C'), p('Bambu PLA Basic @BBL X1C'), p('Generic PLA')]
		]);
		expect(orderChoices(choices, ['Bambu PLA Basic @BBL X1C']).map((c) => c.name)).toEqual([
			'Bambu PLA Basic @BBL X1C',
			'Bambu PETG HF @BBL X1C',
			'Generic PLA',
			'addnorth PLA'
		]);
	});
	test('filtres : recherche, marque, type, cochés', () => {
		const choices = filamentChoices([
			[p('Bambu PLA Basic @BBL X1C'), p('Bambu PETG HF @BBL X1C'), p('Generic PLA @System')]
		]);
		expect(filterChoices(choices, { query: 'petg' }).map((c) => c.name)).toEqual([
			'Bambu PETG HF @BBL X1C'
		]);
		expect(filterChoices(choices, { vendor: 'Generic' })).toHaveLength(1);
		expect(filterChoices(choices, { type: 'PLA' })).toHaveLength(2);
		expect(filterChoices(choices, { only: ['Generic PLA @System'] })).toHaveLength(1);
	});
	test('filament d’un nouveau projet : défaut machine s’il est choisi, sinon premier choisi', () => {
		expect(preferredFilaments('A', null)).toEqual(['A']);
		expect(preferredFilaments('A', ['B', 'A'])).toEqual(['A', 'B']);
		expect(preferredFilaments(['A'], ['B', 'C'])).toEqual(['B', 'C', 'A']);
	});
});
