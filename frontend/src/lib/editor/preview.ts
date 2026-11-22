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
		visibleCount: parts.reduce((n, p) => n + p.visibleCount, 0),
		sizes: concatFloat(parts.map((p) => p.sizes ?? new Float32Array(p.visibleCount * 2)))
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

// --- Vignettes G-code (audit 48 DLG48-b) --------------------------------------

import { GCODE_THUMBNAILS_DEFAULT } from '../settings/preferences';

const THUMBNAIL_FORMATS = new Set(['PNG', 'JPG', 'QOI', 'BTT_TFT', 'COLPIC']);

/**
 * Format de la première entrée (« WxH/EXT » ; extension absente ou inconnue →
 * PNG), règle TabPrinter de `thumbnails_format` (Tab.cpp:4510-4533) pour les
 * firmwares qui lisent encore cette clé.
 */
export function thumbnailFirstFormat(list: string): string {
	const ext = list.split(',')[0]?.trim().split('/')[1] ?? '';
	return THUMBNAIL_FORMATS.has(ext) ? ext : 'PNG';
}

/**
 * Surcharges `printer` à joindre à la requête de tranchage quand la préférence
 * « Vignettes G-code » (audit 48 DLG48-b) est renseignée : `thumbnails` (+
 * `thumbnails_format` dérivée). `null` si la préférence est vide — les défauts
 * du moteur s'appliquent et rien n'est écrit dans la requête — ou si le
 * réglage machine est explicite (une valeur autre que le défaut du registre
 * — le doc résolu machine embarque toujours les défauts, seule une valeur
 * différente trahit un choix explicite) : il prime, parité Orca.
 */
export function gcodeThumbnailOverrides(
	preference: string,
	machineThumbnails: unknown
): Record<string, string> | null {
	if (
		machineThumbnails !== undefined &&
		machineThumbnails !== null &&
		String(machineThumbnails) !== GCODE_THUMBNAILS_DEFAULT
	)
		return null;
	const list = preference.trim();
	if (!list) return null;
	return {
		thumbnails: list,
		thumbnails_format: thumbnailFirstFormat(list)
	};
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
