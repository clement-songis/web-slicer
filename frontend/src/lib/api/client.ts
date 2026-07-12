// Client HTTP typé du backend. Enveloppe `fetch` : cookie de session inclus,
// corps JSON, et traduction de l'enveloppe d'erreur `{ code, message, details }`
// en `ApiError`. Aucune logique métier ici — juste le transport.
import type { ErrorBody } from './types';

/** Préfixe des routes API (le proxy Vite renvoie vers le backend en dev). */
export const API_BASE = '/api';

/** Erreur applicative portant le statut HTTP et le code stable du backend. */
export class ApiError extends Error {
	constructor(
		readonly status: number,
		readonly code: string,
		message: string,
		readonly details?: unknown
	) {
		super(message);
		this.name = 'ApiError';
	}
}

// Les DTO générés par ts-rs typent les `i64` en `bigint` (ex.
// `SliceRequest.plate_index`, `version`). `JSON.stringify` lève sur un BigInt :
// on le projette en nombre JSON, ce que le backend attend (serde i64). Les
// valeurs concernées (index de plateau, compteur de version) restent bien en
// deçà de `Number.MAX_SAFE_INTEGER`.
function bigIntReplacer(_key: string, value: unknown): unknown {
	return typeof value === 'bigint' ? Number(value) : value;
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
	const hasBody = body !== undefined;
	const res = await fetch(`${API_BASE}${path}`, {
		method,
		credentials: 'include',
		headers: hasBody ? { 'content-type': 'application/json' } : undefined,
		body: hasBody ? JSON.stringify(body, bigIntReplacer) : undefined
	});

	// 204 (logout, delete) : pas de corps.
	if (res.status === 204) return undefined as T;

	const text = await res.text();
	const data: unknown = text ? JSON.parse(text) : undefined;

	if (!res.ok) {
		const err = (data ?? {}) as Partial<ErrorBody>;
		throw new ApiError(res.status, err.code ?? 'error', err.message ?? res.statusText, err.details);
	}
	return data as T;
}

/** Verbes HTTP typés. */
export const api = {
	get: <T>(path: string) => request<T>('GET', path),
	post: <T>(path: string, body?: unknown) => request<T>('POST', path, body),
	put: <T>(path: string, body?: unknown) => request<T>('PUT', path, body),
	patch: <T>(path: string, body?: unknown) => request<T>('PATCH', path, body),
	del: <T>(path: string, body?: unknown) => request<T>('DELETE', path, body)
};
