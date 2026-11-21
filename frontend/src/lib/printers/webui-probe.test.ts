// Audit 40 B5 : hôte injoignable détecté par une sonde no-cors.
import { describe, expect, test } from 'vitest';
import { probeHost } from './webui';

describe('probeHost', () => {
	test('réponse (même opaque) → ok', async () => {
		const ok = (async () => new Response(null)) as unknown as typeof fetch;
		expect(await probeHost('http://h', 1000, ok)).toBe('ok');
	});
	test('rejet réseau → unreachable', async () => {
		const ko = (async () => {
			throw new TypeError('Failed to fetch');
		}) as unknown as typeof fetch;
		expect(await probeHost('http://127.0.0.1:9', 1000, ko)).toBe('unreachable');
	});
	test('délai dépassé → timeout', async () => {
		const slow = ((_u: string, init?: RequestInit) =>
			new Promise((_, rej) =>
				init?.signal?.addEventListener('abort', () => rej(new Error('abort')))
			)) as unknown as typeof fetch;
		expect(await probeHost('http://h', 20, slow)).toBe('timeout');
	});
});
