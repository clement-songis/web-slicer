import { describe, expect, test } from 'vitest';
import {
	applyBatchEvent,
	batchFinished,
	batchLabel,
	makeBatch,
	resultFor,
	type BatchState
} from './plate-results';
import { startSlicing } from './session';
import type { ServerEvent } from '../api/types';

const PLATES = ['p1', 'p2', 'p3'];
const jobs = [
	{ id: 'j1', plate_index: 0n },
	{ id: 'j3', plate_index: 2n }
];
const fin = (id: string, gcode = 'g-' + id): ServerEvent => ({
	event: 'job.finished',
	id,
	gcode_id: gcode,
	stats: { id }
});
const upd = (id: string, progress: number, status = 'running'): ServerEvent => ({
	event: 'job.updated',
	id,
	status,
	progress,
	phase: 'slice',
	...(status === 'failed' ? { error: 'E engine_crashed Errors' } : {})
});
function start(): BatchState {
	return {
		session: startSlicing(['j1', 'j3']),
		batch: makeBatch(jobs, PLATES, 'sig'),
		results: {}
	};
}

describe('Trancher tout : un résultat par plateau (phase 25)', () => {
	test('lot : plateaux non vides seulement (plate_index → onglet)', () => {
		const b = makeBatch(jobs, PLATES, 'sig');
		expect(b.jobPlates).toEqual({ j1: 'p1', j3: 'p3' });
		expect(b.plateOrder).toEqual(['p1', 'p3']);
	});

	test('aperçu ouvert quand le plateau actif est prêt ; l’autre plateau est mémorisé ensuite', () => {
		let s = start();
		// p3 fini d'abord : l'onglet actif (p1) n'est pas prêt → toujours en tranchage.
		let step = applyBatchEvent(s, 'p1', fin('j3'));
		expect(step.session.phase).toBe('slicing');
		expect(step.session.progress).toBeCloseTo(0.5);
		expect(step.results.p3.gcodeId).toBe('g-j3');
		s = step;
		step = applyBatchEvent(s, 'p1', fin('j1'));
		expect(step.session.phase).toBe('preview');
		expect(step.session.gcodeId).toBe('g-j1');
		expect(step.showPlate).toBeNull();
		expect(batchFinished(step.batch)).toBe(true);
		expect(batchLabel(step.batch)).toBe('2 / 2 plateaux tranchés');
	});

	test('onglet actif vide (hors lot) : premier plateau tranché affiché, onglet basculé', () => {
		const step = applyBatchEvent(start(), 'p2', fin('j3'));
		expect(step.session.phase).toBe('preview');
		expect(step.session.gcodeId).toBe('g-j3');
		expect(step.showPlate).toBe('p3');
	});

	test('échec d’un autre plateau : le lot continue ; échec du plateau actif : erreur', () => {
		let step = applyBatchEvent(start(), 'p1', upd('j3', 0.4, 'failed'));
		expect(step.session.phase).toBe('slicing');
		expect(step.batch.errors.p3).toMatch(/Errors/);
		step = applyBatchEvent(step, 'p1', fin('j1'));
		expect(step.session.phase).toBe('preview');
		expect(batchLabel(step.batch)).toBe('1 / 2 plateaux tranchés · 1 en échec');
		const failed = applyBatchEvent(start(), 'p1', upd('j1', 0.2, 'failed'));
		expect(failed.session.phase).toBe('error');
	});

	test('après l’aperçu, les plateaux suivants sont encore mémorisés', () => {
		let step = applyBatchEvent(start(), 'p1', fin('j1'));
		expect(step.session.phase).toBe('preview');
		step = applyBatchEvent(step, 'p1', fin('j3'));
		expect(step.session.gcodeId).toBe('g-j1');
		expect(step.results.p3.gcodeId).toBe('g-j3');
	});

	test('progression agrégée et événements étrangers ignorés', () => {
		let step = applyBatchEvent(start(), 'p1', upd('j1', 0.5));
		expect(step.session.progress).toBeCloseTo(0.25);
		step = applyBatchEvent(step, 'p1', fin('autre'));
		expect(step.results).toEqual({});
	});

	test('résultat mémorisé : valable tant que la signature ne change pas', () => {
		const step = applyBatchEvent(start(), 'p1', fin('j1'));
		expect(resultFor(step.results, 'p1', 'sig')?.gcodeId).toBe('g-j1');
		expect(resultFor(step.results, 'p1', 'autre')).toBeNull();
		expect(resultFor(step.results, 'p2', 'sig')).toBeNull();
		expect(resultFor(step.results, null, 'sig')).toBeNull();
	});
});
