import { describe, expect, it } from 'vitest';
import { applyJobEvent, friendlySliceError } from './session';

const WORKER =
	'worker sorti en code 1 — stderr :  — stdout : E engine_crashed configuration invalide : cylinder.stl is too close to others, and collisions may be caused.\n';

describe('messages d échec du tranchage', () => {
	it('objets trop proches (objet par objet) : message traduit avec le nom', () => {
		const m = friendlySliceError(WORKER);
		expect(m).toContain('« cylinder.stl » est trop près des autres objets');
		expect(m).not.toContain('worker');
	});
	it('objet trop haut', () => {
		expect(
			friendlySliceError(
				'configuration invalide : tour.stl is too tall, and collisions will be caused.'
			)
		).toContain('« tour.stl » : trop haut');
	});
	it('plusieurs objets trop proches : tous nommés', () => {
		const m = friendlySliceError(
			'configuration invalide : cube20.stl is too close to others, and collisions may be caused. cylinder.stl is too close to others, and collisions may be caused.\n'
		);
		expect(m.startsWith('« cube20.stl », « cylinder.stl » sont trop près')).toBe(true);
	});
	it('autre erreur : enveloppe du worker retirée', () => {
		expect(
			friendlySliceError('worker sorti en code 1 — stderr :  — stdout : E engine_crashed boom')
		).toBe('boom');
		expect(friendlySliceError('message simple')).toBe('message simple');
	});
	it('erreur de job sous forme d objet { message }', () => {
		const session = {
			phase: 'slicing',
			jobIds: ['j'],
			progress: 0,
			jobPhase: '',
			warnings: [],
			error: null,
			gcodeId: null
		} as unknown as Parameters<typeof applyJobEvent>[0];
		const next = applyJobEvent(session, {
			event: 'job.updated',
			id: 'j',
			status: 'failed',
			progress: 0,
			phase: '',
			error: { message: WORKER }
		} as unknown as Parameters<typeof applyJobEvent>[1]);
		expect(next.phase).toBe('error');
		expect(next.error).toContain('cylinder.stl');
	});
});
