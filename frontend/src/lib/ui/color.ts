// Contraste adaptatif sur une couleur arbitraire (pastilles filament, swatches)
// : choisit le texte noir ou blanc le plus lisible selon la luminance WCAG du
// fond, et une bordure visible quand le fond se confond avec la surface.
// Logique pure (testée) ; les composants posent le résultat en style inline.

/** Normalise `#rgb`, `#rrggbb`, `#rrggbbaa` (Orca stocke parfois l'alpha) en [r, g, b]. */
export function parseHex(color: string | null | undefined): [number, number, number] | null {
	if (!color) return null;
	const m = color.trim().match(/^#?([0-9a-f]{3}|[0-9a-f]{6}|[0-9a-f]{8})$/i);
	if (!m) return null;
	let h = m[1];
	if (h.length === 3) h = [...h].map((c) => c + c).join('');
	const n = parseInt(h.slice(0, 6), 16);
	return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

/** Luminance relative WCAG 2.x (0 = noir, 1 = blanc). */
export function relativeLuminance([r, g, b]: [number, number, number]): number {
	const f = (v: number) => {
		const s = v / 255;
		return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
	};
	return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
}

/** Ratio de contraste WCAG entre deux luminances. */
export function contrastRatio(l1: number, l2: number): number {
	return (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);
}

// Noir pur : avec le blanc, garantit ≥ 4.58:1 pour toute couleur de fond.
const DARK_TEXT = '#000000';
const LIGHT_TEXT = '#ffffff';

/**
 * Couleur de texte (sombre ou blanc) offrant le meilleur contraste sur `bg`.
 * Couleur illisible/inconnue → blanc (comportement historique).
 */
export function readableTextOn(bg: string | null | undefined): string {
	const rgb = parseHex(bg);
	if (!rgb) return LIGHT_TEXT;
	const l = relativeLuminance(rgb);
	const dark = relativeLuminance(parseHex(DARK_TEXT)!);
	return contrastRatio(l, dark) >= contrastRatio(l, 1) ? DARK_TEXT : LIGHT_TEXT;
}

/**
 * Style inline d'une pastille colorée : fond, texte lisible et bordure
 * (WCAG 1.4.11) qui reste ≥ 3:1 contre les surfaces claires comme sombres.
 */
export function swatchStyle(bg: string | null | undefined): string {
	const text = readableTextOn(bg);
	const border = text === LIGHT_TEXT ? 'rgb(0 0 0 / 0.35)' : 'rgb(0 0 0 / 0.55)';
	return `background-color: ${bg ?? 'transparent'}; color: ${text}; border: 1px solid ${border}`;
}
