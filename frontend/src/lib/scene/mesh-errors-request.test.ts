import { describe, expect, test } from 'vitest';
import { meshErrorsKey, meshErrorsRequest } from './mesh-errors';

describe('POST /projects/{id}/mesh-errors : scène courante (lot 39)', () => {
	test('ids vivants, modelId et volumes maillés seulement', () => {
		const req = meshErrorsRequest([
			{
				id: 'live-1',
				modelId: 'm1',
				volumes: [{ shape: 'box' }, { shape: 'mesh', modelId: 'm2' }, { shape: 'mesh' }]
			},
			{ id: 'live-2' }
		]);
		expect(req).toEqual({
			objects: [
				{ id: 'live-1', modelId: 'm1', volumes: [{ shape: 'mesh', modelId: 'm2' }] },
				{ id: 'live-2' }
			]
		});
	});
	test('clé stable : relancée seulement si les maillages changent', () => {
		const a = meshErrorsKey(meshErrorsRequest([{ id: 'o', modelId: 'm' }]));
		expect(meshErrorsKey(meshErrorsRequest([{ id: 'o', modelId: 'm' }]))).toBe(a);
		expect(meshErrorsKey(meshErrorsRequest([{ id: 'o', modelId: 'm-réparé' }]))).not.toBe(a);
	});
});
