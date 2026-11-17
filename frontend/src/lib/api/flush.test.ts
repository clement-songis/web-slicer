import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushVolumesAvailable, resetFlushVolumesProbe } from './flush';

const respond = (status: number, body: unknown = {}) =>
	vi.fn(
		async () =>
			new Response(JSON.stringify(body), {
				status,
				headers: { 'content-type': 'application/json' }
			})
	);

afterEach(() => {
	vi.unstubAllGlobals();
	resetFlushVolumesProbe();
});

describe('détection de la route de calcul des volumes de purge', () => {
	it('404 : route absente', async () => {
		vi.stubGlobal('fetch', respond(404, { error: 'not_found', message: 'absent' }));
		expect(await flushVolumesAvailable()).toBe(false);
	});
	it('400 (corps vide refusé) : route présente', async () => {
		vi.stubGlobal('fetch', respond(400, { error: 'validation', message: 'colours vide' }));
		expect(await flushVolumesAvailable()).toBe(true);
	});
	it('sondée une seule fois', async () => {
		const f = respond(404, {});
		vi.stubGlobal('fetch', f);
		await flushVolumesAvailable();
		await flushVolumesAvailable();
		expect(f).toHaveBeenCalledTimes(1);
	});
});
