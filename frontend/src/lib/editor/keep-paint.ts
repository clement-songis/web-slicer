// « Garder la peinture après modification du maillage » (Orca keep_painting :
// Simplifier, Réparer — ModelVolume::save/restore_painting, lot 21 :
// `POST /api/models/{id}/remap-paint`). Sans la préférence, Orca efface la
// peinture (clear_before_change_mesh).
import { api } from '$lib/api/client';
import type { RemapPaintHttpResponse } from '$lib/api/types';
import type { PaintDocument } from '$lib/scene/gizmos/painting/painting';

export const remapPaint = (modelId: string, channels: PaintDocument) =>
	api.post<RemapPaintHttpResponse>(`/models/${modelId}/remap-paint`, { channels });

/** Canaux non vides d'un document de peinture (rien à envoyer sinon). */
export function paintedChannels(doc: PaintDocument | null | undefined): PaintDocument {
	const out: PaintDocument = {};
	for (const [channel, facets] of Object.entries(doc ?? {}))
		if (facets && Object.keys(facets).length) out[channel as keyof PaintDocument] = facets;
	return out;
}
