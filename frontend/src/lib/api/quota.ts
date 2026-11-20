// Quotas et messages d'erreur explicites (lot 32) : 413 `storage_quota_exceeded`
// (details { used_bytes, quota_bytes, incoming_bytes }), 413 limite d'import,
// 429 `slice_quota_exceeded` (details { active, max, requested }) et usage du
// compte (GET /me/usage). Module pur (testé).
import type { UsageResponse } from './types';

const KB = 1024;

/** Octets lisibles : « 512 o », « 12,5 Ko », « 480 Mo », « 1,2 Go ». */
export function formatBytes(bytes: number): string {
	if (!Number.isFinite(bytes) || bytes < 0) return '—';
	const units = ['o', 'Ko', 'Mo', 'Go', 'To'];
	let v = bytes;
	let u = 0;
	while (v >= KB && u < units.length - 1) {
		v /= KB;
		u++;
	}
	const digits = u === 0 || v >= 100 ? 0 : 1;
	return `${v.toLocaleString('fr-FR', { maximumFractionDigits: digits })} ${units[u]}`;
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
		const state =
			used !== null && quota !== null
				? ` : ${formatBytes(used)} utilisés sur ${formatBytes(quota)}${incoming ? ` (fichier de ${formatBytes(incoming)})` : ''}`
				: '';
		return `Espace de stockage plein${state}. Supprimez des projets ou demandez plus d'espace à l'administrateur.`;
	}
	if (status === 413)
		return "Fichier trop volumineux pour le serveur (limite d'import fixée par l'administrateur).";
	if (status === 429 && code === 'slice_quota_exceeded') {
		const active = num(d.active);
		const max = num(d.max);
		const state =
			active !== null && max !== null
				? ` (${active} sur ${max} autorisé${max > 1 ? 's' : ''})`
				: '';
		const retry = retryAfterSuffix(d);
		return `Trop de tranchages en cours${state}. Attendez la fin d'un tranchage dans la file, puis réessayez.${retry}`;
	}
	if (status === 429) {
		const secs = retryAfterSeconds(d);
		return secs !== null
			? `Trop de requêtes : réessayez dans ${formatSeconds(secs)}.`
			: 'Trop de requêtes : réessayez dans un instant.';
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
	return `${secs} ${secs >= 2 ? 'secondes' : 'seconde'}`;
}

/** Suffixe « — réessayez dans N s » d'un message d'erreur complet. */
export function retryAfterSuffix(details: unknown): string {
	const secs = retryAfterSeconds(details);
	return secs === null ? '' : ` — réessayez dans ${formatSeconds(secs)}`;
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
				? `${formatBytes(u.storage_bytes)} sur ${formatBytes(u.storage_quota_bytes)}`
				: `${formatBytes(u.storage_bytes)} (illimité)`,
		slices:
			u.max_concurrent_slices > 0
				? `${u.active_slices} sur ${u.max_concurrent_slices} en cours`
				: `${u.active_slices} en cours (illimité)`
	};
}
