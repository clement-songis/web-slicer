// Couleurs de filament pour la coloration « filament » de la prévisualisation.
// Les presets filament résolus (`GET /api/presets/{id}/resolved`) exposent
// `filament_colour` (coStrings Orca) sous des formes variées : liste JSON
// `["#RRGGBB"]`, chaîne `"#RRGGBB"`, liste séparée par `;`/`,`, ou chaîne
// contenant elle-même du JSON. Ce module normalise tout cela en `Rgb[]`
// (composantes 0–1) indexé par extrudeur, prêt pour `GeometryOptions` /
// `buildLegend` (`filamentColors`).

import { getResolvedPreset } from '$lib/api/presets';
import { DEFAULT_FILAMENT_COLORS, type Rgb } from './colorations';

/** Clés lues dans les valeurs résolues, par ordre de priorité. */
export const FILAMENT_COLOUR_KEYS = ['filament_colour', 'default_filament_colour'] as const;

const HEX_RE = /^#?([0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/i;

/**
 * Convertit une couleur Orca (`#RRGGBB`, `#RGB`, `#RRGGBBAA`, `#RGBA`, avec
 * ou sans `#`) en `Rgb` normalisé. L'alpha est ignoré. `null` si invalide.
 */
export function parseOrcaColor(value: unknown): Rgb | null {
	if (typeof value !== 'string') return null;
	const m = HEX_RE.exec(value.trim());
	if (!m) return null;
	let hex = m[1];
	if (hex.length <= 4) hex = [...hex].map((c) => c + c).join('');
	const n = (i: number) => parseInt(hex.slice(i, i + 2), 16) / 255;
	return [n(0), n(2), n(4)];
}

/** Aplatit une valeur `filament_colour` en liste de jetons couleur bruts. */
function colourTokens(value: unknown, depth = 0): unknown[] {
	if (depth > 4 || value === null || value === undefined) return [];
	if (Array.isArray(value)) return value.flatMap((v) => colourTokens(v, depth + 1));
	if (typeof value !== 'string') return [value];
	const s = value.trim();
	if (!s) return [];
	// Chaîne JSON (liste ou chaîne échappée) : `'["#FF0000","#00FF00"]'`.
	if (s.startsWith('[') || s.startsWith('"')) {
		try {
			return colourTokens(JSON.parse(s), depth + 1);
		} catch {
			// pas du JSON : on retombe sur le découpage textuel
		}
	}
	// Liste sérialisée Orca (`;`) ou séparée par virgules/espaces.
	if (/[;,\s]/.test(s)) return s.split(/[;,\s]+/).filter(Boolean);
	return [s];
}

/**
 * Normalise une valeur `filament_colour` (toutes formes Orca) en `Rgb[]`.
 * Les entrées invalides prennent la couleur par défaut de leur index (pour
 * conserver l'alignement extrudeur → couleur).
 */
export function parseFilamentColours(value: unknown): Rgb[] {
	return colourTokens(value).map((tok, i) => parseOrcaColor(tok) ?? defaultFilamentColor(i));
}

/** Couleur de repli pour l'extrudeur `index` (palette par défaut cyclique). */
export function defaultFilamentColor(index: number): Rgb {
	return DEFAULT_FILAMENT_COLORS[index % DEFAULT_FILAMENT_COLORS.length];
}

/** Première couleur valide d'un objet `values` résolu (`filament_colour` puis défaut). */
export function colourFromResolvedValues(values: unknown): Rgb | null {
	if (!values || typeof values !== 'object') return null;
	const obj = values as Record<string, unknown>;
	for (const key of FILAMENT_COLOUR_KEYS) {
		for (const tok of colourTokens(obj[key])) {
			const c = parseOrcaColor(tok);
			if (c) return c;
		}
	}
	return null;
}

/**
 * Couleurs par extrudeur à partir des valeurs résolues de chaque preset filament
 * (un preset par extrudeur, dans l'ordre de `active_presets.filaments`).
 * Accepte un `ResolvedPreset` (`{ values }`) ou directement l'objet `values` ;
 * `null`/`undefined` (échec de chargement) → couleur par défaut de l'index.
 */
export function filamentColorsFromResolved(presets: readonly unknown[]): Rgb[] {
	return presets.map((p, i) => {
		const values =
			p && typeof p === 'object' && 'values' in p ? (p as { values: unknown }).values : p;
		return colourFromResolvedValues(values) ?? defaultFilamentColor(i);
	});
}

/** Récupère les valeurs résolues d'un preset (injectable pour les tests). */
export type ResolvedFetcher = (id: string) => Promise<{ values: unknown }>;

/**
 * Charge les couleurs de filament des presets actifs (`ActivePresets.filaments`)
 * via `GET /api/presets/{id}/resolved`, en parallèle. Un échec isolé ne fait
 * pas échouer l'ensemble (couleur par défaut à cet index). Renvoie `undefined`
 * si aucun filament n'est sélectionné → la géométrie garde sa palette par défaut.
 */
export async function loadFilamentColors(
	filamentIds: readonly string[],
	fetchResolved: ResolvedFetcher = getResolvedPreset
): Promise<Rgb[] | undefined> {
	if (!filamentIds.length) return undefined;
	const resolved = await Promise.all(filamentIds.map((id) => fetchResolved(id).catch(() => null)));
	return filamentColorsFromResolved(resolved);
}

/** `Rgb` (0–1) → `#rrggbb` (pastilles de légende, attributs CSS). */
export function rgbToHex([r, g, b]: Rgb): string {
	const h = (v: number) =>
		Math.round(Math.max(0, Math.min(1, v)) * 255)
			.toString(16)
			.padStart(2, '0');
	return `#${h(r)}${h(g)}${h(b)}`;
}
