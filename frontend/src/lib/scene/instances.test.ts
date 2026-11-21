import { describe, expect, it } from 'vitest';
import {
	instanceCountPlan,
	instanceGroups,
	instancePositions,
	instanceStep,
	parseInstanceCount,
	planInstanceSync
} from './instances';

describe('instances (Plater::increase_instances / set_number_of_copies)', () => {
	it('pas = 5 % du plus grand côté du plateau (P1S 256 → 12,8 mm)', () => {
		expect(instanceStep({ width: 256, depth: 256 })).toBeCloseTo(12.8);
		expect(instanceStep({ width: 350, depth: 300 })).toBeCloseTo(17.5);
	});
	it('escalier depuis la dernière instance, jamais deux au même endroit', () => {
		const p = instancePositions([128, 128, 10], 2, 12.8);
		expect(p).toEqual([
			[140.8, 140.8, 10],
			[153.6, 153.6, 10]
		]);
	});
	it('saisie 0–1000, entiers seulement', () => {
		expect(parseInstanceCount('3')).toBe(3);
		expect(parseInstanceCount(' 0 ')).toBe(0);
		expect(parseInstanceCount('1000')).toBe(1000);
		expect(parseInstanceCount('1001')).toBeNull();
		expect(parseInstanceCount('2,5')).toBeNull();
		expect(parseInstanceCount('-1')).toBeNull();
		expect(parseInstanceCount('abc')).toBeNull();
	});
	it('plan : ajouter, retirer, supprimer l’objet à 0', () => {
		expect(instanceCountPlan(1, 3)).toEqual({ add: 2, remove: 0, deleteObject: false });
		expect(instanceCountPlan(4, 2)).toEqual({ add: 0, remove: 2, deleteObject: false });
		expect(instanceCountPlan(2, 0)).toEqual({ add: 0, remove: 2, deleteObject: true });
		expect(instanceCountPlan(2, 2)).toEqual({ add: 0, remove: 0, deleteObject: false });
	});
	it('groupes : membres ≥ 2 seulement', () => {
		const g = instanceGroups([
			{ id: 'a', instanceGroup: 'g1' },
			{ id: 'b' },
			{ id: 'c', instanceGroup: 'g1' },
			{ id: 'd', instanceGroup: 'g2' }
		]);
		expect([...g]).toEqual([['g1', ['a', 'c']]]);
	});
	it('synchronisation : le membre modifié fait foi', () => {
		const last = new Map<string, string>();
		const sig: Record<string, string> = { a: 'x', c: 'x' };
		const groups = new Map([['g1', ['a', 'c']]]);
		expect(planInstanceSync(groups, (id) => sig[id], last)).toEqual([]);
		sig.c = 'y';
		expect(planInstanceSync(groups, (id) => sig[id], last)).toEqual([
			{ source: 'c', targets: ['a'] }
		]);
		sig.a = 'y';
		expect(planInstanceSync(groups, (id) => sig[id], last)).toEqual([]);
		expect(planInstanceSync(new Map(), (id) => sig[id], last)).toEqual([]);
		expect(last.size).toBe(0);
	});
});
