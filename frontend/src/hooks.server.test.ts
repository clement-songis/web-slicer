import { describe, expect, it, vi } from 'vitest';
vi.mock('$env/dynamic/private', () => ({ env: {} }));
import { csrfForbidden } from './hooks.server';

const req = (method: string, type: string, origin?: string) =>
	new Request('http://app.local/projects/1', {
		method,
		headers: { 'content-type': type, ...(origin ? { origin } : {}) },
		body: method === 'GET' ? undefined : 'x'
	});

describe('contrôle CSRF des routes SvelteKit', () => {
	it('refuse un formulaire cross-origin', () => {
		expect(
			csrfForbidden(
				req('POST', 'multipart/form-data; boundary=x', 'https://evil'),
				'http://app.local',
				[]
			)
		).toBe(true);
		expect(
			csrfForbidden(req('POST', 'application/x-www-form-urlencoded'), 'http://app.local', [])
		).toBe(true);
	});
	it('accepte même origine, origine de confiance, JSON et GET', () => {
		expect(
			csrfForbidden(req('POST', 'multipart/form-data', 'http://app.local'), 'http://app.local', [])
		).toBe(false);
		expect(
			csrfForbidden(req('POST', 'text/plain', 'https://ok'), 'http://app.local', ['https://ok'])
		).toBe(false);
		expect(
			csrfForbidden(req('POST', 'application/json', 'https://evil'), 'http://app.local', [])
		).toBe(false);
		expect(
			csrfForbidden(req('GET', 'multipart/form-data', 'https://evil'), 'http://app.local', [])
		).toBe(false);
	});
});
