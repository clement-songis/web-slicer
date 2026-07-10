// Sérialisation des valeurs des dialogs spéciaux (T045). Ces paramètres ne sont
// pas de simples scalaires : forme de plateau (liste de points « XxY »), matrice
// de volumes de purge (coFloats aplatie NxN), températures par type de plaque
// (coInts). Fonctions pures ↔ format de stockage Orca, testées isolément.

/** Un sommet du plateau (mm). */
export interface Point {
	x: number;
	y: number;
}

// --- Points : `printable_area`, `bed_exclude_area` (format Orca « XxY ») -------

/** Parse une liste de points Orca (`["0x0","340x0",…]`) — tolère CSV/number[]. */
export function parsePoints(value: unknown): Point[] {
	const items = toStringItems(value);
	const points: Point[] = [];
	for (const item of items) {
		const [xs, ys] = item.split('x');
		const x = Number(xs);
		const y = Number(ys);
		if (Number.isFinite(x) && Number.isFinite(y)) points.push({ x, y });
	}
	return points;
}

/** Sérialise des points au format Orca (`{x,y}` → `"XxY"`). */
export function serializePoints(points: Point[]): string[] {
	return points.map((p) => `${p.x}x${p.y}`);
}

/** Coins d'un plateau rectangulaire origine (0,0) → (w,d), sens antihoraire. */
export function rectangularBed(width: number, depth: number): Point[] {
	return [
		{ x: 0, y: 0 },
		{ x: width, y: 0 },
		{ x: width, y: depth },
		{ x: 0, y: depth }
	];
}

/** Largeur × profondeur d'un plateau rectangulaire (0 si non rectangulaire). */
export function bedExtents(points: Point[]): { width: number; depth: number } {
	if (points.length === 0) return { width: 0, depth: 0 };
	const xs = points.map((p) => p.x);
	const ys = points.map((p) => p.y);
	return { width: Math.max(...xs) - Math.min(...xs), depth: Math.max(...ys) - Math.min(...ys) };
}

// --- Tableaux numériques : temps plaque (coInts), volumes (coFloats) ----------

/** Parse une valeur Orca en nombres (`["35"]`, `[35]`, `"35,40"` → [35,40]). */
export function parseNumbers(value: unknown): number[] {
	return toStringItems(value)
		.map((s) => Number(s))
		.filter((n) => Number.isFinite(n));
}

/** Sérialise des nombres au format Orca (tableau de chaînes). */
export function serializeNumbers(nums: number[]): string[] {
	return nums.map((n) => String(n));
}

// --- Matrice de volumes de purge : `flush_volumes_matrix` (coFloats NxN) ------

/** Taille N d'une matrice carrée aplatie (arrondi de √longueur). */
export function matrixSize(flat: number[]): number {
	return Math.round(Math.sqrt(flat.length));
}

/** Reconstruit la matrice NxN depuis sa forme aplatie ligne par ligne. */
export function toMatrix(flat: number[], size = matrixSize(flat)): number[][] {
	const rows: number[][] = [];
	for (let r = 0; r < size; r++) rows.push(flat.slice(r * size, r * size + size));
	return rows;
}

/** Aplati une matrice NxN ligne par ligne. */
export function flattenMatrix(matrix: number[][]): number[] {
	return matrix.flat();
}

// --- Structure des tables (clés hors lignes d'option, analyse G6) -------------

/** Un type de plaque et ses deux clés de température (normale + 1re couche). */
export interface PlateType {
	label: string;
	tempKey: string;
	initialKey: string;
}

/** Types de plaque de la table des températures (`PlateTemps`). */
export const PLATE_TYPES: PlateType[] = [
	{
		label: 'Cool Plate / PLA Plate',
		tempKey: 'cool_plate_temp',
		initialKey: 'cool_plate_temp_initial_layer'
	},
	{
		label: 'Supertack Plate',
		tempKey: 'supertack_plate_temp',
		initialKey: 'supertack_plate_temp_initial_layer'
	},
	{
		label: 'Engineering Plate',
		tempKey: 'eng_plate_temp',
		initialKey: 'eng_plate_temp_initial_layer'
	},
	{
		label: 'Smooth High Temp Plate',
		tempKey: 'hot_plate_temp',
		initialKey: 'hot_plate_temp_initial_layer'
	},
	{
		label: 'Textured Cool Plate',
		tempKey: 'textured_cool_plate_temp',
		initialKey: 'textured_cool_plate_temp_initial_layer'
	},
	{
		label: 'Textured PEI Plate',
		tempKey: 'textured_plate_temp',
		initialKey: 'textured_plate_temp_initial_layer'
	}
];

/** Clés de la forme de plateau (`BedShape`). */
export const BED_SHAPE_KEYS = {
	printableArea: 'printable_area',
	excludeArea: 'bed_exclude_area',
	customModel: 'bed_custom_model',
	customTexture: 'bed_custom_texture'
} as const;

/** Clés des volumes de purge (`FlushVolumes`). */
export const FLUSH_KEYS = {
	matrix: 'flush_volumes_matrix',
	vector: 'flush_volumes_vector',
	multiplier: 'flush_multiplier'
} as const;

// --- Interne -----------------------------------------------------------------

/** Normalise une valeur Orca (string[] | number[] | CSV) en liste de chaînes. */
function toStringItems(value: unknown): string[] {
	if (Array.isArray(value)) {
		return value.map((v) => String(v)).filter((s) => s.trim() !== '');
	}
	if (typeof value === 'string') {
		return value
			.split(/[,;]/)
			.map((s) => s.trim())
			.filter((s) => s !== '');
	}
	return [];
}
