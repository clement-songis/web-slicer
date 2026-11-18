// Tests de l'orchestrateur de presets actifs (T098).
import { describe, expect, test } from 'vitest';
import {
	addFilamentSlot,
	canRemoveFilamentSlot,
	clampExtruder,
	clampFilamentSlot,
	emptyActivePresets,
	filamentAt,
	filamentSlotCount,
	remapExtruderAfterRemoval,
	removeFilamentSlot,
	slotAfterRemoval,
	parseActivePresets,
	primaryFilament,
	serializeActivePresets,
	setFilament,
	setPrinter,
	setProcess,
	importedFilamentSlots
} from './presets';
import type { ActivePresets } from './presets';

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

describe('multi-filament (slots)', () => {
	const sel = (filaments: string[]): ActivePresets => ({ printer: 'p1', process: 'q1', filaments });

	test('setFilament sans trou : un index au-delà de la fin ajoute en queue', () => {
		expect(setFilament(emptyActivePresets(), 'f1', 3).filaments).toEqual(['f1']);
		expect(setFilament(sel(['a']), 'b', 5).filaments).toEqual(['a', 'b']);
		expect(setFilament(sel(['a', 'b']), 'c', 1).filaments).toEqual(['a', 'c']);
	});

	test('filamentSlotCount : au moins un slot affiché', () => {
		expect(filamentSlotCount(emptyActivePresets())).toBe(1);
		expect(filamentSlotCount(sel(['a', 'b', 'c']))).toBe(3);
	});

	test('filamentAt / clampFilamentSlot', () => {
		const a = sel(['a', 'b']);
		expect(filamentAt(a, 1)).toBe('b');
		expect(filamentAt(a, 2)).toBeNull();
		expect(clampFilamentSlot(a, 5)).toBe(1);
		expect(clampFilamentSlot(a, -2)).toBe(0);
		expect(clampFilamentSlot(a, Number.NaN)).toBe(0);
		expect(clampFilamentSlot(emptyActivePresets(), 3)).toBe(0);
	});

	test('addFilamentSlot clone le dernier filament, sinon le repli', () => {
		const a = sel(['a', 'b']);
		const b = addFilamentSlot(a, 'x');
		expect(b.filaments).toEqual(['a', 'b', 'b']);
		expect(a.filaments).toEqual(['a', 'b']);
		expect(addFilamentSlot(emptyActivePresets(), 'x').filaments).toEqual(['x']);
		const none = emptyActivePresets();
		expect(addFilamentSlot(none)).toBe(none);
	});

	test('removeFilamentSlot garde au moins un slot', () => {
		expect(canRemoveFilamentSlot(sel(['a']), 0)).toBe(false);
		expect(removeFilamentSlot(sel(['a']), 0).filaments).toEqual(['a']);
		expect(canRemoveFilamentSlot(sel(['a', 'b']), 2)).toBe(false);
		expect(removeFilamentSlot(sel(['a', 'b', 'c']), 1).filaments).toEqual(['a', 'c']);
		expect(removeFilamentSlot(sel(['a', 'b', 'c']), 9).filaments).toEqual(['a', 'b', 'c']);
	});

	test('remapExtruderAfterRemoval conserve le filament de chaque objet', () => {
		// slots [a, b, c] → retrait de b (slot 1) → [a, c]
		expect(remapExtruderAfterRemoval(0, 1)).toBe(0); // auto reste auto
		expect(remapExtruderAfterRemoval(1, 1)).toBe(1); // a reste extrudeur 1
		expect(remapExtruderAfterRemoval(2, 1)).toBe(0); // b retiré → auto
		expect(remapExtruderAfterRemoval(3, 1)).toBe(2); // c passe en extrudeur 2
	});

	test('clampExtruder : hors slots → auto', () => {
		expect(clampExtruder(0, 2)).toBe(0);
		expect(clampExtruder(2, 2)).toBe(2);
		expect(clampExtruder(3, 2)).toBe(0);
		expect(clampExtruder(-1, 2)).toBe(0);
	});

	test('slotAfterRemoval suit le filament sélectionné', () => {
		expect(slotAfterRemoval(2, 1, 2)).toBe(1); // c sélectionné reste sélectionné
		expect(slotAfterRemoval(0, 1, 2)).toBe(0);
		expect(slotAfterRemoval(1, 1, 2)).toBe(1); // slot retiré → voisin suivant
		expect(slotAfterRemoval(2, 2, 2)).toBe(1); // dernier retiré → nouveau dernier
	});

	test('persistance : ajout/suppression puis aller-retour du blob', () => {
		let a = sel(['a']);
		a = addFilamentSlot(a);
		a = setFilament(a, 'b', 1);
		a = addFilamentSlot(a);
		a = setFilament(a, 'c', 2);
		a = removeFilamentSlot(a, 0);
		const blob = serializeActivePresets(a);
		expect(blob).toEqual({ printer: 'p1', process: 'q1', filaments: ['b', 'c'] });
		const back = parseActivePresets(JSON.parse(JSON.stringify(blob)));
		expect(back).toEqual(a);
		expect(primaryFilament(back)).toBe('b');
	});
});

describe('importedFilamentSlots (projet 3MF ouvert)', () => {
	const presets = [
		{ id: 'pla', name: 'Bambu PLA Basic @BBL H2D' },
		{ id: 'petg', name: 'Bambu PETG HF @BBL H2D' }
	];
	test('un slot par couleur du projet, presets retrouvés par nom', () => {
		const a = { printer: 'm', process: 'p', filaments: [] };
		const out = importedFilamentSlots(
			a,
			2,
			['Bambu PETG HF @BBL H2D', 'Bambu PLA Basic @BBL H2D'],
			presets,
			'pla'
		);
		expect(out?.filaments).toEqual(['petg', 'pla']);
	});
	test('garde les slots existants et complète avec le dernier filament si le nom est inconnu', () => {
		const a = { printer: 'm', process: 'p', filaments: ['pla'] };
		expect(
			importedFilamentSlots(a, 3, ['x', 'Inconnu', 'Bambu PETG HF @BBL H2D'], presets)?.filaments
		).toEqual(['pla', 'pla', 'petg']);
	});
	test('rien à faire quand les slots couvrent déjà les couleurs ou sans aucune source', () => {
		expect(
			importedFilamentSlots({ printer: null, process: null, filaments: ['a', 'b'] }, 2, [], presets)
		).toBeNull();
		expect(
			importedFilamentSlots({ printer: null, process: null, filaments: [] }, 2, [], [])
		).toBeNull();
	});
});
