// Audit 45 MM-b — séquences de filaments du plateau (PlateSettingsDialog d'Orca).
import { describe, expect, test } from 'vitest';
import {
	MAX_LAYER_VALUE,
	formatOrder,
	formatOtherLayersSequence,
	moveItem,
	newRange,
	normalizeOrder,
	parseFirstLayerSequence,
	parseOtherLayersSequence,
	rangeValid,
	sortRanges
} from './plate-sequence';

describe('1ʳᵉ couche', () => {
	test('absent / « 0 » = Auto', () => {
		expect(parseFirstLayerSequence(undefined, 3)).toBeNull();
		expect(parseFirstLayerSequence('0', 3)).toBeNull();
	});
	test('texte Orca, ordre complété (update_first_layer_print_sequence)', () => {
		expect(parseFirstLayerSequence('3,1', 3)).toEqual([3, 1, 2]);
		expect(parseFirstLayerSequence([2, 2, 9, 1], 2)).toEqual([2, 1]);
		expect(formatOrder([3, 1, 2])).toBe('3,1,2');
	});
	test('normalizeOrder retire doublons et numéros hors bornes', () => {
		expect(normalizeOrder([4, 2, 2, 0], 3)).toEqual([2, 1, 3]);
	});
});

describe('autres couches', () => {
	test('aller-retour [début, fin, ordre…] × nums (ParameterUtils)', () => {
		const ranges = [
			{ begin: 2, end: 10, order: [2, 1] },
			{ begin: 11, end: MAX_LAYER_VALUE, order: [1, 2] }
		];
		const out = formatOtherLayersSequence(ranges);
		expect(out).toEqual({ sequence: `2,10,2,1,11,${MAX_LAYER_VALUE},1,2`, nums: '2' });
		expect(parseOtherLayersSequence(out.sequence, out.nums, 2)).toEqual(ranges);
	});
	test('incohérent → aucune plage', () => {
		expect(parseOtherLayersSequence('2,10,1', '2', 2)).toEqual([]);
		expect(parseOtherLayersSequence(undefined, undefined, 2)).toEqual([]);
	});
	test('première plage « couche 2 → Fin », validité, tri', () => {
		expect(newRange(0, 2)).toEqual({ begin: 2, end: MAX_LAYER_VALUE, order: [1, 2] });
		expect(rangeValid({ begin: 1, end: 5, order: [] })).toBe(false);
		expect(rangeValid({ begin: 6, end: 5, order: [] })).toBe(false);
		expect(rangeValid({ begin: 2, end: 2, order: [] })).toBe(true);
		expect(
			sortRanges([
				{ begin: 5, end: 9, order: [] },
				{ begin: 2, end: 4, order: [] }
			]).map((r) => r.begin)
		).toEqual([2, 5]);
	});
	test('moveItem (glisser-déposer)', () => {
		expect(moveItem([1, 2, 3], 2, 0)).toEqual([3, 1, 2]);
		expect(moveItem([1, 2, 3], 0, 5)).toEqual([1, 2, 3]);
	});
});
