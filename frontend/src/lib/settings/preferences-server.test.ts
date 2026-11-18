import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import {
	DEFAULT_PREFERENCES,
	PREFERENCES_KEY,
	SAVE_DELAY_MS,
	appPreferences,
	flushPreferences,
	parsePreferences,
	preferencesKey,
	preferencesSettled,
	savePreferences,
	setPreferencesRemote,
	setPreferencesUser,
	type PreferencesRemote
} from './preferences';

function memoryStorage(): Storage {
	const s: Record<string, string> = {};
	Object.defineProperties(s, {
		getItem: { value: (k: string) => (k in s ? s[k] : null) },
		setItem: { value: (k: string, v: string) => (s[k] = String(v)) },
		removeItem: { value: (k: string) => delete s[k] },
		clear: { value: () => Object.keys(s).forEach((k) => delete s[k]) }
	});
	return s as unknown as Storage;
}

/** Serveur simulé : un document par compte (le compte « connecté » est session.user). */
function fakeServer() {
	const docs: Record<string, Record<string, unknown>> = {};
	const session = { user: 'alice' };
	const calls: { method: string; user: string; doc?: Record<string, unknown> }[] = [];
	const remote: PreferencesRemote = {
		async load() {
			calls.push({ method: 'GET', user: session.user });
			return structuredClone(docs[session.user] ?? {});
		},
		async save(doc) {
			calls.push({ method: 'PUT', user: session.user, doc });
			docs[session.user] = structuredClone(doc);
		}
	};
	return { docs, session, calls, remote };
}

const puts = (s: ReturnType<typeof fakeServer>) => s.calls.filter((c) => c.method === 'PUT');

describe('préférences serveur (GET/PUT /api/me/preferences)', () => {
	let server: ReturnType<typeof fakeServer>;
	beforeEach(() => {
		(globalThis as { localStorage?: unknown }).localStorage = memoryStorage();
		setPreferencesRemote(null);
		setPreferencesUser(null);
		server = fakeServer();
		setPreferencesRemote(server.remote);
	});
	afterEach(() => {
		vi.useRealTimers();
		setPreferencesRemote(null);
	});

	it('lecture au changement de session : le serveur fait foi et remplit le cache', async () => {
		server.docs.alice = { units: 'in', camera: { style: 'touchpad' }, theme: 'dark' };
		setPreferencesUser('alice');
		await preferencesSettled();
		expect(get(appPreferences).units).toBe('in');
		expect(get(appPreferences).camera.style).toBe('touchpad');
		expect(parsePreferences(localStorage.getItem(preferencesKey('alice'))).units).toBe('in');
		// Clé historique toujours écrite (test gelé).
		expect(localStorage.getItem(PREFERENCES_KEY)).toContain('"units":"in"');
	});

	it('écriture différée : une rafale = un seul PUT, clés inconnues conservées', async () => {
		vi.useFakeTimers();
		server.docs.alice = { units: 'mm', theme: 'dark' };
		setPreferencesUser('alice');
		await preferencesSettled();
		savePreferences({ ...get(appPreferences), currency: 'CHF' });
		savePreferences({ ...get(appPreferences), autoArrange: false });
		expect(localStorage.getItem(preferencesKey('alice'))).toContain('CHF');
		expect(puts(server)).toHaveLength(0);
		await vi.advanceTimersByTimeAsync(SAVE_DELAY_MS + 10);
		expect(puts(server)).toHaveLength(1);
		expect(server.docs.alice).toMatchObject({ currency: 'CHF', autoArrange: false, theme: 'dark' });
	});

	it('migration unique des préférences locales quand le serveur renvoie {}', async () => {
		localStorage.setItem(PREFERENCES_KEY, JSON.stringify({ units: 'in', currency: '$' }));
		setPreferencesUser('alice');
		await preferencesSettled();
		expect(server.docs.alice).toMatchObject({ units: 'in', currency: '$' });
		expect(puts(server)).toHaveLength(1);
		// Serveur remis à zéro : pas de seconde migration.
		delete server.docs.alice;
		setPreferencesUser(null);
		setPreferencesUser('alice');
		await preferencesSettled();
		expect(puts(server)).toHaveLength(1);
	});

	it('rien à migrer pour un nouveau compte (clé historique d’un autre compte)', async () => {
		localStorage.setItem(preferencesKey('alice'), JSON.stringify({ units: 'in' }));
		localStorage.setItem(PREFERENCES_KEY, JSON.stringify({ units: 'in' }));
		server.session.user = 'bob';
		setPreferencesUser('bob');
		await preferencesSettled();
		expect(get(appPreferences)).toEqual(DEFAULT_PREFERENCES);
		expect(puts(server)).toHaveLength(0);
	});

	it('changer de compte annule l’écriture différée du précédent', async () => {
		vi.useFakeTimers();
		setPreferencesUser('alice');
		await preferencesSettled();
		savePreferences({ ...get(appPreferences), currency: 'CHF' });
		server.session.user = 'bob';
		setPreferencesUser('bob');
		await vi.advanceTimersByTimeAsync(SAVE_DELAY_MS * 2);
		expect(server.docs.bob ?? {}).not.toHaveProperty('currency');
		expect(get(appPreferences).currency).toBe('€');
	});

	it('modification pendant la lecture : le local gagne, puis est écrit', async () => {
		server.docs.alice = { units: 'in' };
		setPreferencesUser('alice');
		savePreferences({ ...get(appPreferences), currency: 'CHF' });
		await preferencesSettled();
		expect(get(appPreferences).currency).toBe('CHF');
		await flushPreferences();
		expect(server.docs.alice).toMatchObject({ currency: 'CHF' });
	});

	it('serveur injoignable : cache local conservé, pas d’exception', async () => {
		localStorage.setItem(preferencesKey('alice'), JSON.stringify({ units: 'in' }));
		setPreferencesRemote({
			load: () => Promise.reject(new Error('offline')),
			save: () => Promise.reject(new Error('offline'))
		});
		expect(setPreferencesUser('alice').units).toBe('in');
		await preferencesSettled();
		savePreferences({ ...get(appPreferences), currency: 'CHF' });
		await flushPreferences();
		expect(localStorage.getItem(preferencesKey('alice'))).toContain('CHF');
	});

	it('préférence « répartition des filaments avant tranchage » (défaut Orca : non)', () => {
		expect(DEFAULT_PREFERENCES.filamentMapPopup).toBe(false);
		expect(parsePreferences('{"filamentMapPopup":true}').filamentMapPopup).toBe(true);
		expect(parsePreferences('{"filamentMapPopup":"yes"}').filamentMapPopup).toBe(false);
	});
});
