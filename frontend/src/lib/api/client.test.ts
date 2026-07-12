import { afterEach, describe, expect, it } from 'vitest';
import { api, ApiError } from './client';

const realFetch = globalThis.fetch;

/** Remplace `fetch` par une réponse contrôlée, et capture l'appel. */
function stubFetch(response: Response): { calls: Array<[string, RequestInit]> } {
	const calls: Array<[string, RequestInit]> = [];
	globalThis.fetch = ((input: string, init: RequestInit) => {
		calls.push([input, init]);
		return Promise.resolve(response);
	}) as unknown as typeof fetch;
	return { calls };
}

afterEach(() => {
	globalThis.fetch = realFetch;
});

describe('client API', () => {
	it('sérialise le corps, inclut le cookie et parse la réponse', async () => {
		const { calls } = stubFetch(
			new Response(JSON.stringify({ id: '1', name: 'Benchy' }), { status: 200 })
		);
		const out = await api.post<{ id: string; name: string }>('/projects', { name: 'Benchy' });

		expect(out).toEqual({ id: '1', name: 'Benchy' });
		const [url, init] = calls[0];
		expect(url).toBe('/api/projects');
		expect(init.method).toBe('POST');
		expect(init.credentials).toBe('include');
		expect(init.body).toBe(JSON.stringify({ name: 'Benchy' }));
	});

	it('sérialise les champs BigInt (i64 ts-rs) en nombre JSON', async () => {
		const { calls } = stubFetch(new Response(JSON.stringify({ ok: true }), { status: 201 }));
		// `plate_index` est un `bigint` généré : sans coercion, JSON.stringify lèverait.
		await api.post('/projects/p1/slice', { plate_index: 2n });
		expect(calls[0][1].body).toBe('{"plate_index":2}');
	});

	it('traduit une enveloppe d’erreur en ApiError (statut + code + message)', async () => {
		stubFetch(
			new Response(JSON.stringify({ code: 'validation', message: 'nom requis' }), {
				status: 422
			})
		);
		const err = await api.post('/projects', {}).catch((e: unknown) => e);

		expect(err).toBeInstanceOf(ApiError);
		const api_err = err as ApiError;
		expect(api_err.status).toBe(422);
		expect(api_err.code).toBe('validation');
		expect(api_err.message).toBe('nom requis');
	});

	it('rend `undefined` sur 204 sans tenter de parser', async () => {
		stubFetch(new Response(null, { status: 204 }));
		const out = await api.del<void>('/auth/logout');
		expect(out).toBeUndefined();
	});

	it('n’envoie pas d’en-tête content-type sans corps', async () => {
		const { calls } = stubFetch(new Response(JSON.stringify({ status: 'ok' }), { status: 200 }));
		await api.get('/health');
		const [, init] = calls[0];
		expect(init.headers).toBeUndefined();
		expect(init.body).toBeUndefined();
	});
});
