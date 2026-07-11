// Appels de prévisualisation (T067/T084) : méta JSON + buffers binaires de
// couches (`WSPv`). Le chargement paresseux par fenêtre vit dans
// `lib/preview/loader.ts` ; ici, seul le transport.
import { decodePreview, type PreviewSegments } from '$lib/preview/decode';
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
