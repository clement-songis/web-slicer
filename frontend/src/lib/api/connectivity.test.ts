import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { setLocale } from '$lib/i18n';
import { get } from 'svelte/store';
import { api, ApiError } from './client';
import { __resetConnectivity, connection, probeNow, unreachableMessage } from './connectivity';

beforeAll(() => setLocale('fr'));
afterAll(() => setLocale('en'));

describe('serveur injoignable / hors ligne', () => {
	beforeEach(() => __resetConnectivity(async () => true));
	afterEach(() => vi.unstubAllGlobals());

	it('fetch rejeté → ApiError réseau lisible, liaison perdue', async () => {
		vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new TypeError('Failed to fetch')));
		const err = (await api.get('/projects').catch((e: unknown) => e)) as ApiError;
		expect(err).toBeInstanceOf(ApiError);
		expect(err.code).toBe('network');
		expect(err.message).toMatch(/Serveur injoignable/);
		expect(get(connection).reachable).toBe(false);
	});

	it('502 du proxy sans corps JSON → « Serveur indisponible », jamais de SyntaxError', async () => {
		vi.stubGlobal(
			'fetch',
			vi.fn().mockResolvedValue(new Response('<html>Bad Gateway</html>', { status: 502 }))
		);
		const err = (await api.get('/projects').catch((e: unknown) => e)) as ApiError;
		expect(err).toBeInstanceOf(ApiError);
		expect(err.code).toBe('unavailable');
		expect(err.message).toMatch(/Serveur indisponible \(502\)/);
	});

	it('erreur applicative JSON inchangée ; réponse suivante = liaison rétablie', async () => {
		vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new TypeError('x')));
		await api.get('/a').catch(() => {});
		vi.stubGlobal(
			'fetch',
			vi
				.fn()
				.mockResolvedValue(
					new Response(JSON.stringify({ code: 'conflict', message: 'nom pris' }), { status: 409 })
				)
		);
		const err = (await api.get('/a').catch((e: unknown) => e)) as ApiError;
		expect(err.message).toBe('nom pris');
		expect(err.status).toBe(409);
		const state = get(connection);
		expect(state.reachable).toBe(true);
		expect(state.restored).toBe(1);
	});

	it('annulation propagée telle quelle', async () => {
		vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new DOMException('stop', 'AbortError')));
		const err = (await api.get('/a').catch((e: unknown) => e)) as ApiError;
		expect(err).toBeInstanceOf(DOMException);
		expect(get(connection).reachable).toBe(true);
	});

	it('sonde : retour détecté', async () => {
		__resetConnectivity(async () => false);
		expect(await probeNow()).toBe(false);
		expect(get(connection).reachable).toBe(false);
		__resetConnectivity(async () => true);
		connection.update((c) => ({ ...c, reachable: false }));
		expect(await probeNow()).toBe(true);
		expect(get(connection).reachable).toBe(true);
	});

	it('messages', () => {
		expect(unreachableMessage(503)).toMatch(/503/);
		vi.stubGlobal('navigator', { onLine: false });
		expect(unreachableMessage()).toMatch(/hors ligne/);
	});
});
