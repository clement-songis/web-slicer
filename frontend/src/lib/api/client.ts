// Client HTTP typé du backend. Enveloppe `fetch` : cookie de session inclus,
// corps JSON, et traduction de l'enveloppe d'erreur `{ code, message, details }`
// en `ApiError`. Aucune logique métier ici — juste le transport.
import type { ErrorBody } from './types';
import { reportReachable, reportUnreachable, unreachableMessage } from './connectivity';
import { apiErrorMessage } from './quota';
import { readableServerMessage } from './engine-error';
import { tr } from '$lib/i18n';

/** Préfixe des routes API (le proxy Vite renvoie vers le backend en dev). */
export const API_BASE = '/api';

/**
 * Erreur applicative portant le statut HTTP et le code stable du backend. Le
 * message affiché est rendu lisible (sortie brute du worker moteur remplacée
 * par sa cause, phase 38) ; `technical` garde le message d'origine.
 */
export class ApiError extends Error {
	readonly technical: string;
	constructor(
		readonly status: number,
		readonly code: string,
		message: string,
		readonly details?: unknown
	) {
		super(readableServerMessage(message));
		this.technical = message;
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
	const json = hasBody ? JSON.stringify(body, bigIntReplacer) : undefined;
	let res: Response;
	try {
		res = await fetchApi(path, method, hasBody, json);
	} catch (e) {
		// Annulation volontaire (navigation, AbortController) : propagée telle quelle.
		if (e instanceof DOMException && e.name === 'AbortError') throw e;
		// Phase 32 : serveur injoignable / hors ligne → message clair (au lieu de
		// « Failed to fetch » ou « Erreur inattendue ») + bandeau global.
		reportUnreachable();
		throw new ApiError(0, 'network', unreachableMessage());
	}

	// 204 (logout, delete) : pas de corps.
	if (res.status === 204) {
		reportReachable();
		return undefined as T;
	}

	const text = await res.text();
	let data: unknown;
	try {
		data = text ? JSON.parse(text) : undefined;
	} catch {
		// Corps non JSON : page d'erreur du proxy (502/503/504 quand le serveur
		// est arrêté) ou réponse tronquée. Jamais d'« Unexpected token < ».
		data = undefined;
	}

	if (!res.ok) {
		const err = (data ?? {}) as Partial<ErrorBody>;
		if (!err.message && GATEWAY_DOWN.has(res.status)) {
			reportUnreachable();
			throw new ApiError(res.status, 'unavailable', unreachableMessage(res.status));
		}
		reportReachable();
		throw new ApiError(
			res.status,
			err.code ?? 'error',
			apiErrorMessage(res.status, err, res.statusText),
			err.details
		);
	}
	reportReachable();
	if (text && data === undefined) {
		throw new ApiError(res.status, 'bad_response', tr('Unreadable server response. Try again.'));
	}
	return data as T;
}

/** Statuts renvoyés par le proxy quand le backend ne répond pas. */
const GATEWAY_DOWN = new Set([502, 503, 504]);

function fetchApi(
	path: string,
	method: string,
	hasBody: boolean,
	json: string | undefined
): Promise<Response> {
	return fetch(`${API_BASE}${path}`, {
		method,
		credentials: 'include',
		headers: hasBody ? { 'content-type': 'application/json' } : undefined,
		body: json,
		// Phase 31 : une écriture (sauvegarde du projet…) lancée juste avant un
		// rechargement / une fermeture d'onglet va à son terme au lieu d'être
		// annulée. keepalive limite les corps en vol à 64 Kio : petits corps seuls.
		keepalive: (method === 'PUT' || method === 'PATCH') && (json?.length ?? 0) < 30_000
	});
}

/** Verbes HTTP typés. */
export const api = {
	get: <T>(path: string) => request<T>('GET', path),
	post: <T>(path: string, body?: unknown) => request<T>('POST', path, body),
	put: <T>(path: string, body?: unknown) => request<T>('PUT', path, body),
	patch: <T>(path: string, body?: unknown) => request<T>('PATCH', path, body),
	del: <T>(path: string, body?: unknown) => request<T>('DELETE', path, body)
};
