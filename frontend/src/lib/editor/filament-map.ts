// Répartition des filaments par buse (Orca FilamentMapDialog, imprimantes à
// plusieurs buses comme la H2D) : `filament_map_mode` et `filament_map`
// (1-based, une buse par filament) portés par le plateau (lot 21).

export type FilamentMapMode = 'Auto For Flush' | 'Auto For Match' | 'Manual' | 'Default';

export const FILAMENT_MAP_MODES: { value: FilamentMapMode; label: string; hint: string }[] = [
	{
		value: 'Auto For Flush',
		label: 'Automatique — moins de purge',
		hint: 'Le trancheur répartit les filaments pour limiter les changements et la purge.'
	},
	{
		value: 'Auto For Match',
		label: 'Automatique — selon les bobines',
		hint: 'Répartition selon les filaments chargés sur chaque buse (AMS).'
	},
	{ value: 'Manual', label: 'Manuelle', hint: 'Vous choisissez la buse de chaque filament.' },
	{
		value: 'Default',
		label: 'Réglage global',
		hint: 'Suit le mode du traitement / de l’imprimante.'
	}
];

/** Nombre de buses (longueur de nozzle_diameter du preset machine). */
export function extruderCount(nozzleDiameter: unknown): number {
	return Array.isArray(nozzleDiameter) ? Math.max(1, nozzleDiameter.length) : 1;
}

/** Libellé d'une buse : H2D = gauche (1) / droite (2), comme Orca. */
export function nozzleLabel(index1: number, count: number): string {
	if (count === 2) return index1 === 1 ? 'Gauche' : 'Droite';
	return `Buse ${index1}`;
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
