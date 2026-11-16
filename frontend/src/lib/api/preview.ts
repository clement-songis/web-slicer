// Appels de prévisualisation (T067/T084) : méta JSON + buffers binaires de
// couches (`WSPv`). Le chargement paresseux par fenêtre vit dans
// `lib/preview/loader.ts` ; ici, seul le transport.
import { decodePreview, type PreviewSegments } from '$lib/preview/decode';
import { decodeEngineMoves, type EngineMeta, type EngineMoves } from '$lib/preview/engine-track';
import { api, API_BASE, ApiError } from './client';
import type { PreviewMeta } from './types';

/** Méta-données de prévisualisation (couches, types, échelles). */
export const getPreviewMeta = (gcodeId: string) =>
	api.get<PreviewMeta>(`/gcodes/${gcodeId}/preview/meta`);

/**
 * Récupère et décode une plage de couches `[from, to]` (incluses) en segments.
 * Corps binaire `application/octet-stream` (hors enveloppe JSON du client).
 */
export async function fetchPreviewLayers(
	gcodeId: string,
	from: number,
	to: number
): Promise<PreviewSegments> {
	const res = await fetch(`${API_BASE}/gcodes/${gcodeId}/preview/layers?from=${from}&to=${to}`, {
		credentials: 'include'
	});
	if (!res.ok) {
		throw new ApiError(res.status, 'error', res.statusText);
	}
	return decodePreview(await res.arrayBuffer());
}

/**
 * Simulation GCodeProcessor d'Orca (moteur) : résumé (temps par mode, couche,
 * rôle, extrudeur ; volumes ; G-code personnalisés).
 */
export const getEngineMeta = (gcodeId: string) =>
	api.get<EngineMeta>(`/gcodes/${gcodeId}/preview/processed/meta`);

/** Mouvements simulés des couches [from, to] (format `WSPm`). */
export async function fetchEngineMoves(
	gcodeId: string,
	from?: number,
	to?: number
): Promise<EngineMoves> {
	const q = from === undefined ? '' : `?from=${from}&to=${to ?? from}`;
	const res = await fetch(`${API_BASE}/gcodes/${gcodeId}/preview/processed/moves${q}`, {
		credentials: 'include'
	});
	if (!res.ok) throw new ApiError(res.status, 'error', res.statusText);
	return decodeEngineMoves(await res.arrayBuffer());
}

/**
 * G-code local (menu « Open G-code… ») : aperçu sans stockage via
 * `POST /api/gcode/preview` — le fichier est renvoyé à chaque requête.
 */
async function postLocalGcode(file: Blob, query: string): Promise<Response> {
	const res = await fetch(`${API_BASE}/gcode/preview?${query}`, {
		method: 'POST',
		credentials: 'include',
		headers: { 'content-type': 'text/plain' },
		body: file
	});
	if (!res.ok) {
		let message = res.statusText;
		try {
			message = ((await res.json()) as { message?: string }).message ?? message;
		} catch {
			// corps non JSON : statut seul
		}
		throw new ApiError(res.status, 'error', message);
	}
	return res;
}

export async function localGcodeMeta(file: Blob): Promise<PreviewMeta> {
	return (await (await postLocalGcode(file, 'meta=1')).json()) as PreviewMeta;
}

export async function localGcodeLayers(
	file: Blob,
	from: number,
	to: number
): Promise<PreviewSegments> {
	const res = await postLocalGcode(file, `from=${from}&to=${to}`);
	return decodePreview(await res.arrayBuffer());
}
