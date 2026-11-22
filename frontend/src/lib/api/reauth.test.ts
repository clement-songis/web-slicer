// Phase 46 (audit 44 SESS-a) : 401 → reconnexion sur place puis requête rejouée.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import { api } from './client';
import { canReauth, finishReauth, reauthState, setReauthHost, setReauthUser } from './reauth';

function json(status: number, body: unknown) {
	return new Response(JSON.stringify(body), {
		status,
		headers: { 'content-type': 'application/json' }
	});
}

describe('canReauth', () => {
	afterEach(() => {
		setReauthHost(false);
		setReauthUser(null);
	});
	it('seulement un 401 hors /auth/, avec un compte connu et le dialogue monté', () => {
		expect(canReauth('/projects/1', 401)).toBe(false);
		setReauthHost(true);
		expect(canReauth('/projects/1', 401)).toBe(false);
		setReauthUser('a@b.c');
		expect(canReauth('/projects/1', 401)).toBe(true);
		expect(canReauth('/projects/1', 403)).toBe(false);
		expect(canReauth('/auth/me', 401)).toBe(false);
		expect(canReauth('/auth/login', 401)).toBe(false);
	});
});

describe('requête rejouée après reconnexion', () => {
	beforeEach(() => {
		setReauthHost(true);
		setReauthUser('a@b.c');
	});
	afterEach(() => {
		setReauthHost(false);
		setReauthUser(null);
		vi.unstubAllGlobals();
	});

	it('PUT 401 → dialogue ouvert → reconnecté → même PUT rejoué (corps conservé)', async () => {
		const bodies: string[] = [];
		let calls = 0;
		vi.stubGlobal(
			'fetch',
			vi.fn(async (_url: string, init: RequestInit) => {
				bodies.push(String(init.body));
				calls++;
				return calls === 1
					? json(401, { code: 'unauthorized', message: 'Session requise' })
					: json(200, { ok: true });
			})
		);
		const save = api.put<{ ok: boolean }>('/projects/1', { scene: { x: 120 } });
		await vi.waitFor(() => expect(get(reauthState).open).toBe(true));
		expect(get(reauthState).email).toBe('a@b.c');
		finishReauth(true);
		await expect(save).resolves.toEqual({ ok: true });
		expect(bodies).toEqual([bodies[0], bodies[0]]);
		expect(JSON.parse(bodies[0])).toEqual({ scene: { x: 120 } });
		expect(get(reauthState).open).toBe(false);
	});

	it('annulé → erreur 401 d’origine, sans nouvel essai', async () => {
		const fetchMock = vi.fn(async () =>
			json(401, { code: 'unauthorized', message: 'Session requise' })
		);
		vi.stubGlobal('fetch', fetchMock);
		const save = api.put('/projects/1', { a: 1 });
		await vi.waitFor(() => expect(get(reauthState).open).toBe(true));
		finishReauth(false);
		await expect(save).rejects.toMatchObject({ status: 401 });
		expect(fetchMock).toHaveBeenCalledTimes(1);
	});

	it('requêtes simultanées : un seul dialogue, toutes rejouées', async () => {
		let n = 0;
		vi.stubGlobal(
			'fetch',
			vi.fn(async () =>
				++n <= 2 ? json(401, { code: 'unauthorized', message: 'x' }) : json(200, { n })
			)
		);
		const a = api.get('/projects/1');
		const b = api.get('/projects/2');
		await vi.waitFor(() => expect(n).toBe(2));
		await new Promise((r) => setTimeout(r, 20));
		expect(get(reauthState).open).toBe(true);
		finishReauth(true);
		await expect(Promise.all([a, b])).resolves.toHaveLength(2);
	});
});
