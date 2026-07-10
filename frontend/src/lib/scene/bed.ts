// Géométrie du plateau d'impression (T050), dérivée des valeurs résolues d'un
// preset machine : contour `printable_area` (points « XxY »), hauteur
// `printable_height`, fichiers modèle/texture. Pur (sans Three.js) → testable.

/** Point 2D du plateau (mm). */
export interface Point2 {
	x: number;
	y: number;
}

/** Plateau d'impression prêt à afficher. */
export interface BedShape {
	/** Contour du volume imprimable (mm), sens du preset. */
	polygon: Point2[];
	/** Largeur (X) et profondeur (Y) de la boîte englobante. */
	width: number;
	depth: number;
	/** Hauteur imprimable (Z). */
	height: number;
	/** Origine (coin min) du plateau. */
	origin: Point2;
	/** Centre de la boîte englobante (repère plateau). */
	center: Point2;
	/** Vrai si le contour est un rectangle aligné sur les axes. */
	rectangular: boolean;
	/** Chemins éventuels du modèle/texture personnalisés (aperçu). */
	customModel: string | null;
	customTexture: string | null;
}

const DEFAULT_SIZE = 256;
const DEFAULT_HEIGHT = 256;

/** Parse une liste de points Orca (`["0x0","340x0",…]`, CSV ou number[]). */
export function parseAreaPoints(value: unknown): Point2[] {
	const items = toItems(value);
	const points: Point2[] = [];
	for (const item of items) {
		const [xs, ys] = item.split('x');
		const x = Number(xs);
		const y = Number(ys);
		if (Number.isFinite(x) && Number.isFinite(y)) points.push({ x, y });
	}
	return points;
}

/** Un contour est-il un rectangle aligné (4 sommets, angles droits) ? */
export function isRectangular(polygon: Point2[]): boolean {
	if (polygon.length !== 4) return false;
	const xs = new Set(polygon.map((p) => p.x));
	const ys = new Set(polygon.map((p) => p.y));
	return xs.size === 2 && ys.size === 2;
}

/** Construit le plateau depuis les valeurs effectives d'un preset machine. */
export function bedFromValues(values: Record<string, unknown>): BedShape {
	let polygon = parseAreaPoints(values['printable_area']);
	if (polygon.length < 3) {
		polygon = [
			{ x: 0, y: 0 },
			{ x: DEFAULT_SIZE, y: 0 },
			{ x: DEFAULT_SIZE, y: DEFAULT_SIZE },
			{ x: 0, y: DEFAULT_SIZE }
		];
	}
	const xs = polygon.map((p) => p.x);
	const ys = polygon.map((p) => p.y);
	const minX = Math.min(...xs);
	const minY = Math.min(...ys);
	const maxX = Math.max(...xs);
	const maxY = Math.max(...ys);
	const height = numberOr(values['printable_height'], DEFAULT_HEIGHT);

	return {
		polygon,
		width: maxX - minX,
		depth: maxY - minY,
		height,
		origin: { x: minX, y: minY },
		center: { x: (minX + maxX) / 2, y: (minY + maxY) / 2 },
		rectangular: isRectangular(polygon),
		customModel: nonEmpty(values['bed_custom_model']),
		customTexture: nonEmpty(values['bed_custom_texture'])
	};
}

/** Nombre de divisions d'une grille de pas `step` couvrant `size` (≥ 1). */
export function gridDivisions(size: number, step = 10): number {
	return Math.max(1, Math.round(size / step));
}

// --- Interne -----------------------------------------------------------------

function toItems(value: unknown): string[] {
	if (Array.isArray(value)) return value.map((v) => String(v)).filter((s) => s.trim() !== '');
	if (typeof value === 'string') {
		return value
			.split(/[,;]/)
			.map((s) => s.trim())
			.filter((s) => s !== '');
	}
	return [];
}

function numberOr(value: unknown, fallback: number): number {
	if (typeof value === 'number' && Number.isFinite(value)) return value;
	if (typeof value === 'string') {
		const n = Number(value);
		if (Number.isFinite(n)) return n;
	}
	return fallback;
}

function nonEmpty(value: unknown): string | null {
	return typeof value === 'string' && value.trim() !== '' ? value : null;
}
