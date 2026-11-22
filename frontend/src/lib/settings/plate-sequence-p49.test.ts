// Audit 47 PLATE-b — plages « autres couches » : saisie validée ligne par ligne,
// tri d'Orca (LayerSeqInfo::operator<, champs vides en dernier), plage ajoutée
// vide (OtherLayersSeqPanel::append_layer), LayerNumberTextInput::validate_input_value.
import { describe, expect, test } from 'vitest';
import {
	EMPTY_LAYER,
	MAX_LAYER_VALUE,
	commitBegin,
	commitEnd,
	formatOtherLayersSequence,
	newRange,
	rangeValid,
	sortRanges,
	type LayerSeqRange
} from './plate-sequence';

type Row = LayerSeqRange & { id: number };
const row = (id: number, begin: number, end: number, order = [1, 2, 3, 4]): Row => ({
	id,
	begin,
	end,
	order
});

describe('append_layer', () => {
	test('1ʳᵉ plage « 2 → Fin », suivantes : début vide, fin « Fin »', () => {
		expect(newRange(0, 2)).toEqual({ begin: 2, end: MAX_LAYER_VALUE, order: [1, 2] });
		expect(newRange(1, 2)).toEqual({ begin: EMPTY_LAYER, end: MAX_LAYER_VALUE, order: [1, 2] });
		expect(rangeValid(newRange(1, 2))).toBe(false);
	});
});

describe('tri (LayerSeqInfo::operator<)', () => {
	test('début vide trié après les autres ; ordre stable', () => {
		const sorted = sortRanges([row(1, EMPTY_LAYER, MAX_LAYER_VALUE), row(2, 5, 9), row(3, 2, 4)]);
		expect(sorted.map((r) => r.id)).toEqual([3, 2, 1]);
	});
	test('même début : fin vide en dernier, sinon par fin', () => {
		const sorted = sortRanges([row(1, 2, EMPTY_LAYER), row(2, 2, 9), row(3, 2, 4)]);
		expect(sorted.map((r) => r.id)).toEqual([3, 2, 1]);
	});
});

describe('validate_input_value', () => {
	test('début : borné à ≥ 2, fin renseignée repoussée, vide → champ vide', () => {
		expect(commitBegin(row(1, 2, 4), '7')).toMatchObject({ begin: 7, end: 7 });
		expect(commitBegin(row(1, 2, MAX_LAYER_VALUE), '7')).toMatchObject({
			begin: 7,
			end: MAX_LAYER_VALUE
		});
		expect(commitBegin(row(1, 2, EMPTY_LAYER), '7')).toMatchObject({ begin: 7, end: EMPTY_LAYER });
		expect(commitBegin(row(1, 5, 9), '1')).toMatchObject({ begin: 2, end: 9 });
		expect(commitBegin(row(1, 5, 9), '')).toMatchObject({ begin: EMPTY_LAYER, end: 9 });
	});
	test('fin : inférieure au début refusée si renseignée, sinon = début', () => {
		expect(commitEnd(row(1, 5, 9), '3')).toBeNull();
		expect(commitEnd(row(1, 5, EMPTY_LAYER), '3')).toMatchObject({ begin: 5, end: 5 });
		expect(commitEnd(row(1, 5, 9), '12')).toMatchObject({ end: 12 });
		expect(commitEnd(row(1, EMPTY_LAYER, MAX_LAYER_VALUE), '4')).toMatchObject({ end: 4 });
		expect(commitEnd(row(1, 5, 9), '')).toMatchObject({ end: EMPTY_LAYER });
	});
	test("l'identité de ligne est conservée (lignes à clé stable)", () => {
		expect(commitBegin(row(42, 2, 4), '3').id).toBe(42);
		expect(commitEnd(row(42, 2, 4), '6')?.id).toBe(42);
	});
});

describe('scénario de l’audit 47 : 2→4, 5→9, 10→Fin', () => {
	test('valeurs saisies = valeurs enregistrées', () => {
		// Ouverture « Personnaliser » : 2 → Fin ; Ajouter × 2 (débuts vides).
		let rows: Row[] = [
			{ ...newRange(0, 4), id: 1 },
			{ ...newRange(1, 4), id: 2 },
			{ ...newRange(2, 4), id: 3 }
		];
		const edit = (id: number, f: (r: Row) => Row | null) => {
			rows = sortRanges(rows.map((r) => (r.id === id ? (f(r) ?? r) : r)));
		};
		// Ligne 1 : fin « Personnaliser » (champ vidé) puis 4, ordre 2,1,3,4.
		edit(1, (r) => ({ ...r, end: EMPTY_LAYER }));
		edit(1, (r) => commitEnd(r, '4'));
		edit(1, (r) => ({ ...r, order: [2, 1, 3, 4] }));
		// Ligne 2 : 5 → 9, ordre 4,1,2,3.
		edit(2, (r) => commitBegin(r, '5'));
		edit(2, (r) => ({ ...r, end: EMPTY_LAYER }));
		edit(2, (r) => commitEnd(r, '9'));
		edit(2, (r) => ({ ...r, order: [4, 1, 2, 3] }));
		// Ligne 3 : 10 → Fin, ordre 4,3,2,1.
		edit(3, (r) => commitBegin(r, '10'));
		edit(3, (r) => ({ ...r, order: [4, 3, 2, 1] }));
		expect(rows.map((r) => r.id)).toEqual([1, 2, 3]);
		expect(rows.every(rangeValid)).toBe(true);
		expect(formatOtherLayersSequence(rows)).toEqual({
			sequence: `2,4,2,1,3,4,5,9,4,1,2,3,10,${MAX_LAYER_VALUE},4,3,2,1`,
			nums: '3'
		});
	});
	test('une plage ajoutée ne passe pas devant la ligne éditée (note de l’audit 46)', () => {
		let rows: Row[] = [row(1, 2, 4), row(2, 5, 9)];
		rows = sortRanges([...rows, { ...newRange(rows.length, 4), id: 3 }]);
		expect(rows.map((r) => r.id)).toEqual([1, 2, 3]);
	});
});
