// Quotas et messages d'erreur explicites (lot 32) : 413 `storage_quota_exceeded`
// (details { used_bytes, quota_bytes, incoming_bytes }), 413 limite d'import,
// 429 `slice_quota_exceeded` (details { active, max, requested }) et usage du
// compte (GET /me/usage). Module pur (testé).
import type { UsageResponse } from './types';
import { numberLocale, tr } from '#lib/i18n';

const KB = 1024;

/** Octets lisibles : « 512 o », « 12,5 Ko », « 480 Mo », « 1,2 Go » (fr) ; « 512 B », « 12.5 KB »… (en). */
export function formatBytes(bytes: number): string {
	if (!Number.isFinite(bytes) || bytes < 0) return '—';
	const units = ['B', 'KB', 'MB', 'GB', 'TB'];
	let v = bytes;
	let u = 0;
	while (v >= KB && u < units.length - 1) {
		v /= KB;
		u++;
	}
	const digits = u === 0 || v >= 100 ? 0 : 1;
	return `${v.toLocaleString(numberLocale(), { maximumFractionDigits: digits })} ${tr(units[u])}`;
}

const num = (v: unknown): number | null => (typeof v === 'number' && Number.isFinite(v) ? v : null);

/**
 * Message affiché pour une erreur d'API ; les quotas reçoivent une phrase qui
 * dit quoi faire. `null` = garder le message d'origine.
 */
export function quotaMessage(
	status: number,
	code: string | undefined,
	details?: unknown
): string | null {
	const d = (details ?? {}) as Record<string, unknown>;
	if (status === 413 && code === 'storage_quota_exceeded') {
		const used = num(d.used_bytes);
		const quota = num(d.quota_bytes);
		const incoming = num(d.incoming_bytes);
		if (used === null || quota === null)
			return tr('Storage full. Delete projects or ask the administrator for more space.');
		const params = {
			used: formatBytes(used),
			quota: formatBytes(quota),
			incoming: formatBytes(incoming ?? 0)
		};
		return incoming
			? tr(
					'Storage full: {used} used of {quota} (file of {incoming}). Delete projects or ask the administrator for more space.',
					params
				)
			: tr(
					'Storage full: {used} used of {quota}. Delete projects or ask the administrator for more space.',
					params
				);
	}
	if (status === 413)
		return tr('File too large for the server (import limit set by the administrator).');
	if (status === 429 && code === 'slice_quota_exceeded') {
		const active = num(d.active);
		const max = num(d.max);
		const retry = retryAfterSuffix(d);
		if (active === null || max === null)
			return (
				tr(
					'Too many slices in progress. Wait for a slice in the queue to finish, then try again.'
				) + retry
			);
		const msg =
			max > 1
				? 'Too many slices in progress ({active} of {max} allowed). Wait for a slice in the queue to finish, then try again.||plural'
				: 'Too many slices in progress ({active} of {max} allowed). Wait for a slice in the queue to finish, then try again.';
		return tr(msg, { active, max }) + retry;
	}
	if (status === 429) {
		const secs = retryAfterSeconds(d);
		return secs !== null
			? tr('Too many requests: try again in {time}.', { time: formatSeconds(secs) })
			: tr('Too many requests: try again in a moment.');
	}
	return null;
}

/**
 * Secondes du `Retry-After` d'un 429 (`details.retry_after`, lot 35 : 30 s sur
 * slice_quota) ; `null` si absent ou invalide.
 */
export function retryAfterSeconds(details: unknown): number | null {
	const d = (details ?? {}) as Record<string, unknown>;
	const secs = num(d.retry_after) ?? num(d.retry_after_secs);
	return secs !== null && secs > 0 ? Math.round(secs) : null;
}

/** « N seconde(s) » sans déclinaison par appelant. */
function formatSeconds(secs: number): string {
	return tr(secs >= 2 ? '{n} seconds' : '{n} second', { n: secs });
}

/** Suffixe « — réessayez dans N s » d'un message d'erreur complet. */
export function retryAfterSuffix(details: unknown): string {
	const secs = retryAfterSeconds(details);
	return secs === null ? '' : ' ' + tr('— try again in {time}', { time: formatSeconds(secs) });
}

/**
 * Message d'une réponse d'erreur : quotas (codes dédiés) reformulés ; sinon
 * message du serveur, ou phrase 413 / 429 explicite s'il n'y en a pas (proxy).
 */
export function apiErrorMessage(
	status: number,
	body: { code?: string; message?: string; details?: unknown },
	statusText: string
): string {
	const quota = quotaMessage(status, body.code, body.details);
	if (body.code === 'storage_quota_exceeded' || body.code === 'slice_quota_exceeded') return quota!;
	return body.message || quota || statusText;
}

/** Jauge d'un quota (0 = illimité → pas de pourcentage). */
export interface Gauge {
	used: number;
	max: number;
	/** 0–100, null si illimité. */
	percent: number | null;
	level: 'ok' | 'warn' | 'full';
}

export function gauge(used: number, max: number): Gauge {
	if (!(max > 0)) return { used, max: 0, percent: null, level: 'ok' };
	const percent = Math.min(100, (100 * used) / max);
	return { used, max, percent, level: percent >= 100 ? 'full' : percent >= 85 ? 'warn' : 'ok' };
}

/** Lignes de la jauge du menu compte. */
export function usageLines(u: UsageResponse): { storage: string; slices: string } {
	return {
		storage:
			u.storage_quota_bytes > 0
				? tr('{used} of {quota}', {
						used: formatBytes(u.storage_bytes),
						quota: formatBytes(u.storage_quota_bytes)
					})
				: tr('{used} (unlimited)', { used: formatBytes(u.storage_bytes) }),
		slices:
			u.max_concurrent_slices > 0
				? tr('{active} of {max} in progress', {
						active: u.active_slices,
						max: u.max_concurrent_slices
					})
				: tr('{active} in progress (unlimited)', { active: u.active_slices })
	};
}
