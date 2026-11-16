// Tests de la réparation de maillage côté éditeur (FR-012).
import { describe, expect, test } from 'vitest';
import type { ImportItem } from '../editor/imports';
import type { RepairResponse } from '../api/types';
import type { SimplifyResponse } from '../api/scene';
import {
	instancesOfModel,
	repairCount,
	repairSummary,
	repairTarget,
	replaceMesh,
	simplifyRequest,
	simplifySummary
} from './repair';

const item = (
	objectId: string,
	modelId: string | null,
	status: ImportItem['status']
): ImportItem => ({
	objectId,
	filename: 'cube.stl',
	status,
	modelId,
	error: null
});

const report = (over: Partial<RepairResponse> = {}): RepairResponse => ({
	triangles: 12,
	degenerate: 0,
	open_edges: 0,
	watertight: true,
	repairs: {
		edges_fixed: 2,
		degenerate_facets: 1,
		facets_removed: 0,
		facets_reversed: 3,
		backwards_edges: 0
	},
	...over
});

describe('repairTarget', () => {
	test('modèle prêt → réparable', () => {
		expect(repairTarget('o1', [item('o1', 'm1', 'ready')])).toEqual({ ok: true, modelId: 'm1' });
	});
	test('refuse sans modèle, en conversion ou en échec', () => {
		expect(repairTarget('o1', []).ok).toBe(false);
		expect(repairTarget('o1', [item('o1', null, 'uploading')]).ok).toBe(false);
		expect(repairTarget('o1', [item('o1', 'm1', 'converting')]).ok).toBe(false);
		expect(repairTarget('o1', [item('o1', 'm1', 'failed')]).ok).toBe(false);
	});
});

describe('replaceMesh', () => {
	test('remplace le maillage de toutes les instances en conservant transformations', () => {
		const oldMesh = { tag: 'old' };
		const newMesh = { tag: 'new' };
		const other = {
			id: 'x',
			mesh: { tag: 'other' },
			position: [9, 9, 9] as [number, number, number]
		};
		const objects = [
			{
				id: 'a',
				mesh: oldMesh,
				position: [1, 2, 3] as [number, number, number],
				rotation: [0, 0, 90] as [number, number, number],
				scale: [2, 2, 2] as [number, number, number]
			},
			{ id: 'b', mesh: oldMesh, position: [4, 5, 6] as [number, number, number] },
			other
		];
		const ids = instancesOfModel(
			[item('a', 'm1', 'ready'), item('b', 'm1', 'ready'), item('x', 'm2', 'ready')],
			'm1'
		);
		const out = replaceMesh(objects, ids, newMesh);
		expect(out[0]).toEqual({ ...objects[0], mesh: newMesh });
		expect(out[1]).toEqual({ ...objects[1], mesh: newMesh });
		expect(out[2]).toBe(other);
		expect(objects[0].mesh).toBe(oldMesh);
	});
});

describe('repairSummary', () => {
	test('compte les corrections et décrit l’état', () => {
		expect(repairCount(report())).toBe(6);
		expect(repairSummary(report())).toBe(
			'Maillage réparé : 6 correction(s), 12 triangles, étanche.'
		);
		const clean = report({
			watertight: false,
			open_edges: 4,
			repairs: {
				edges_fixed: 0,
				degenerate_facets: 0,
				facets_removed: 0,
				facets_reversed: 0,
				backwards_edges: 0
			}
		});
		expect(repairSummary(clean)).toBe(
			'Maillage réparé : aucune correction nécessaire, 12 triangles, non étanche (4 arête(s) de bord).'
		);
	});
});

describe('simplifyRequest', () => {
	test('ratio dans ]0, 100[ → corps serveur', () => {
		expect(simplifyRequest({ mode: 'ratio', decimateRatio: 50 })).toEqual({
			ok: true,
			body: { mode: 'ratio', decimate_ratio: 50 }
		});
	});
	test('niveau entier 0..4 → corps serveur', () => {
		expect(simplifyRequest({ mode: 'detail', level: 4 })).toEqual({
			ok: true,
			body: { mode: 'detail', level: 4 }
		});
	});
	test('refuse ratios et niveaux hors bornes', () => {
		for (const r of [0, 100, -1, Number.NaN, Infinity]) {
			expect(simplifyRequest({ mode: 'ratio', decimateRatio: r }).ok).toBe(false);
		}
		for (const l of [-1, 5, 1.5]) {
			expect(simplifyRequest({ mode: 'detail', level: l }).ok).toBe(false);
		}
	});
});

describe('simplifySummary', () => {
	const simplified = (over: Partial<SimplifyResponse> = {}): SimplifyResponse => ({
		triangles_before: 768,
		triangles_after: 384,
		vertices_before: 386,
		vertices_after: 194,
		max_error: 0,
		open_edges: 0,
		watertight: true,
		...over
	});
	test('décrit la réduction et l’état', () => {
		expect(simplifySummary(simplified())).toBe(
			'Maillage simplifié : 768 → 384 triangles (−50 %), étanche.'
		);
		expect(simplifySummary(simplified({ watertight: false, open_edges: 3 }))).toBe(
			'Maillage simplifié : 768 → 384 triangles (−50 %), non étanche (3 arête(s) de bord).'
		);
	});
	test('signale une simplification sans effet', () => {
		expect(simplifySummary(simplified({ triangles_after: 768 }))).toBe(
			'Simplification sans effet : 768 triangles conservés.'
		);
	});
});
