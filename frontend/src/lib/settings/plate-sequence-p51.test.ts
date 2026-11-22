// Audit 49 PLATE49-a — une plage « autres couches » laissée vide (ligne issue
// d'« Ajouter », jamais remplie) ne doit plus invalider l'application entière :
// seules les plages remplies (début saisi) sont validées et sérialisées ; les
// valeurs réellement invalides (fin vide/inférieure) gardent le message de
// saisie, et l'absence totale de plage remplie n'efface pas les réglages.
import { describe, expect, test } from 'vitest';
import {
	EMPTY_LAYER,
	MAX_LAYER_VALUE,
	filledLayerRanges,
	formatOtherLayersSequence,
	newRange,
	rangeValid,
	type LayerSeqRange
} from './plate-sequence';

type Row = LayerSeqRange & { id: number };
const row = (id: number, begin: number, end: number, order = [1, 2]): Row => ({
	id,
	begin,
	end,
	order
});

/** Ce que fait apply() du dialogue (PlateSettingsDialog.svelte) avec les plages. */
function applyRanges(ranges: Row[]): { error: boolean; sequence?: string; nums?: string } {
	const filled = filledLayerRanges(ranges);
	if (!filled.length || !filled.every(rangeValid)) return { error: true };
	const out = formatOtherLayersSequence(filled);
	return { error: false, sequence: out.sequence, nums: out.nums };
}

describe('PLATE49-a — plages vides ignorées à l’application', () => {
	test('2 plages remplies + 1 ligne vide (repro audit 49) : la ligne vide est écartée', () => {
		// Repro plate49 : 2→6 [2,1] et 7→Fin [1,2] remplies, 3ᵉ ligne jamais saisie.
		const ranges = [
			row(1, 2, 6, [2, 1]),
			row(2, 7, MAX_LAYER_VALUE, [1, 2]),
			{ ...newRange(2, 2), id: 3 } // begin EMPTY_LAYER, end « Fin », ordre par défaut
		];
		expect(filledLayerRanges(ranges).map((r) => r.id)).toEqual([1, 2]);
		const applied = applyRanges(ranges);
		expect(applied.error).toBe(false);
		expect(applied.sequence).toBe('2,6,2,1,7,2147483646,1,2');
		expect(applied.nums).toBe('2');
	});

	test('toutes les plages vides : message conservé, rien n’est appliqué (pas de vidage)', () => {
		const ranges = [{ ...newRange(0, 2), id: 1 }];
		// L'utilisateur a vidé le début de la seule plage initiale.
		ranges[0] = row(1, EMPTY_LAYER, MAX_LAYER_VALUE);
		expect(applyRanges(ranges).error).toBe(true);
		expect(filledLayerRanges(ranges)).toEqual([]);
	});

	test('valeurs invalides (début saisi, fin vide) : message de saisie conservé', () => {
		const ranges = [row(1, 2, EMPTY_LAYER), row(2, 5, 8)];
		expect(applyRanges(ranges).error).toBe(true);
		expect(rangeValid(ranges[0])).toBe(false);
	});

	test('l’ordre saisi des plages remplies est conservé, sans trou', () => {
		const ranges = [
			row(1, 2, 4, [2, 1]),
			{ ...newRange(1, 2), id: 2 }, // vide, au milieu
			row(3, 5, MAX_LAYER_VALUE, [1, 2])
		];
		expect(applyRanges(ranges).sequence).toBe('2,4,2,1,5,2147483646,1,2');
	});
});
