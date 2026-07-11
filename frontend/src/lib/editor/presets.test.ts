// Tests de l'orchestrateur de presets actifs (T098).
import { describe, expect, test } from 'bun:test';
import {
	emptyActivePresets,
	parseActivePresets,
	primaryFilament,
	serializeActivePresets,
	setFilament,
	setPrinter,
	setProcess
} from './presets';

describe('parseActivePresets', () => {
	test('blob vide → sélection vide', () => {
		expect(parseActivePresets({})).toEqual({ printer: null, process: null, filaments: [] });
		expect(parseActivePresets(null)).toEqual(emptyActivePresets());
		expect(parseActivePresets('nope')).toEqual(emptyActivePresets());
	});

	test('lit printer/process/filaments (schéma backend)', () => {
		const a = parseActivePresets({ printer: 'p1', process: 'q1', filaments: ['f1', 'f2'] });
		expect(a).toEqual({ printer: 'p1', process: 'q1', filaments: ['f1', 'f2'] });
	});

	test('ignore les valeurs non-chaînes et vides', () => {
		const a = parseActivePresets({ printer: '', process: 3, filaments: ['f1', 7, ''] });
		expect(a).toEqual({ printer: null, process: null, filaments: ['f1'] });
	});
});

describe('serializeActivePresets', () => {
	test('omet les clés vides', () => {
		expect(serializeActivePresets(emptyActivePresets())).toEqual({});
		expect(serializeActivePresets({ printer: 'p1', process: null, filaments: ['f1'] })).toEqual({
			printer: 'p1',
			filaments: ['f1']
		});
	});

	test('aller-retour stable', () => {
		const a = { printer: 'p1', process: 'q1', filaments: ['f1'] };
		expect(parseActivePresets(serializeActivePresets(a))).toEqual(a);
	});
});

describe('transitions', () => {
	test('setPrinter/setProcess immuables', () => {
		const a = emptyActivePresets();
		const b = setPrinter(a, 'p1');
		expect(b.printer).toBe('p1');
		expect(a.printer).toBeNull();
		expect(setProcess(b, 'q1').process).toBe('q1');
	});

	test('setFilament pose, remplace et retire', () => {
		let a = emptyActivePresets();
		a = setFilament(a, 'f1');
		expect(a.filaments).toEqual(['f1']);
		expect(primaryFilament(a)).toBe('f1');
		a = setFilament(a, 'f2', 1);
		expect(a.filaments).toEqual(['f1', 'f2']);
		a = setFilament(a, null, 0);
		expect(a.filaments).toEqual(['f2']);
	});
});
