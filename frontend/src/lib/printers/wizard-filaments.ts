// Assistant de premier lancement, page « Filaments » (Orca GuideFrame,
// `resources/web/guide` : après les imprimantes, l'utilisateur coche les
// filaments qu'il possède ; les `default_materials` des modèles choisis sont
// pré-cochés). Logique pure : liste des filaments compatibles des imprimantes
// choisies, pré-sélection, filtre, et ordre de préférence du filament par défaut
// d'un nouveau projet.
import type { PresetSummary } from '$lib/api/types';

/** Filament proposé par l'assistant (nom de preset, comme la section « filaments » d'AppConfig). */
export interface FilamentChoice {
	name: string;
	vendor: string;
	/** Type déduit du nom (PLA, PETG…), pour le filtre. */
	type: string;
}

/** Types de filament reconnus dans les noms de presets (ordre : du plus spécifique au plus général). */
const TYPES = [
	'PLA-CF',
	'PETG-CF',
	'PA-CF',
	'PAHT-CF',
	'PET-CF',
	'ABS-GF',
	'ASA',
	'ABS',
	'PETG',
	'PCTG',
	'PLA',
	'TPU',
	'PC',
	'PA',
	'PVA',
	'HIPS',
	'PP',
	'PE'
];

/** Type d'un filament d'après son nom (« Bambu PLA Basic @BBL X1C » → PLA). */
export function filamentType(name: string): string {
	const upper = name.toUpperCase();
	for (const t of TYPES) {
		if (new RegExp(`(^|[^A-Z])${t.replace('-', '\\-')}([^A-Z]|$)`).test(upper)) return t;
	}
	return 'Autre';
}

/** Noms de presets d'une valeur Orca (chaîne « a;b » ou tableau). */
export function presetNames(value: unknown): string[] {
	return (Array.isArray(value) ? value : [value])
		.flatMap((v) => (typeof v === 'string' ? v.split(';') : []))
		.map((v) => v.trim().replace(/^"|"$/g, ''))
		.filter(Boolean);
}

/**
 * Filaments compatibles des imprimantes choisies (union par nom), presets
 * système instanciables seulement (Orca ne liste que `instantiation: true`),
 * triés par marque puis nom.
 */
export function filamentChoices(lists: readonly (readonly PresetSummary[])[]): FilamentChoice[] {
	const byName = new Map<string, FilamentChoice>();
	for (const list of lists)
		for (const p of list) {
			if (p.origin !== 'system' || !p.instantiation || byName.has(p.name)) continue;
			byName.set(p.name, { name: p.name, vendor: vendorOf(p), type: filamentType(p.name) });
		}
	return [...byName.values()].sort(
		(a, b) => a.vendor.localeCompare(b.vendor) || a.name.localeCompare(b.name)
	);
}

function vendorOf(p: PresetSummary): string {
	const first = p.name.split(/\s+/)[0] ?? '';
	// Les presets système d'Orca commencent par la marque (« Bambu », « Generic », « Polymaker »…).
	return first || p.vendor || 'Autre';
}

/** Pré-sélection : filaments par défaut des imprimantes choisies présents dans la liste. */
export function defaultSelection(
	choices: readonly FilamentChoice[],
	defaults: readonly unknown[]
): string[] {
	const names = new Set(choices.map((c) => c.name));
	return [...new Set(defaults.flatMap(presetNames))].filter((n) => names.has(n));
}

/**
 * Ordre d'affichage : filaments pré-cochés, puis ceux de leurs marques (ex. Bambu
 * pour une Bambu Lab) et « Generic », puis le reste (plusieurs centaines).
 */
export function orderChoices(
	choices: readonly FilamentChoice[],
	pinned: readonly string[]
): FilamentChoice[] {
	const vendors = new Set(['Generic']);
	for (const c of choices) if (pinned.includes(c.name)) vendors.add(c.vendor);
	const rank = (c: FilamentChoice) => (pinned.includes(c.name) ? 0 : vendors.has(c.vendor) ? 1 : 2);
	return [...choices].sort((a, b) => rank(a) - rank(b));
}

/** Filtre de la liste (recherche, marque, type, cochés seulement). */
export function filterChoices(
	choices: readonly FilamentChoice[],
	opts: { query?: string; vendor?: string; type?: string; only?: readonly string[] | null }
): FilamentChoice[] {
	const q = (opts.query ?? '').trim().toLowerCase();
	return choices.filter(
		(c) =>
			(!q || c.name.toLowerCase().includes(q)) &&
			(!opts.vendor || c.vendor === opts.vendor) &&
			(!opts.type || c.type === opts.type) &&
			(!opts.only || opts.only.includes(c.name))
	);
}

/**
 * Ordre de préférence du filament d'un nouveau projet : le filament par défaut
 * de l'imprimante s'il fait partie des filaments choisis à l'assistant, puis les
 * autres filaments choisis, puis le défaut de l'imprimante (comportement
 * antérieur quand rien de compatible n'a été choisi).
 */
export function preferredFilaments(machineDefault: unknown, chosen: readonly string[] | null): string[] {
	const defaults = presetNames(machineDefault);
	if (!chosen?.length) return defaults;
	const first = defaults.filter((n) => chosen.includes(n));
	return [...new Set([...first, ...chosen, ...defaults])];
}
