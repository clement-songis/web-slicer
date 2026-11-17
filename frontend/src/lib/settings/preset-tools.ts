// Outils de gestion des presets (parité Orca Tab / DiffPresetDialog /
// UnsavedChangesDialog / EditGCodeDialog) : logique pure, testée.
import { PARAMS } from '../../generated/params';
import { UI_LAYOUT, type PresetKind } from '../../generated/ui-layout';
import {
	GCODE_PLACEHOLDER_GROUPS,
	GCODE_SPECIFIC_DEFS,
	GCODE_SPECIFIC_PLACEHOLDERS
} from '../../generated/gcode-placeholders';
import { settingChanged, type PresetValues } from './editing';

/** Page / section d'Orca où vit une clé (pour regrouper un diff). */
const LOCATION = new Map<string, { page: string; section: string; kind: PresetKind }>();
for (const page of UI_LAYOUT)
	for (const section of page.sections)
		for (const option of section.options)
			if (typeof option === 'string' && !LOCATION.has(option))
				LOCATION.set(option, { page: page.title, section: section.title, kind: page.kind });

export function locationOf(key: string) {
	return LOCATION.get(key) ?? null;
}

export interface SettingDiff {
	key: string;
	label: string;
	page: string;
	section: string;
	left: unknown;
	right: unknown;
}

/** Différences entre deux jeux de valeurs (clés affichées dans les onglets du
 *  type de preset ; ordre des onglets d'Orca). */
export function diffPresets(
	kind: PresetKind,
	left: PresetValues,
	right: PresetValues
): SettingDiff[] {
	const out: SettingDiff[] = [];
	for (const page of UI_LAYOUT) {
		if (page.kind !== kind) continue;
		for (const section of page.sections)
			for (const key of section.options) {
				if (typeof key !== 'string' || !PARAMS[key] || out.some((d) => d.key === key)) continue;
				if (left[key] === undefined && right[key] === undefined) continue;
				if (!settingChanged(key, left, right[key])) continue;
				out.push({
					key,
					label: PARAMS[key].label,
					page: page.title,
					section: section.title,
					left: left[key],
					right: right[key]
				});
			}
	}
	return out;
}

/** Valeur courte pour une cellule de diff. */
export function formatSetting(value: unknown): string {
	if (value === undefined || value === null) return '—';
	const flat = Array.isArray(value)
		? value.map((v) => (v === null ? 'nil' : String(v))).join(', ')
		: String(value);
	const lines = flat.split('\n');
	const head = lines[0].length > 60 ? lines[0].slice(0, 57) + '…' : lines[0];
	return lines.length > 1 ? `${head} (… ${lines.length} lignes)` : head;
}

/** Brouillon transféré vers un autre preset (Orca « Transfer ») : les
 *  différences du brouillon par rapport à l'enregistré, posées sur le nouveau. */
export function transferChanges(changed: PresetValues, target: PresetValues): PresetValues {
	return { ...target, ...structuredClone(changed) };
}

// — Éditeur de G-code personnalisé ------------------------------------------

export interface PlaceholderItem {
	key: string;
	label: string;
	tooltip: string;
	/** Texte inséré (Orca : `clé`, `clé[]` pour un vecteur, `clé[current_extruder]`
	 *  pour un vecteur de filament), entouré d'accolades. */
	insert: string;
}
export interface PlaceholderGroup {
	title: string;
	items: PlaceholderItem[];
}

const VECTOR = /^co(Floats|Ints|Strings|Bools|Percents|Points|FloatsOrPercents|Enums)/;
function insertFor(key: string, type: string, filament: boolean): string {
	if (!VECTOR.test(type)) return `{${key}}`;
	return filament ? `{${key}[current_extruder]}` : `{${key}[]}`;
}

/** Groupes de variables proposés pour une clé de G-code (EditGCodeDialog). */
export function placeholderGroups(gcodeKey: string): PlaceholderGroup[] {
	const groups: PlaceholderGroup[] = [];
	const specific = GCODE_SPECIFIC_PLACEHOLDERS[gcodeKey] ?? [];
	if (specific.length)
		groups.push({
			title: `Spécifique à ${PARAMS[gcodeKey]?.label ?? gcodeKey}`,
			items: specific.map((k) => {
				const d = GCODE_SPECIFIC_DEFS[k];
				return { key: k, label: d.label, tooltip: d.tooltip, insert: insertFor(k, d.type, false) };
			})
		});
	for (const g of GCODE_PLACEHOLDER_GROUPS)
		if (g.items.length)
			groups.push({
				title: g.title,
				items: g.items.map((d) => ({
					key: d.key,
					label: d.label,
					tooltip: d.tooltip,
					insert: insertFor(d.key, d.type, false)
				}))
			});
	const titles: Record<PresetKind, string> = {
		process: "Réglages d'impression",
		filament: 'Réglages du filament',
		machine: "Réglages de l'imprimante"
	};
	for (const kind of ['process', 'filament', 'machine'] as const) {
		const seen = new Set<string>();
		const items: PlaceholderItem[] = [];
		for (const page of UI_LAYOUT) {
			if (page.kind !== kind) continue;
			for (const section of page.sections)
				for (const key of section.options) {
					if (typeof key !== 'string' || seen.has(key) || !PARAMS[key] || /gcode$/.test(key))
						continue;
					seen.add(key);
					const def = PARAMS[key];
					items.push({
						key,
						label: def.label,
						tooltip: def.tooltip,
						insert: insertFor(key, def.type, kind === 'filament')
					});
				}
		}
		groups.push({ title: titles[kind], items });
	}
	return groups;
}

/** Filtre (clé, libellé, infobulle), insensible à la casse et aux accents. */
export function filterPlaceholders(
	groups: PlaceholderGroup[],
	query: string,
	label = (s: string) => s
): PlaceholderGroup[] {
	const fold = (s: string) =>
		s
			.normalize('NFD')
			.replace(/\p{Diacritic}/gu, '')
			.toLowerCase();
	const q = fold(query.trim());
	if (!q) return groups;
	return groups
		.map((g) => ({
			...g,
			items: g.items.filter((i) => fold(i.key + ' ' + label(i.label) + ' ' + i.label).includes(q))
		}))
		.filter((g) => g.items.length);
}

/** Insère `text` à la sélection ; renvoie le nouveau texte et la position du
 *  curseur (entre les crochets d'un `[]` vide, comme Orca). */
export function insertAt(
	value: string,
	start: number,
	end: number,
	text: string
): { value: string; cursor: number } {
	const next = value.slice(0, start) + text + value.slice(end);
	const inBrackets = text.endsWith('[]}') ? text.length - 2 : text.length;
	return { value: next, cursor: start + inBrackets };
}
