// Fenêtre G-code de l'aperçu (Orca « Show G-code Window », raccourci C) :
// lignes du fichier synchronisées avec la couche courante. Pur : indexation
// des débuts de couche et calcul de la fenêtre affichée.

/** Marqueurs de changement de couche écrits par Orca (Bambu / autres). */
const LAYER_MARKERS = new Set(['; CHANGE_LAYER', ';LAYER_CHANGE']);

/** Index (0-based) de la ligne marquant le début de chaque couche. */
export function layerStartLines(lines: readonly string[]): number[] {
	const starts: number[] = [];
	for (let i = 0; i < lines.length; i++) {
		if (LAYER_MARKERS.has(lines[i].trimEnd())) starts.push(i);
	}
	return starts;
}

export interface GcodeWindow {
	/** Première ligne affichée (0-based). */
	first: number;
	/** Lignes affichées. */
	lines: string[];
	/** Ligne courante (début de la couche), 0-based dans le fichier. */
	current: number;
}

/**
 * Fenêtre de `size` lignes commençant un peu avant le début de la couche
 * `layer` (contexte), bornée au fichier. Couche hors index → bornée.
 */
export function gcodeWindowFor(
	lines: readonly string[],
	starts: readonly number[],
	layer: number,
	size = 40,
	context = 3
): GcodeWindow {
	if (lines.length === 0) return { first: 0, lines: [], current: 0 };
	const index = Math.min(Math.max(0, layer), Math.max(0, starts.length - 1));
	const current = starts.length ? starts[index] : 0;
	const first = Math.max(0, Math.min(current - context, lines.length - size));
	return { first, lines: lines.slice(first, first + size), current };
}
