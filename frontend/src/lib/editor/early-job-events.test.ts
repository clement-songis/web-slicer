// Phase 41c : événements de job reçus avant la réponse de POST /slice.
import { describe, expect, it } from 'vitest';
import type { ServerEvent } from '../api/types';
import { EarlyJobEvents } from './early-job-events';
import { applyJobEvent, startSlicing } from './session';

const updated = (id: string, progress: number): ServerEvent => ({
	event: 'job.updated',
	id,
	status: 'running',
	progress,
	phase: 'slice'
});
const finished = (id: string): ServerEvent => ({ event: 'job.finished', id, gcode_id: 'g-' + id, stats: null });
const none = () => false;

describe('EarlyJobEvents', () => {
	it('rejoue, dans l’ordre, un job fini avant la création de la session', () => {
		const early = new EarlyJobEvents();
		early.offer(updated('j1', 0.5), none);
		early.offer(finished('j1'), none);
		let session = startSlicing(['j1']);
		for (const e of early.take(['j1'])) session = applyJobEvent(session, e);
		expect(session.phase).toBe('preview');
		expect(session.gcodeId).toBe('g-j1');
		expect(early.size).toBe(0);
	});

	it('ignore les jobs déjà suivis et les événements hors jobs', () => {
		const early = new EarlyJobEvents();
		early.offer(finished('j1'), (id) => id === 'j1');
		early.offer({ event: 'model.converted', model_id: 'm', mesh_url: '/m' }, none);
		expect(early.size).toBe(0);
	});

	it('take ne rend que les jobs demandés, une seule fois', () => {
		const early = new EarlyJobEvents();
		early.offer(finished('a'), none);
		early.offer(finished('b'), none);
		expect(early.take(['b'])).toEqual([finished('b')]);
		expect(early.take(['b'])).toEqual([]);
		expect(early.size).toBe(1);
	});

	it('reste borné (jobs les plus anciens oubliés, derniers événements gardés)', () => {
		const early = new EarlyJobEvents(2, 2);
		early.offer(finished('a'), none);
		early.offer(finished('b'), none);
		early.offer(finished('c'), none);
		expect(early.take(['a'])).toEqual([]);
		for (let i = 1; i <= 3; i++) early.offer(updated('c', i / 10), none);
		const kept = early.take(['c']);
		expect(kept).toHaveLength(2);
		expect(kept.at(-1)).toEqual(updated('c', 0.3));
	});
});
