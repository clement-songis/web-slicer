// Tests de la logique pure de file (T071) : réducteur d'événements, partition
// active/historique, métadonnées d'état, progression, décodage d'événements.
import { describe, expect, test } from 'bun:test';
import type { JobResponse, ServerEvent } from '$lib/api/types';
import { applyEvent, isActive, partitionJobs, progressPercent, statusMeta } from './queue';
import { decodeEvent } from './events';

const job = (over: Partial<JobResponse> = {}): JobResponse => ({
	id: 'j1',
	project_id: 'p1',
	plate_index: 0n,
	status: 'queued',
	progress: 0,
	phase: '',
	created_at: '2026-07-11T10:00:00Z',
	updated_at: '2026-07-11T10:00:00Z',
	...over
});

describe('partitionJobs', () => {
	test('splits active from history, most recent first', () => {
		const jobs = [
			job({ id: 'a', status: 'queued', created_at: '2026-07-11T10:00:00Z' }),
			job({ id: 'b', status: 'succeeded', created_at: '2026-07-11T11:00:00Z' }),
			job({ id: 'c', status: 'running', created_at: '2026-07-11T12:00:00Z' }),
			job({ id: 'd', status: 'failed', created_at: '2026-07-11T09:00:00Z' })
		];
		const { active, history } = partitionJobs(jobs);
		expect(active.map((j) => j.id)).toEqual(['c', 'a']);
		expect(history.map((j) => j.id)).toEqual(['b', 'd']);
	});
});

describe('isActive', () => {
	test('queued and running are active; terminal states are not', () => {
		expect(isActive(job({ status: 'queued' }))).toBe(true);
		expect(isActive(job({ status: 'running' }))).toBe(true);
		expect(isActive(job({ status: 'succeeded' }))).toBe(false);
		expect(isActive(job({ status: 'cancelled' }))).toBe(false);
	});
});

describe('applyEvent', () => {
	const jobs = [job({ id: 'j1' }), job({ id: 'j2' })];

	test('job.updated patches status/progress/phase of the matching job', () => {
		const ev: ServerEvent = {
			event: 'job.updated',
			id: 'j1',
			status: 'running',
			progress: 0.42,
			phase: 'slicing'
		};
		const next = applyEvent(jobs, ev);
		expect(next[0]).toMatchObject({ status: 'running', progress: 0.42, phase: 'slicing' });
		expect(next[1]).toEqual(jobs[1]); // inchangé
	});

	test('job.finished marks succeeded with the gcode id', () => {
		const ev: ServerEvent = { event: 'job.finished', id: 'j2', gcode_id: 'g9', stats: {} };
		const next = applyEvent(jobs, ev);
		expect(next[1]).toMatchObject({ status: 'succeeded', progress: 1, gcode_id: 'g9' });
	});

	test('an event for an unknown job leaves the list unchanged', () => {
		const ev: ServerEvent = {
			event: 'job.updated',
			id: 'ghost',
			status: 'running',
			progress: 0.5,
			phase: 'x'
		};
		expect(applyEvent(jobs, ev)).toEqual(jobs);
	});

	test('model.converted does not affect the queue', () => {
		const ev: ServerEvent = { event: 'model.converted', model_id: 'm1', mesh_url: '/x' };
		expect(applyEvent(jobs, ev)).toEqual(jobs);
	});
});

describe('statusMeta & progressPercent', () => {
	test('known status yields a label and badge class', () => {
		expect(statusMeta('running').label).toBe('En cours');
		expect(statusMeta('failed').badge).toContain('danger');
	});

	test('unknown status falls back to a neutral badge', () => {
		expect(statusMeta('weird').label).toBe('weird');
	});

	test('progressPercent clamps to 0–100', () => {
		expect(progressPercent(job({ progress: 0.5 }))).toBe(50);
		expect(progressPercent(job({ progress: 2 }))).toBe(100);
		expect(progressPercent(job({ progress: -1 }))).toBe(0);
	});
});

describe('decodeEvent', () => {
	test('parses a valid tagged event', () => {
		const ev = decodeEvent('{"event":"job.finished","id":"j1","stats":{}}');
		expect(ev?.event).toBe('job.finished');
	});

	test('rejects non-string or malformed payloads', () => {
		expect(decodeEvent(123)).toBeNull();
		expect(decodeEvent('not json')).toBeNull();
		expect(decodeEvent('{"no":"tag"}')).toBeNull();
	});
});
