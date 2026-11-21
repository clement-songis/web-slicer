// Recherche et regroupement des presets d'un sélecteur (process, filament) :
// filtre texte insensible aux accents et à la casse, compatibilité imprimante
// approchée (fabricant + buse, comme OrcaSlicer qui ne montre que les process
// compatibles avec l'imprimante active), regroupement par fabricant.
// Sans dépendance Svelte → testable isolément.

import { parseNozzle } from './catalog';
import { groupUserPresets, type UserGrouping, type UserPresetMeta } from './user-grouping';

/** Forme minimale d'un preset listé. */
export interface SearchablePreset {
	id: string;
	name: string;
	origin: string;
	vendor?: string | null;
}

/** Imprimante active (preset machine) servant au filtre de compatibilité. */
export interface ActivePrinter {
	name: string;
	vendor?: string | null;
}

/** Groupe affiché dans le menu (`<optgroup>`). */
export interface PresetGroup<T> {
	label: string;
	presets: T[];
}

export interface PresetFilterOptions {
	/** Texte saisi (plusieurs mots = tous requis). */
	query?: string;
	/** Imprimante active ; absente = pas de filtre de compatibilité. */
	printer?: ActivePrinter | null;
	/** Montrer aussi les presets d'autres fabricants / buses. */
	showAll?: boolean;
	/** Preset sélectionné : toujours conservé pour que le menu garde sa valeur. */
	selectedId?: string | null;
	/** Regroupement des presets utilisateur (préférence filament, phase 34) ;
	 *  défaut « all » : un seul groupe « Mes presets ». */
	userGrouping?: UserGrouping;
	/** Type / fabricant des presets utilisateur (regroupement par type / fabricant). */
	userMeta?: Readonly<Record<string, UserPresetMeta>>;
}

export interface PresetFilterResult<T> {
	groups: PresetGroup<T>[];
	/** Presets visibles (hors sélection forcée). */
	shown: number;
	/** Presets compatibles (avant recherche texte). */
	compatible: number;
	/** Tous les presets reçus. */
	total: number;
}

/** Fabricants dont les presets valent pour toute imprimante. */
const GENERIC_VENDORS = new Set(['', 'custom', 'generic', 'orca', 'orcafilamentlibrary']);

/** Libellé du groupe des presets sans fabricant ou génériques. */
export const GENERIC_GROUP = 'Generic||group';
/** Libellé du groupe des presets de l'utilisateur. */
export const USER_GROUP = 'My Presets';
/** Libellé du groupe gardant la sélection masquée par le filtre. */
export const SELECTED_GROUP = 'Current selection';

/** Minuscule sans accents ni espaces superflus (« Élégoo  » → « elegoo »). */
export function normalizeSearch(text: string): string {
	return text
		.normalize('NFD')
		.replace(/[\u0300-\u036f]/g, '')
		.toLowerCase()
		.replace(/\s+/g, ' ')
		.trim();
}

/** Vrai si chaque mot de `query` apparaît dans le nom ou le fabricant. */
export function matchesQuery(preset: SearchablePreset, query: string): boolean {
	const terms = normalizeSearch(query).split(' ').filter(Boolean);
	if (terms.length === 0) return true;
	const haystack = normalizeSearch(`${preset.name} ${preset.vendor ?? ''}`);
	return terms.every((t) => haystack.includes(t));
}

function isGenericVendor(vendor: string | null | undefined): boolean {
	return GENERIC_VENDORS.has(normalizeSearch(vendor ?? ''));
}

/**
 * Compatibilité approchée d'un preset avec l'imprimante active : même
 * fabricant (ou preset générique) et, si le nom indique une buse
 * (« … 0.6 nozzle »), même diamètre que l'imprimante. Les presets utilisateur
 * sont déjà filtrés côté serveur (FR-021) et restent toujours visibles.
 */
export function isLikelyCompatible(
	preset: SearchablePreset,
	printer: ActivePrinter | null | undefined
): boolean {
	if (!printer || preset.origin === 'user') return true;
	if (!isGenericVendor(preset.vendor) && !isGenericVendor(printer.vendor)) {
		if (normalizeSearch(preset.vendor ?? '') !== normalizeSearch(printer.vendor ?? ''))
			return false;
	}
	const presetNozzle = parseNozzleLoose(preset.name);
	const printerNozzle = parseNozzleLoose(printer.name);
	if (presetNozzle && printerNozzle && Number(presetNozzle) !== Number(printerNozzle)) return false;
	return true;
}

/** Diamètre « X nozzle » où qu'il soit dans le nom (fin de nom ou non). */
function parseNozzleLoose(name: string): string | null {
	return parseNozzle(name) ?? name.match(/([\d.]+)\s*nozzle/i)?.[1] ?? null;
}

/**
 * Filtre (compatibilité puis texte) et regroupe : presets utilisateur, puis
 * presets système par fabricant — celui de l'imprimante active en tête, les
 * génériques ensuite, les autres par ordre alphabétique. L'ordre d'entrée est
 * conservé dans chaque groupe.
 */
export function filterPresetList<T extends SearchablePreset>(
	presets: T[],
	options: PresetFilterOptions = {}
): PresetFilterResult<T> {
	const {
		query = '',
		printer = null,
		showAll = false,
		selectedId = null,
		userGrouping = 'all',
		userMeta = {}
	} = options;
	const compatible = showAll ? presets : presets.filter((p) => isLikelyCompatible(p, printer));
	const visible = compatible.filter((p) => matchesQuery(p, query));

	const user: T[] = [];
	const byVendor = new Map<string, T[]>();
	for (const p of visible) {
		if (p.origin === 'user') {
			user.push(p);
			continue;
		}
		const label = isGenericVendor(p.vendor) ? GENERIC_GROUP : (p.vendor as string);
		const list = byVendor.get(label) ?? [];
		list.push(p);
		byVendor.set(label, list);
	}
	const printerVendor = normalizeSearch(printer?.vendor ?? '');
	const rank = (label: string) =>
		printerVendor && normalizeSearch(label) === printerVendor ? 0 : label === GENERIC_GROUP ? 1 : 2;
	const vendorGroups = [...byVendor.entries()]
		.sort(([a], [b]) => rank(a) - rank(b) || a.localeCompare(b))
		.map(([label, list]) => ({ label, presets: list }));

	const groups: PresetGroup<T>[] = [];
	const selected = selectedId ? presets.find((p) => p.id === selectedId) : undefined;
	if (selected && !visible.includes(selected)) {
		groups.push({ label: SELECTED_GROUP, presets: [selected] });
	}
	groups.push(...groupUserPresets(user, userGrouping, userMeta));
	groups.push(...vendorGroups);
	return { groups, shown: visible.length, compatible: compatible.length, total: presets.length };
}
