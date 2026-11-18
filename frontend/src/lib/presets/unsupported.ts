// Presets incompatibles avec l'imprimante active (Orca `show_unsupported_presets`,
// PresetComboBoxes.cpp:1429) : la liste complète d'un type (sans filtre
// d'imprimante) moins les presets compatibles déjà listés. Liste complète mise
// en cache par type (une requête par session et par type).
import { listPresets } from '$lib/api/presets';
import type { PresetSummary } from '$lib/api/types';

const cache = new Map<string, Promise<PresetSummary[]>>();

/** Presets de `all` absents de `compatible` (instanciables seulement). */
export function subtractCompatible(
	all: readonly PresetSummary[],
	compatible: readonly PresetSummary[]
): PresetSummary[] {
	const ids = new Set(compatible.map((p) => p.id));
	return all.filter((p) => !ids.has(p.id) && p.instantiation !== false);
}

export async function unsupportedPresets(
	kind: string,
	compatible: readonly PresetSummary[]
): Promise<PresetSummary[]> {
	let all = cache.get(kind);
	if (!all) {
		all = listPresets(kind).catch(() => {
			cache.delete(kind);
			return [] as PresetSummary[];
		});
		cache.set(kind, all);
	}
	return subtractCompatible(await all, compatible);
}

/** Tests : vide le cache. */
export function resetUnsupportedCache(): void {
	cache.clear();
}
