// Assemblage de l'aperçu dans le workspace (T088) : transforme la fenêtre de
// tranches du `LazyPreviewLoader` (T084) en une géométrie de lignes unique
// (T068) et dérive les bornes de coloration de la méta de prévisualisation.
// Pur → testable ; le composant se contente d'appeler ces helpers.
import type { PreviewMeta, SliceRequest } from '../api/types';
import type { PreviewSegments } from '../preview/decode';
import {
	buildPreviewGeometry,
	type GeometryOptions,
	type PreviewGeometry,
	type PreviewRanges
} from '../preview/geometry';

/** Concatène la géométrie de plusieurs tranches (fenêtre glissante du loader). */
export function buildWindowGeometry(
	window: PreviewSegments[],
	options: GeometryOptions
): PreviewGeometry {
	const parts = window.map((seg) => buildPreviewGeometry(seg, options));
	return {
		positions: concatFloat(parts.map((p) => p.positions)),
		colors: concatFloat(parts.map((p) => p.colors)),
		visibleCount: parts.reduce((n, p) => n + p.visibleCount, 0)
	};
}

/** Bornes de coloration (vitesse/largeur/hauteur) issues de la méta d'aperçu. */
export function rangesFromMeta(meta: PreviewMeta): PreviewRanges {
	return {
		feedrate: [meta.feedrate_min, meta.feedrate_max],
		width: [meta.width_min, meta.width_max],
		height: [meta.height_min, meta.height_max]
	};
}

/** Corps de `POST …/slice` : plateau ciblé (0-based) ou tous. */
export function sliceRequestFor(scope: 'plate' | 'all', plateIndex: number): SliceRequest {
	return scope === 'all' ? { all: true } : { plate_index: BigInt(Math.max(0, plateIndex)) };
}

// --- Interne -----------------------------------------------------------------

function concatFloat(arrays: Float32Array[]): Float32Array {
	const total = arrays.reduce((n, a) => n + a.length, 0);
	const out = new Float32Array(total);
	let offset = 0;
	for (const a of arrays) {
		out.set(a, offset);
		offset += a.length;
	}
	return out;
}
