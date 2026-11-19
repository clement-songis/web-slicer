import { describe, expect, it, vi } from 'vitest';
import type { JobResponse } from '$lib/api/types';
import { subscribeEvents } from './events';
import { jobErrorText, phaseLabel, queuePositions, unknownJobId, upsertJob } from './queue';

const job = (id: string, status: string, created: string): JobResponse => ({
	id,
	project_id: 'p',
	plate_index: BigInt(0),
	status,
	progress: 0,
	phase: '',
	created_at: created,
	updated_at: created
});

describe('file : plusieurs jobs simultanés', () => {
	it('rang des jobs en file, par ordre d’arrivée', () => {
		const jobs = [
			job('c', 'queued', '2026-01-01T00:00:03Z'),
			job('a', 'running', '2026-01-01T00:00:01Z'),
			job('b', 'queued', '2026-01-01T00:00:02Z')
		];
		expect(queuePositions(jobs)).toEqual({ b: 1, c: 2 });
	});
	it('ajoute un job lancé ailleurs, remplace un job connu', () => {
		const list = [job('a', 'running', '1')];
		expect(upsertJob(list, job('b', 'queued', '2')).map((j) => j.id)).toEqual(['b', 'a']);
		expect(upsertJob(list, job('a', 'succeeded', '1'))[0].status).toBe('succeeded');
		expect(
			unknownJobId(list, {
				event: 'job.updated',
				id: 'b',
				status: 'running',
				progress: 0,
				phase: ''
			} as never)
		).toBe('b');
		expect(
			unknownJobId(list, {
				event: 'job.updated',
				id: 'a',
				status: 'running',
				progress: 0,
				phase: ''
			} as never)
		).toBeNull();
		expect(unknownJobId(list, { event: 'model.converted', model_id: 'm' } as never)).toBeNull();
	});
	it('phase et erreur lisibles', () => {
		expect(phaseLabel('slice')).toBe('tranchage');
		expect(phaseLabel('')).toBe('');
		expect(phaseLabel('autre chose')).toBe('autre chose');
		expect(jobErrorText('boom')).toBe('boom');
		expect(jobErrorText({ message: 'objet hors plateau' })).toBe('objet hors plateau');
		expect(jobErrorText(null)).toBe('');
	});
});

describe('flux d’événements : reconnexion signalée', () => {
	it('onOpen(false) puis onOpen(true) après une coupure', async () => {
		const sockets: { onopen?: () => void; onclose?: () => void; close: () => void }[] = [];
		vi.stubGlobal('location', { protocol: 'http:', host: 'app.local' });
		const opens: boolean[] = [];
		const sub = subscribeEvents({
			onEvent: () => {},
			onOpen: (r) => opens.push(r),
			reconnectDelayMs: 1,
			socketFactory: () => {
				const s = { close: () => {} } as (typeof sockets)[number];
				sockets.push(s);
				return s as unknown as WebSocket;
			}
		});
		sockets[0].onopen?.();
		sockets[0].onclose?.();
		await new Promise((r) => setTimeout(r, 10));
		sockets[1].onopen?.();
		expect(opens).toEqual([false, true]);
		sub.close();
		vi.unstubAllGlobals();
	});
});
