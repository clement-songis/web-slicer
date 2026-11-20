// Appels de prévisualisation (T067/T084) : méta JSON + buffers binaires de
// couches (`WSPv`). Le chargement paresseux par fenêtre vit dans
// `lib/preview/loader.ts` ; ici, seul le transport.
import { decodePreview, type PreviewSegments } from '$lib/preview/decode';
import { decodeEngineMoves, type EngineMeta, type EngineMoves } from '$lib/preview/engine-track';
import type { EngineSeam, PlatesStatsResponse, ProcessedGcode } from '$lib/preview/engine-data';
import { api, API_BASE, ApiError } from './client';
import { apiErrorMessage } from './quota';
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

/** Coutures EMoveType::Seam du moteur (lot 31), couches moteur [from, to]. */
export async function getEngineSeams(gcodeId: string, from?: number, to?: number) {
	const q = from === undefined ? '' : `?from=${from}&to=${to ?? from}`;
	return (await api.get<{ seams: EngineSeam[] }>(`/gcodes/${gcodeId}/preview/processed/seams${q}`))
		.seams;
}

/** Tableau « Filament » et totaux de plusieurs plateaux (render_all_plates_stats). */
export const getPlatesStats = (gcodeIds: string[]) =>
	api.post<PlatesStatsResponse>('/gcodes/plates-stats', { gcode_ids: gcodeIds });

/**
 * Source des données moteur d'un aperçu : G-code tranché (`/gcodes/{id}`) ou
 * G-code externe simulé (`/gcode/opened/{token}`, lot 31). Même méta / WSPm.
 */
export interface EngineSource {
	key: string;
	meta(): Promise<EngineMeta>;
	moves(from?: number, to?: number): Promise<EngineMoves>;
	seams(): Promise<EngineSeam[]>;
	/** Palette de la vue Couleur : `palette` (tranché) / `viewer_palette` (ouvert). */
	viewer: boolean;
}

async function fetchMoves(path: string, from?: number, to?: number): Promise<EngineMoves> {
	const q = from === undefined ? '' : `?from=${from}&to=${to ?? from}`;
	const res = await fetch(`${API_BASE}${path}${q}`, { credentials: 'include' });
	if (!res.ok) throw new ApiError(res.status, 'error', res.statusText);
	return decodeEngineMoves(await res.arrayBuffer());
}

export function gcodeEngineSource(gcodeId: string): EngineSource {
	return {
		key: 'g:' + gcodeId,
		meta: () => getEngineMeta(gcodeId),
		moves: (from, to) => fetchEngineMoves(gcodeId, from, to),
		seams: () => getEngineSeams(gcodeId),
		viewer: false
	};
}

export function openedEngineSource(token: string, meta?: EngineMeta): EngineSource {
	const base = `/gcode/opened/${encodeURIComponent(token)}`;
	return {
		key: 'o:' + token,
		meta: () => (meta ? Promise.resolve(meta) : api.get<EngineMeta>(`${base}/meta`)),
		moves: (from, to) => fetchMoves(`${base}/moves`, from, to),
		seams: async () => (await api.get<{ seams: EngineSeam[] }>(`${base}/seams`)).seams,
		viewer: true
	};
}

/**
 * G-code externe simulé par le moteur (lot 31) : `POST /api/gcode/process`
 * → jeton + statistiques GCodeProcessor + pied + méta d'aperçu.
 */
export async function processLocalGcode(file: Blob): Promise<ProcessedGcode> {
	const res = await fetch(`${API_BASE}/gcode/process`, {
		method: 'POST',
		credentials: 'include',
		headers: { 'content-type': 'text/plain' },
		body: file
	});
	if (!res.ok) {
		let body: { code?: string; message?: string; details?: unknown } = {};
		try {
			body = (await res.json()) as typeof body;
		} catch {
			// corps non JSON : statut seul
		}
		throw new ApiError(
			res.status,
			body.code ?? 'error',
			apiErrorMessage(res.status, body, res.statusText),
			body.details
		);
	}
	return (await res.json()) as ProcessedGcode;
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
		let body: { code?: string; message?: string; details?: unknown } = {};
		try {
			body = (await res.json()) as typeof body;
		} catch {
			// corps non JSON (413 du proxy…) : statut seul
		}
		throw new ApiError(
			res.status,
			body.code ?? 'error',
			apiErrorMessage(res.status, body, res.statusText),
			body.details
		);
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
