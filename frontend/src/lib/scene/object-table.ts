// Tableau des paramètres d'objets (Orca « Edit in Parameter Table »,
// GUI_ObjectTable.cpp : ObjectGridTable::init_cols) : une ligne par objet et
// par pièce / modificateur, une colonne par réglage courant (imprimable,
// plateau, nom, filament, hauteur de couche, boucles de paroi, densité de
// remplissage, supports, bordure, vitesse de paroi extérieure), chaque
// colonne de réglage avec sa remise au profil. Les colonnes « objet seul »
// (only_object d'Orca) sont vides sur les lignes de volumes. Pur.

export type TableCategory = 'all' | 'quality' | 'strength' | 'support' | 'speed';

export type TableColumnKind =
	'printable' | 'plate' | 'name' | 'extruder' | 'float' | 'int' | 'percent' | 'bool' | 'enum';

export interface TableColumn {
	key: string;
	/** En-tête (traduction française d'Orca). */
	label: string;
	kind: TableColumnKind;
	category: TableCategory;
	/** Colonne réservée aux objets (only_object d'Orca). */
	objectOnly: boolean;
	/** Colonne de réglage (surcharge + remise au profil). */
	config: boolean;
	unit?: string;
	choices?: { value: string; label: string }[];
	/** Colonne triable (en-têtes « ↑↓ » d'Orca : plateau, nom, filament). */
	sortable?: boolean;
}

/** PrintConfig.cpp `brim_type` (libellés français d'Orca). */
export const BRIM_CHOICES = [
	{ value: 'auto_brim', label: 'Auto' },
	{ value: 'brim_ears', label: 'Oreilles de souris' },
	{ value: 'painted', label: 'Peinte' },
	{ value: 'outer_only', label: 'Bordure extérieure uniquement' },
	{ value: 'inner_only', label: 'Bordure intérieure uniquement' },
	{ value: 'outer_and_inner', label: 'Bordure extérieure et intérieure' },
	{ value: 'no_brim', label: 'Pas de bordure' }
];

/** Colonnes d'ObjectGridTable::init_cols, dans l'ordre d'Orca. */
export const TABLE_COLUMNS: TableColumn[] = [
	{
		key: 'printable',
		label: 'Imprimable',
		kind: 'printable',
		category: 'all',
		objectOnly: true,
		config: false
	},
	{
		key: 'plate_index',
		label: 'Plateau',
		kind: 'plate',
		category: 'all',
		objectOnly: true,
		config: false,
		sortable: true
	},
	{
		key: 'name',
		label: 'Nom',
		kind: 'name',
		category: 'all',
		objectOnly: false,
		config: false,
		sortable: true
	},
	{
		key: 'extruder',
		label: 'Filament',
		kind: 'extruder',
		category: 'all',
		objectOnly: false,
		config: true,
		sortable: true
	},
	{
		key: 'layer_height',
		label: 'Hauteur de couche',
		kind: 'float',
		category: 'quality',
		objectOnly: true,
		config: true,
		unit: 'mm'
	},
	{
		key: 'wall_loops',
		label: 'Boucles de paroi',
		kind: 'int',
		category: 'strength',
		objectOnly: false,
		config: true
	},
	{
		key: 'sparse_infill_density',
		label: 'Densité de remplissage',
		kind: 'percent',
		category: 'strength',
		objectOnly: false,
		config: true,
		unit: '%'
	},
	{
		key: 'enable_support',
		label: 'Supports',
		kind: 'bool',
		category: 'support',
		objectOnly: true,
		config: true
	},
	{
		key: 'brim_type',
		label: 'Bordure',
		kind: 'enum',
		category: 'support',
		objectOnly: true,
		config: true,
		choices: BRIM_CHOICES
	},
	{
		key: 'outer_wall_speed',
		label: 'Vitesse de paroi extérieure',
		kind: 'float',
		category: 'speed',
		objectOnly: false,
		config: true,
		unit: 'mm/s'
	}
];

/** Filtre « catégorie » d'Orca (Tout, Qualité, Résistance, Support, Vitesse). */
export const TABLE_CATEGORIES: { value: TableCategory; label: string }[] = [
	{ value: 'all', label: 'Tout' },
	{ value: 'quality', label: 'Qualité' },
	{ value: 'strength', label: 'Résistance' },
	{ value: 'support', label: 'Support' },
	{ value: 'speed', label: 'Vitesse' }
];

/** Colonnes affichées pour une catégorie (les colonnes « Tout » restent). */
export function visibleColumns(category: TableCategory): TableColumn[] {
	return TABLE_COLUMNS.filter(
		(c) => c.category === 'all' || category === 'all' || c.category === category
	);
}

/** Ligne du tableau : objet, ou pièce / modificateur d'un objet. */
export interface TableRow {
	kind: 'object' | 'volume';
	objectId: string;
	volumeId?: string;
	name: string;
	/** Plateau (1-based) ; 0 = sans plateau. */
	plate: number;
	printable: boolean;
	/** Extrudeur propre (0 = hérité). */
	extruder: number;
	settings: Readonly<Record<string, unknown>>;
	/** Type du volume (pièce / modificateur), pour l'icône. */
	volumeType?: string;
}

export interface TableObjectSource {
	id: string;
	name: string;
	plate: number;
	printable: boolean;
	extruder: number;
	settings: Readonly<Record<string, unknown>>;
	volumes?: readonly {
		id: string;
		name: string;
		type: string;
		extruder?: number;
		settings: Readonly<Record<string, unknown>>;
	}[];
}

/**
 * Lignes : chaque objet, suivi de ses pièces et modificateurs (Orca n'affiche
 * pas les volumes négatifs ni les bloqueurs / générateurs : sans réglages).
 */
export function tableRows(objects: readonly TableObjectSource[]): TableRow[] {
	return objects.flatMap((o) => [
		{
			kind: 'object' as const,
			objectId: o.id,
			name: o.name,
			plate: o.plate,
			printable: o.printable,
			extruder: o.extruder,
			settings: o.settings
		},
		...(o.volumes ?? [])
			.filter((v) => v.type === 'part' || v.type === 'modifier')
			.map((v) => ({
				kind: 'volume' as const,
				objectId: o.id,
				volumeId: v.id,
				name: v.name,
				plate: o.plate,
				printable: o.printable,
				extruder: v.extruder ?? 0,
				settings: v.settings,
				volumeType: v.type
			}))
	]);
}

export type SortKey = 'plate_index' | 'name' | 'extruder';

/**
 * Tri des en-têtes « ↑↓ » (ObjectGridTable::sort_by_col) : par objet, les
 * volumes restant sous leur objet ; ordre stable.
 */
export function sortRows(
	rows: readonly TableRow[],
	key: SortKey | null,
	descending = false
): TableRow[] {
	if (!key) return [...rows];
	const groups: TableRow[][] = [];
	for (const row of rows) {
		if (row.kind === 'object' || !groups.length) groups.push([row]);
		else groups[groups.length - 1].push(row);
	}
	const value = (r: TableRow) =>
		key === 'plate_index'
			? r.plate
			: key === 'extruder'
				? r.extruder || 1
				: r.name.toLocaleLowerCase();
	const sorted = groups
		.map((g, i) => ({ g, i }))
		.sort((a, b) => {
			const va = value(a.g[0]);
			const vb = value(b.g[0]);
			const cmp = va < vb ? -1 : va > vb ? 1 : a.i - b.i;
			return descending && va !== vb ? -cmp : cmp;
		});
	return sorted.flatMap(({ g }) => g);
}

/** Valeur affichée d'une cellule de réglage et état de surcharge. */
export function cellValue(
	row: TableRow,
	column: TableColumn,
	inherited: Readonly<Record<string, unknown>>,
	/** Réglages de l'objet parent (une pièce hérite de son objet). */
	parent?: Readonly<Record<string, unknown>>
): { value: unknown; overridden: boolean; inherited: unknown } {
	if (column.kind === 'extruder') {
		return { value: row.extruder, overridden: row.extruder > 0, inherited: 0 };
	}
	const base = parent && column.key in parent ? parent[column.key] : inherited[column.key];
	const own = row.settings[column.key];
	const scalar = (v: unknown) => (Array.isArray(v) ? v[0] : v);
	return own === undefined
		? { value: scalar(base), overridden: false, inherited: scalar(base) }
		: { value: scalar(own), overridden: true, inherited: scalar(base) };
}

/** Texte d'une valeur (pourcentages « 15% », booléens Orca « 1 »/« 0 »). */
export function cellText(column: TableColumn, value: unknown): string {
	if (value === undefined || value === null || value === '') return '';
	if (column.kind === 'percent') {
		const n = parseFloat(String(value));
		return Number.isFinite(n) ? String(n) : String(value);
	}
	if (column.kind === 'enum')
		return column.choices?.find((c) => c.value === value)?.label ?? String(value);
	return String(value);
}

export function boolValue(value: unknown): boolean {
	return value === true || value === 1 || value === '1' || value === 'true';
}

/**
 * Saisie d'une cellule → valeur stockée (forme Orca), ou `null` si invalide.
 * Bornes de PrintConfig.cpp : hauteur de couche > 0, boucles ≥ 0 entières,
 * densité 0–100 %, vitesse ≥ 0.
 */
export function parseCell(column: TableColumn, raw: string | boolean): unknown | null {
	if (column.kind === 'bool') return typeof raw === 'boolean' ? raw : boolValue(raw);
	if (column.kind === 'enum') return column.choices?.some((c) => c.value === raw) ? raw : null;
	const text = String(raw).trim().replace(',', '.').replace(/%$/, '');
	if (!text) return null;
	const n = Number(text);
	if (!Number.isFinite(n)) return null;
	switch (column.kind) {
		case 'int':
			return Number.isInteger(n) && n >= 0 ? n : null;
		case 'percent':
			return n >= 0 && n <= 100 ? `${n}%` : null;
		case 'float':
			if (column.key === 'layer_height') return n > 0 ? n : null;
			return n >= 0 ? n : null;
		default:
			return null;
	}
}
