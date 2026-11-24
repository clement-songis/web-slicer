// Répartition des filaments par buse (Orca FilamentMapDialog, imprimantes à
// plusieurs buses comme la H2D) : `filament_map_mode` et `filament_map`
// (1-based, une buse par filament) portés par le plateau (lot 21).

import { tr } from '#lib/i18n';

export type FilamentMapMode = 'Auto For Flush' | 'Auto For Match' | 'Manual' | 'Default';

// Libellés traduits à la lecture (accesseurs `tr`) : clés anglaises, langue courante.
function mapMode(value: FilamentMapMode, label: string, hint: string) {
	return {
		value,
		get label() {
			return tr(label);
		},
		get hint() {
			return tr(hint);
		}
	};
}

export const FILAMENT_MAP_MODES: { value: FilamentMapMode; label: string; hint: string }[] = [
	mapMode(
		'Auto For Flush',
		'Automatic — less flushing',
		'The slicer distributes filaments to limit changes and flushing.'
	),
	mapMode(
		'Auto For Match',
		'Automatic — by loaded spools',
		'Distribution by the filaments loaded on each nozzle (AMS).'
	),
	mapMode('Manual', 'Manual||map', 'You choose the nozzle of each filament.'),
	mapMode('Default', 'Global setting', 'Follows the process / printer mode.')
];

/** Nombre de buses (longueur de nozzle_diameter du preset machine). */
export function extruderCount(nozzleDiameter: unknown): number {
	return Array.isArray(nozzleDiameter) ? Math.max(1, nozzleDiameter.length) : 1;
}

/** Libellé d'une buse : H2D = gauche (1) / droite (2), comme Orca. */
export function nozzleLabel(index1: number, count: number): string {
	if (count === 2) return index1 === 1 ? tr('Left') : tr('Right');
	return tr('Nozzle {n}', { n: index1 });
}

/** Carte lue depuis les réglages du plateau, complétée à `n` filaments (défaut buse 1). */
export function readFilamentMap(raw: unknown, n: number, extruders: number): number[] {
	const list = Array.isArray(raw) ? raw : typeof raw === 'string' ? raw.split(/[ ,;]+/) : [];
	return Array.from({ length: n }, (_, i) => {
		const v = Math.round(Number(list[i]));
		return Number.isFinite(v) && v >= 1 && v <= extruders ? v : 1;
	});
}

export function readFilamentMapMode(raw: unknown): FilamentMapMode | '' {
	return FILAMENT_MAP_MODES.some((m) => m.value === raw) ? (raw as FilamentMapMode) : '';
}

/** Valeur à écrire dans le plateau (tableau de chaînes, format Orca). */
export function writeFilamentMap(map: readonly number[]): string[] {
	return map.map((v) => String(v));
}

// — Fenêtre avant tranchage (Orca try_pop_up_before_slice, FilamentMapDialog.cpp) —

/** Imprimante Bambu Lab (Orca : preset_bundle->is_bbl_vendor()). */
export function isBambuMachine(
	values: Record<string, unknown>,
	presetName?: string | null
): boolean {
	const model = String(values.printer_model ?? '');
	const name = String(presetName ?? '');
	return /^bambu lab/i.test(model) || /bambu lab|@bbl/i.test(name);
}

/**
 * Faut-il proposer la répartition avant de trancher ? Comme Orca : deux buses
 * exactement, imprimante Bambu Lab (les changeurs d'outils et les autres
 * marques sont exclus, Orca #12390) ; ouverte quand la préférence
 * pop_up_filament_map_dialog est active.
 */
export function shouldPromptFilamentMap(o: {
	enabled: boolean;
	extruders: number;
	bambu: boolean;
}): boolean {
	return o.enabled && o.extruders === 2 && o.bambu;
}

/** Mode réellement appliqué (Orca PartPlate::get_real_filament_map_mode). */
export function effectiveMapMode(plateMode: unknown, globalMode: unknown): FilamentMapMode {
	const plate = readFilamentMapMode(plateMode);
	if (plate && plate !== 'Default') return plate;
	const global = readFilamentMapMode(Array.isArray(globalMode) ? globalMode[0] : globalMode);
	return global && global !== 'Default' ? global : 'Auto For Flush';
}

/** Filaments listés (1-based) : ceux du plateau, sinon tous. */
export function listedFilaments(used: Iterable<number>, count: number): number[] {
	const out = [...new Set(used)].filter((n) => n >= 1 && n <= count).sort((a, b) => a - b);
	return out.length ? out : Array.from({ length: count }, (_, i) => i + 1);
}

/** Surcharges du plateau après validation : la carte n'est écrite qu'en manuel (Orca). */
export function filamentMapChoice(
	mode: FilamentMapMode,
	map: readonly number[]
): [key: 'filament_map_mode' | 'filament_map', value: unknown][] {
	const out: [key: 'filament_map_mode' | 'filament_map', value: unknown][] = [
		['filament_map_mode', mode]
	];
	if (mode === 'Manual') out.push(['filament_map', writeFilamentMap(map)]);
	return out;
}

/** Buse suivante (clic sur une pastille : gauche ⇄ droite). */
export function nextNozzle(current: number, extruders: number): number {
	return (current % Math.max(1, extruders)) + 1;
}
