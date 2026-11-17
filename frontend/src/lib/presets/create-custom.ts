// Création de presets personnalisés (Orca CreateFilamentPresetDialog /
// CreatePrinterPresetDialog) : noms et valeurs, logique pure testée.

/** Types de filament proposés par Orca (CreatePresetsDialog.cpp:66). */
export const FILAMENT_TYPES = [
	'PLA',
	'rPLA',
	'PLA+',
	'PLA Tough',
	'PETG',
	'ABS',
	'ASA',
	'FLEX',
	'HIPS',
	'PA',
	'PACF',
	'NYLON',
	'PVA',
	'PVB',
	'PC',
	'PCABS',
	'PCTG',
	'PCCF',
	'PHA',
	'PP',
	'PEI',
	'PET',
	'PETGCF',
	'PTBA',
	'PTBA90A',
	'PEEK',
	'TPU93A',
	'TPU75D',
	'TPU',
	'TPU92A',
	'TPU98A',
	'Misc',
	'TPE',
	'GLAZE',
	'Nylon',
	'CPE',
	'METAL',
	'ABST',
	'Carbon Fiber',
	'SBS'
] as const;

/** Fournisseurs suggérés (extrait de la liste d'Orca ; saisie libre possible). */
export const FILAMENT_VENDORS = [
	'Generic',
	'Bambu Lab',
	'colorFabb',
	'Creality',
	'Elegoo',
	'eSUN',
	'Extrudr',
	'Fiberlogy',
	'Fillamentum',
	'FormFutura',
	'Hatchbox',
	'Inland',
	'MatterHackers',
	'Overture',
	'Polymaker',
	'Prusament',
	'Spectrum',
	'Sunlu',
	'Voron'
];

/** Orca remove_special_key : retire les caractères interdits dans un nom de preset. */
export function cleanName(s: string): string {
	return s
		.replace(/[\\/:*?"<>|@;]/g, '')
		.replace(/\s+/g, ' ')
		.trim();
}

export interface CustomFilament {
	vendor: string;
	type: string;
	serial: string;
}

/** « Fournisseur Type Série » (Orca : vendor + " " + type + " " + serial). */
export function filamentPresetName(f: CustomFilament): string {
	return cleanName(`${cleanName(f.vendor)} ${f.type} ${cleanName(f.serial)}`);
}

export function validateFilament(f: CustomFilament, existing: readonly string[]): string | null {
	if (!cleanName(f.vendor)) return 'Indiquez un fournisseur.';
	if (!f.type) return 'Choisissez un type de filament.';
	if (!cleanName(f.serial)) return 'Indiquez une série (ex. « Basic », « Matte »).';
	if (existing.includes(filamentPresetName(f))) return `« ${filamentPresetName(f)} » existe déjà.`;
	return null;
}

/** Valeurs propres du filament créé (le reste est hérité du preset de base). */
export function filamentValues(f: CustomFilament): Record<string, unknown> {
	return { filament_vendor: [cleanName(f.vendor)], filament_type: [f.type] };
}

export interface CustomPrinter {
	vendor: string;
	model: string;
	nozzle: number;
	width: number;
	depth: number;
	height: number;
	/** Origine au centre du plateau (delta / plateau rond centré). */
	centered?: boolean;
}

/** « Fournisseur Modèle 0.4 nozzle » (convention des presets machine d'Orca). */
export function printerPresetName(p: CustomPrinter): string {
	return cleanName(`${cleanName(p.vendor)} ${cleanName(p.model)} ${p.nozzle} nozzle`);
}

export function validatePrinter(p: CustomPrinter, existing: readonly string[]): string | null {
	if (!cleanName(p.vendor) || !cleanName(p.model)) return 'Indiquez le fournisseur et le modèle.';
	for (const [v, label] of [
		[p.width, 'largeur'],
		[p.depth, 'profondeur'],
		[p.height, 'hauteur']
	] as const)
		if (!(v > 0 && v <= 2000)) return `La ${label} du volume doit être entre 1 et 2000 mm.`;
	if (!(p.nozzle > 0 && p.nozzle <= 2)) return 'Diamètre de buse invalide.';
	if (existing.includes(printerPresetName(p))) return `« ${printerPresetName(p)} » existe déjà.`;
	return null;
}

/** Valeurs machine (Orca : printable_area rectangle, hauteur, buse). */
export function printerValues(p: CustomPrinter): Record<string, unknown> {
	const [x0, y0] = p.centered ? [-p.width / 2, -p.depth / 2] : [0, 0];
	const pt = (x: number, y: number) => `${+(x0 + x).toFixed(3)}x${+(y0 + y).toFixed(3)}`;
	return {
		printable_area: [pt(0, 0), pt(p.width, 0), pt(p.width, p.depth), pt(0, p.depth)],
		printable_height: p.height,
		nozzle_diameter: [p.nozzle],
		printer_model: `${cleanName(p.vendor)} ${cleanName(p.model)}`,
		printer_variant: String(p.nozzle)
	};
}
