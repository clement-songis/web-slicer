import { describe, expect, it } from 'vitest';
import type { JobResponse } from '#lib/api/types';
import { jobAsEvent, planResume, resumedProgress } from './resume-job';

const job = (
	id: string,
	status: string,
	created: string,
	extra: Partial<JobResponse> = {}
): JobResponse => ({
	id,
	project_id: 'p1',
	plate_index: BigInt(0),
	status,
	progress: 0,
	phase: '',
	created_at: created,
	updated_at: created,
	...extra
});

describe('reprise après rechargement', () => {
	it('reprend le tranchage en cours plus récent que le dernier résultat', () => {
		const plan = planResume(
			[
				job('old', 'succeeded', '2026-01-01T00:00:01Z', { gcode_id: 'g' }),
				job('run', 'running', '2026-01-01T00:00:05Z', { progress: 0.4 }),
				job('other', 'running', '2026-01-01T00:00:06Z', { project_id: 'p2' })
			],
			'p1'
		);
		expect(plan?.kind).toBe('slicing');
		expect(plan && plan.kind === 'slicing' && plan.jobs.map((j) => j.id)).toEqual(['run']);
	});
	it('ignore un job actif plus ancien que le dernier résultat ; sinon le résultat', () => {
		const plan = planResume(
			[
				job('stale', 'queued', '2026-01-01T00:00:01Z'),
				job('done', 'succeeded', '2026-01-01T00:00:03Z', { gcode_id: 'g' })
			],
			'p1'
		);
		expect(plan).toEqual({ kind: 'result', job: expect.objectContaining({ id: 'done' }) });
		expect(planResume([job('x', 'failed', '1')], 'p1')).toBeNull();
	});
	it('état d’un job → événement de resynchronisation', () => {
		expect(jobAsEvent(job('a', 'succeeded', '1', { gcode_id: 'g' }))).toEqual({
			event: 'job.finished',
			id: 'a',
			gcode_id: 'g',
			stats: null
		});
		expect(jobAsEvent(job('b', 'failed', '1', { error: 'boom' }))).toMatchObject({
			event: 'job.updated',
			status: 'failed',
			error: 'boom'
		});
	});
	it('progression moyenne', () => {
		expect(
			resumedProgress([
				job('a', 'running', '1', { progress: 0.5 }),
				job('b', 'queued', '1', { progress: 0 })
			])
		).toBe(0.25);
		expect(resumedProgress([])).toBe(0);
	});
});
