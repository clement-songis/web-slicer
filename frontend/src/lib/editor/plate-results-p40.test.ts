// Phase 40 (audit B1) : signature de tranchage par plateau et résultats périmés.
import { describe, expect, test } from 'vitest';
import { applyBatchEvent, makeBatch, plateSceneSignature, staleResults } from './plate-results';
import { prepareSession } from './session';

const scene = (
	objects: { id: string; x: number }[],
	plates = [
		{ id: 'p1', name: 'A', objectIds: ['a', 'b'] },
		{ id: 'p2', name: 'B', objectIds: ['c'] }
	]
) => ({ schemaVersion: 1, plates: { plates, activeId: 'p1' }, objects });

describe('plateSceneSignature', () => {
	const base = scene([
		{ id: 'a', x: 0 },
		{ id: 'b', x: 1 },
		{ id: 'c', x: 2 }
	]);
	test('change quand un objet du plateau est supprimé ou déplacé', () => {
		const s = plateSceneSignature(base, 'p1');
		const removed = scene(
			[
				{ id: 'a', x: 0 },
				{ id: 'c', x: 2 }
			],
			[
				{ id: 'p1', name: 'A', objectIds: ['a'] },
				{ id: 'p2', name: 'B', objectIds: ['c'] }
			]
		);
		expect(plateSceneSignature(removed, 'p1')).not.toBe(s);
		const moved = scene([
			{ id: 'a', x: 5 },
			{ id: 'b', x: 1 },
			{ id: 'c', x: 2 }
		]);
		expect(plateSceneSignature(moved, 'p1')).not.toBe(s);
	});
	test('ne change pas pour un autre plateau, le plateau actif ou le nom', () => {
		const s = plateSceneSignature(base, 'p1');
		const other = scene([
			{ id: 'a', x: 0 },
			{ id: 'b', x: 1 },
			{ id: 'c', x: 9 }
		]);
		expect(plateSceneSignature(other, 'p1')).toBe(s);
		expect(plateSceneSignature(other, 'p2')).not.toBe(plateSceneSignature(base, 'p2'));
		const renamed = {
			...base,
			plates: {
				plates: [{ id: 'p1', name: 'Z', objectIds: ['a', 'b'] }, base.plates.plates[1]],
				activeId: 'p2'
			}
		};
		expect(plateSceneSignature(renamed, 'p1')).toBe(s);
	});
	test('plateau inconnu → chaîne vide', () => {
		expect(plateSceneSignature(base, 'nope')).toBe('');
	});
});

describe('staleResults / makeBatch par plateau', () => {
	test('résultat dont la signature diffère = périmé ; sans signature = conservé', () => {
		const results = {
			p1: { gcodeId: 'g1', jobId: 'j1', stats: null, signature: 's1' },
			p2: { gcodeId: 'g2', jobId: 'j2', stats: null, signature: 's2' },
			p3: { gcodeId: 'g3', jobId: '', stats: null, signature: null }
		};
		expect(staleResults(results, (id) => (id === 'p1' ? 's1' : 'x'))).toEqual(['p2']);
	});
	test('le résultat d’un job porte la signature de SON plateau', () => {
		const batch = makeBatch([{ id: 'j1', plate_index: 1 }], ['p1', 'p2'], 'global', {
			p1: 'a',
			p2: 'b'
		});
		const step = applyBatchEvent(
			{ session: { ...prepareSession(), phase: 'slicing', jobIds: ['j1'] }, batch, results: {} },
			'p2',
			{ event: 'job.finished', id: 'j1', gcode_id: 'g', stats: null } as never
		);
		expect(step.results.p2.signature).toBe('b');
	});
});
