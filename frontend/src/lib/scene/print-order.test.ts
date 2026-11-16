import { describe, expect, it } from 'vitest';
import {
	isByObject,
	mergeInRank,
	moveToTarget,
	neighbourOf,
	orderFromPlacements,
	permuteInSlots,
	sequentialArrangeFields
} from './print-order';

const k = (s: string) => s;

describe('ordre d impression séquentielle', () => {
	it('glisser vers le bas place l objet après la cible (Orca OnDrop)', () => {
		expect(moveToTarget(['a', 'b', 'c', 'd'], 'a', 'c', k)).toEqual(['b', 'c', 'a', 'd']);
	});
	it('glisser vers le haut place l objet avant la cible', () => {
		expect(moveToTarget(['a', 'b', 'c', 'd'], 'd', 'b', k)).toEqual(['a', 'd', 'b', 'c']);
	});
	it('ids absents ou identiques : inchangé', () => {
		expect(moveToTarget(['a', 'b'], 'a', 'a', k)).toEqual(['a', 'b']);
		expect(moveToTarget(['a', 'b'], 'x', 'a', k)).toEqual(['a', 'b']);
	});
	it('voisin parmi les objets retenus (clavier)', () => {
		const list = ['a', 'p', 'b', 'c'];
		const among = (s: string) => s !== 'p';
		expect(neighbourOf(list, 'b', -1, k, among)).toBe('a');
		expect(neighbourOf(list, 'b', 1, k, among)).toBe('c');
		expect(neighbourOf(list, 'a', -1, k, among)).toBeNull();
	});
	it('restauration : objets classés remis dans l ordre du document', () => {
		const rank: Record<string, number> = { a: 0, b: 1, c: 2 };
		const out = mergeInRank(['c', 'x'], ['a', 'b'], (s) => rank[s]);
		expect(out).toEqual(['a', 'x', 'b', 'c']);
	});
	it('arrangement séquentiel : champs machine prioritaires, défauts Orca', () => {
		expect(
			sequentialArrangeFields(
				{ extruder_clearance_radius: 65 },
				{ extruder_clearance_radius: 40, nozzle_height: '4' }
			)
		).toEqual({
			is_seq_print: true,
			extruder_clearance_radius: 65,
			extruder_clearance_height_to_rod: 40,
			extruder_clearance_height_to_lid: 120,
			nozzle_height: 4
		});
	});
	it('ordre renvoyé par l arrangement : complet ou rien', () => {
		expect(
			orderFromPlacements([
				{ id: 'a', print_order: 2 },
				{ id: 'b', print_order: 1 }
			])
		).toEqual(['b', 'a']);
		expect(orderFromPlacements([{ id: 'a', print_order: 2 }, { id: 'b' }])).toBeNull();
		expect(orderFromPlacements([])).toBeNull();
	});
	it('permutation dans les emplacements des éléments listés', () => {
		expect(permuteInSlots(['a', 'g', 'b', 'c'], ['c', 'a', 'b'], (s) => s)).toEqual([
			'c',
			'g',
			'a',
			'b'
		]);
	});
	it('séquence effective : plateau prioritaire, sinon traitement', () => {
		expect(isByObject(undefined, 'by object')).toBe(true);
		expect(isByObject('by layer', 'by object')).toBe(false);
		expect(isByObject('by object', 'by layer')).toBe(true);
		expect(isByObject('', 'by layer')).toBe(false);
	});
});
