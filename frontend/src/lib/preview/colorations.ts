// Colorations de la prévisualisation G-code (T068, FR-041). Sept modes :
// type de ligne, vitesse, hauteur de couche, largeur de ligne, débit,
// température et numéro de filament — chacun avec légende/échelle.
//
// Palettes fidèles à OrcaSlicer (`libvgcode` : `DEFAULT_EXTRUSION_ROLES_COLORS`
// et `DEFAULT_RANGES_COLORS`) pour rester conforme à `references/orca-preview.png`.

/** Couleur RGB normalisée (composantes 0–1). */
export type Rgb = [number, number, number];

/** Les sept colorations de FR-041. */
export type Coloration =
	'type' | 'speed' | 'height' | 'width' | 'flow' | 'temperature' | 'filament';

const rgb = (r: number, g: number, b: number): Rgb => [r / 255, g / 255, b / 255];

/**
 * Couleur par rôle d'extrusion (id `SEGMENT_KINDS` du backend). Miroir de
 * `DEFAULT_EXTRUSION_ROLES_COLORS` (mappé sur nos ids stables).
 */
export const SEGMENT_ROLE_COLORS: Record<number, Rgb> = {
	0: rgb(230, 179, 179), // Undefined
	1: rgb(255, 230, 77), // Inner wall (Perimeter)
	2: rgb(255, 125, 56), // Outer wall (External perimeter)
	3: rgb(31, 31, 255), // Overhang wall
	4: rgb(176, 48, 41), // Sparse infill (Internal infill)
	5: rgb(150, 84, 204), // Internal solid infill
	6: rgb(240, 64, 64), // Top surface
	7: rgb(102, 92, 199), // Bottom surface
	8: rgb(255, 140, 105), // Ironing
	9: rgb(77, 128, 186), // Bridge
	10: rgb(77, 128, 186), // Internal Bridge
	11: rgb(255, 255, 255), // Gap infill
	12: rgb(0, 135, 110), // Skirt
	13: rgb(0, 59, 110), // Brim
	14: rgb(0, 255, 0), // Support
	15: rgb(0, 128, 0), // Support interface
	16: rgb(0, 64, 0), // Support transition
	17: rgb(179, 227, 171), // Prime tower (Wipe tower)
	18: rgb(94, 209, 148), // Custom
	19: rgb(128, 128, 128), // Multiple (Mixed)
	20: rgb(56, 72, 155) // Travel
};

/** Gris neutre pour un attribut sans donnée (ex. température non tracée). */
export const NO_DATA_COLOR: Rgb = rgb(120, 120, 120);

/** Nom lisible d'un rôle d'extrusion (légende du mode « type »). */
export const SEGMENT_ROLE_NAMES: Record<number, string> = {
	0: 'Undefined',
	1: 'Inner wall',
	2: 'Outer wall',
	3: 'Overhang wall',
	4: 'Sparse infill',
	5: 'Internal solid infill',
	6: 'Top surface',
	7: 'Bottom surface',
	8: 'Ironing',
	9: 'Bridge',
	10: 'Internal Bridge',
	11: 'Gap infill',
	12: 'Skirt',
	13: 'Brim',
	14: 'Support',
	15: 'Support interface',
	16: 'Support transition',
	17: 'Prime tower',
	18: 'Custom',
	19: 'Multiple',
	20: 'Travel'
};

/**
 * Dégradé de valeurs (bleu → rouge) miroir de `DEFAULT_RANGES_COLORS` : 11
 * arrêts interpolés linéairement pour les colorations continues.
 */
export const RANGE_PALETTE: Rgb[] = [
	rgb(11, 44, 122),
	rgb(19, 89, 133),
	rgb(28, 136, 145),
	rgb(4, 214, 15),
	rgb(170, 242, 0),
	rgb(252, 249, 3),
	rgb(245, 206, 10),
	rgb(227, 136, 32),
	rgb(209, 104, 48),
	rgb(194, 82, 60),
	rgb(148, 38, 22)
];

/** Palette de filaments par défaut (extrudeur → couleur), repli si non fournie. */
export const DEFAULT_FILAMENT_COLORS: Rgb[] = [
	rgb(0, 145, 209),
	rgb(230, 178, 24),
	rgb(213, 70, 70),
	rgb(70, 200, 120),
	rgb(160, 90, 210),
	rgb(230, 120, 40),
	rgb(90, 200, 200),
	rgb(200, 90, 160)
];

/** Échantillonne le dégradé de valeurs à `t ∈ [0,1]` (interpolation linéaire). */
export function sampleRange(t: number): Rgb {
	const clamped = Math.max(0, Math.min(1, t));
	const last = RANGE_PALETTE.length - 1;
	const scaled = clamped * last;
	const i = Math.min(last - 1, Math.floor(scaled));
	const f = scaled - i;
	const a = RANGE_PALETTE[i];
	const b = RANGE_PALETTE[i + 1];
	return [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f, a[2] + (b[2] - a[2]) * f];
}

/** Couleur d'une valeur dans `[min,max]` (échelle plate → couleur médiane). */
export function rangeColor(value: number, min: number, max: number): Rgb {
	if (!(max > min)) return sampleRange(0.5);
	return sampleRange((value - min) / (max - min));
}

/** Une entrée de légende. */
export interface LegendEntry {
	label: string;
	color: Rgb;
}

/** Légende continue (dégradé + bornes) d'un mode par valeur. */
export interface LegendScale {
	kind: 'scale';
	min: number;
	max: number;
	unit: string;
	stops: Rgb[];
}

/** Légende discrète (mode « type » / « filament »). */
export interface LegendList {
	kind: 'list';
	entries: LegendEntry[];
}

export type Legend = LegendScale | LegendList;
