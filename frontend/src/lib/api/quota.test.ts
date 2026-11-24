import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { setLocale } from '#lib/i18n';
import {
	apiErrorMessage,
	formatBytes,
	gauge,
	quotaMessage,
	retryAfterSeconds,
	usageLines
} from './quota';

const MB = 1024 * 1024;

beforeAll(() => setLocale('fr'));
afterAll(() => setLocale('en'));

describe('quotas (lot 32)', () => {
	it('formate les octets', () => {
		expect(formatBytes(512)).toBe('512 o');
		expect(formatBytes(1536)).toBe('1,5 Ko');
		expect(formatBytes(480 * MB)).toBe('480 Mo');
		expect(formatBytes(1.25 * 1024 * MB)).toBe('1,3 Go');
		expect(formatBytes(-1)).toBe('—');
	});

	it('413 storage_quota_exceeded : état du stockage et action', () => {
		const m = quotaMessage(413, 'storage_quota_exceeded', {
			used_bytes: 480 * MB,
			quota_bytes: 500 * MB,
			incoming_bytes: 30 * MB
		});
		expect(m).toBe(
			"Espace de stockage plein : 480 Mo utilisés sur 500 Mo (fichier de 30 Mo). Supprimez des projets ou demandez plus d'espace à l'administrateur."
		);
		expect(quotaMessage(413, 'storage_quota_exceeded')).toMatch(/^Espace de stockage plein\. /);
		expect(quotaMessage(413, undefined)).toMatch(/Fichier trop volumineux/);
	});

	it('429 slice_quota_exceeded avec Retry-After (lot 35)', () => {
		expect(
			quotaMessage(429, 'slice_quota_exceeded', {
				active: 2,
				max: 2,
				requested: 1,
				retry_after: 30
			})
		).toBe(
			"Trop de tranchages en cours (2 sur 2 autorisés). Attendez la fin d'un tranchage dans la file, puis réessayez. — réessayez dans 30 secondes"
		);
		// Sans Retry-After : message du lot 32 inchangé.
		expect(quotaMessage(429, 'slice_quota_exceeded', { active: 1, max: 1 })).not.toContain(
			'— réessayez'
		);
	});

	it('429 générique : Retry-After affiché en secondes', () => {
		expect(quotaMessage(429, 'rate_limited', { retry_after: 30 })).toBe(
			'Trop de requêtes : réessayez dans 30 secondes.'
		);
		expect(quotaMessage(429, 'rate_limited', { retry_after_secs: 1 })).toBe(
			'Trop de requêtes : réessayez dans 1 seconde.'
		);
		expect(quotaMessage(429, 'rate_limited', {})).toBe(
			'Trop de requêtes : réessayez dans un instant.'
		);
		expect(retryAfterSeconds({ retry_after: 0 })).toBeNull();
		expect(retryAfterSeconds(undefined)).toBeNull();
	});

	it('429 slice_quota_exceeded (lot 32, sans Retry-After)', () => {
		expect(quotaMessage(429, 'slice_quota_exceeded', { active: 2, max: 2, requested: 1 })).toBe(
			"Trop de tranchages en cours (2 sur 2 autorisés). Attendez la fin d'un tranchage dans la file, puis réessayez."
		);
		expect(quotaMessage(429, 'slice_quota_exceeded', { active: 1, max: 1 })).toContain(
			'1 sur 1 autorisé)'
		);
		expect(quotaMessage(429, undefined)).toMatch(/Trop de requêtes/);
		expect(quotaMessage(404, 'not_found')).toBeNull();
	});

	it('message d’erreur : quotas reformulés, sinon message serveur, sinon 413/429 explicite', () => {
		expect(
			apiErrorMessage(
				413,
				{ code: 'storage_quota_exceeded', message: 'quota' },
				'Payload Too Large'
			)
		).toMatch(/^Espace de stockage plein/);
		expect(
			apiErrorMessage(413, { code: 'payload_too_large', message: 'Fichier > 500 Mo' }, 'x')
		).toBe('Fichier > 500 Mo');
		expect(apiErrorMessage(413, {}, 'Payload Too Large')).toMatch(/Fichier trop volumineux/);
		expect(apiErrorMessage(500, {}, 'Internal')).toBe('Internal');
	});

	it('jauge et lignes d’usage', () => {
		expect(gauge(10, 0)).toEqual({ used: 10, max: 0, percent: null, level: 'ok' });
		expect(gauge(50, 100).level).toBe('ok');
		expect(gauge(90, 100).level).toBe('warn');
		expect(gauge(120, 100)).toMatchObject({ percent: 100, level: 'full' });
		expect(
			usageLines({
				storage_bytes: 120 * MB,
				storage_quota_bytes: 500 * MB,
				active_slices: 1,
				max_concurrent_slices: 2
			})
		).toEqual({ storage: '120 Mo sur 500 Mo', slices: '1 sur 2 en cours' });
		expect(
			usageLines({
				storage_bytes: 2048,
				storage_quota_bytes: 0,
				active_slices: 0,
				max_concurrent_slices: 0
			})
		).toEqual({ storage: '2 Ko (illimité)', slices: '0 en cours (illimité)' });
	});
});
