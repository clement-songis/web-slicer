import { describe, expect, test } from 'vitest';
import {
	cutLinkedIds,
	equalCutId,
	invalidatedIds,
	parseCutId,
	sameCutId,
	syncAfterCut,
	uniformCutScale,
	withCutPartners
} from './cut-info';

const a = { id: 7, checkSum: 2, connectorsCnt: 1 };
const nodes = [
	{ id: 'up', cutId: a },
	{ id: 'low', cutId: { ...a } },
	{ id: 'old', cutId: { id: 7, checkSum: 1, connectorsCnt: 0 } },
	{ id: 'other', cutId: { id: 9, checkSum: 1, connectorsCnt: 0 } },
	{ id: 'plain' }
];

describe('informations de coupe (cut_id d’Orca)', () => {
	test('lecture tolérante : id 0 / absent / invalide → null', () => {
		expect(parseCutId({ id: 3, checkSum: 1, connectorsCnt: 2 })).toEqual({
			id: 3,
			checkSum: 1,
			connectorsCnt: 2
		});
		expect(parseCutId({ id: 0, checkSum: 1 })).toBeNull();
		expect(parseCutId(null)).toBeNull();
		expect(parseCutId([1])).toBeNull();
		expect(parseCutId({ id: 4 })).toEqual({ id: 4, checkSum: 0, connectorsCnt: 0 });
	});

	test('has_same_id / is_equal', () => {
		expect(sameCutId(a, nodes[2].cutId)).toBe(true);
		expect(equalCutId(a, nodes[2].cutId)).toBe(false);
		expect(equalCutId(a, nodes[1].cutId)).toBe(true);
		expect(sameCutId(a, null)).toBe(false);
	});

	test('pièces liées (fix_cut_selection) : même id, sélection complétée sans doublon', () => {
		expect(cutLinkedIds(nodes, 'up')).toEqual(['up', 'low', 'old']);
		expect(cutLinkedIds(nodes, 'plain')).toEqual(['plain']);
		expect(withCutPartners(nodes, ['plain', 'low', 'up'])).toEqual(['plain', 'up', 'low', 'old']);
	});

	test('invalider : seulement les objets de cut_id égal (invalidate_cut_info_for_object)', () => {
		expect(invalidatedIds(nodes, 'up')).toEqual(['up', 'low']);
		expect(invalidatedIds(nodes, 'plain')).toEqual([]);
	});

	test('échelle uniforme : l’axe modifié donne le facteur des trois axes', () => {
		expect(uniformCutScale([1, 1, 1], [2, 1, 1])).toEqual([2, 2, 2]);
		expect(uniformCutScale([2, 2, 2], [2, 2, 1])).toEqual([1, 1, 1]);
		expect(uniformCutScale([1, 1, 1], [1, 1, 1])).toEqual([1, 1, 1]);
	});

	test('synchronize_model_after_cut : les autres objets de même id reçoivent le nouveau cut_id', () => {
		expect(syncAfterCut(nodes, { id: 7, checkSum: 3, connectorsCnt: 1 }, ['up'])).toEqual([
			'low',
			'old'
		]);
		expect(syncAfterCut(nodes, null, [])).toEqual([]);
	});
});
