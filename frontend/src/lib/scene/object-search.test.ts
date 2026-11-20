import { describe, expect, test } from 'vitest';
import { searchObjects } from './object-search';

const nodes = [
	{ id: 'g', name: 'Groupe', parentId: null },
	{ id: 'a', name: 'Benchy', parentId: 'g' },
	{ id: 'b', name: 'Cube calibration', parentId: null },
	{ id: 'c', name: 'Écrou', parentId: null }
];
const plates = [
	{ id: 'p1', name: 'Plateau 1', objectIds: ['g', 'b'] },
	{ id: 'p2', name: 'Pièces PETG', objectIds: ['c'] }
];

describe("recherche dans la liste d'objets (Orca m_search_bar)", () => {
	test('vide = pas de filtre', () => {
		expect(searchObjects(nodes, '  ')).toBeNull();
	});
	test('nom insensible casse/accents ; ancêtres gardés, seul le résultat surligné', () => {
		const r = searchObjects(nodes, 'BENCH', plates)!;
		expect([...r.shown].sort()).toEqual(['a', 'g']);
		expect([...r.matched]).toEqual(['a']);
		expect(r.hits[0]).toMatchObject({ id: 'a', plateId: 'p1', via: 'object' });
		expect([...searchObjects(nodes, 'ecrou', plates)!.matched]).toEqual(['c']);
	});
	test('plusieurs mots : tous requis ; plateau et pièces cherchés aussi', () => {
		expect(searchObjects(nodes, 'cube calib', plates)!.hits.map((h) => h.id)).toEqual(['b']);
		expect(searchObjects(nodes, 'cube benchy', plates)!.hits).toEqual([]);
		const byPlate = searchObjects(nodes, 'petg', plates)!;
		expect(byPlate.hits).toEqual([
			{ id: 'c', name: 'Écrou', plateId: 'p2', plateName: 'Pièces PETG', via: 'plate' }
		]);
		const byPart = searchObjects(nodes, 'aimant', plates, { b: ['Cube_1', 'Aimant'] })!;
		expect(byPart.hits[0]).toMatchObject({ id: 'b', via: 'part', part: 'Aimant' });
	});
});
