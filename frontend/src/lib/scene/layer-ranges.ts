// Plages de hauteur à réglages propres (Orca `ModelObject::layer_config_ranges`,
// panneau `GUI_ObjectLayers`, menu « Height range Modifier »). Chaque plage
// [min, max) (mm, repère de l'objet posé) porte ses surcharges ; par défaut
// `layer_height` de l'objet/du process et `extruder = 0`
// (`ObjectList::get_default_layer_config`). Pur → testable.

export interface LayerRange {
	min: number;
	max: number;
	settings: Record<string, unknown>;
}

/** Hauteur de couche minimale d'un extrudeur (`get_min_layer_height`, défaut Orca). */
export const DEFAULT_MIN_LAYER_HEIGHT = 0.07;

/** Réglages par défaut d'une nouvelle plage. */
export function defaultLayerConfig(layerHeight: number): Record<string, unknown> {
	return { layer_height: layerHeight, extruder: 0 };
}

function sorted(ranges: readonly LayerRange[]): LayerRange[] {
	return [...ranges].sort((a, b) => a.min - b.min || a.max - b.max);
}

const round = (v: number) => Math.round(v * 1e4) / 1e4;

/** Première plage (« Height range Modifier » sur un objet sans plage) : [0, 2]. */
export function addRootRange(ranges: readonly LayerRange[], layerHeight: number): LayerRange[] {
	if (ranges.length > 0) return addRangeAfter(ranges, ranges.length - 1, layerHeight);
	return [{ min: 0, max: 2, settings: defaultLayerConfig(layerHeight) }];
}

/**
 * Ajoute une plage après la plage `index` (`ObjectList::add_layer_range_after_current`) :
 * après la dernière → [max, max + 2] ; trou avant la suivante → le combler ;
 * plages contiguës → scinder la suivante si elle est assez épaisse.
 */
export function addRangeAfter(
	ranges: readonly LayerRange[],
	index: number,
	layerHeight: number,
	minLayerHeight = DEFAULT_MIN_LAYER_HEIGHT
): LayerRange[] {
	const list = sorted(ranges);
	const current = list[index];
	if (!current) return list;
	const next = list[index + 1];
	const fresh = (min: number, max: number): LayerRange => ({
		min: round(min),
		max: round(max),
		settings: defaultLayerConfig(layerHeight)
	});
	if (!next) return [...list, fresh(current.max, current.max + 2)];
	if (current.max === next.min) {
		const delta = next.max - next.min;
		if (delta < 2 * minLayerHeight - 1e-9) return list;
		const middle =
			minLayerHeight > 0.5 * delta
				? next.max - minLayerHeight
				: next.min + Math.max(minLayerHeight, 0.5 * delta);
		return sorted([
			...list.filter((r) => r !== next),
			fresh(current.max, middle),
			{ ...next, min: round(middle) }
		]);
	}
	if (next.min - current.max >= minLayerHeight - 1e-9)
		return sorted([...list, fresh(current.max, next.min)]);
	return list;
}

/** Modifie les bornes d'une plage ; refusé (liste inchangée) si vide ou chevauchante. */
export function setRangeBounds(
	ranges: readonly LayerRange[],
	index: number,
	min: number,
	max: number
): LayerRange[] {
	const list = sorted(ranges);
	if (!list[index] || !(min >= 0) || !(max > min)) return list;
	const overlaps = list.some((r, i) => i !== index && min < r.max && max > r.min);
	if (overlaps) return list;
	return sorted(list.map((r, i) => (i === index ? { ...r, min: round(min), max: round(max) } : r)));
}

export function removeRange(ranges: readonly LayerRange[], index: number): LayerRange[] {
	return sorted(ranges).filter((_, i) => i !== index);
}

/** Normalise une liste lue d'un document (entrées invalides écartées). */
export function parseLayerRanges(raw: unknown): LayerRange[] {
	if (!Array.isArray(raw)) return [];
	return sorted(
		raw.flatMap((r) => {
			const o = (r ?? {}) as Partial<LayerRange>;
			const min = Number(o.min);
			const max = Number(o.max);
			if (!Number.isFinite(min) || !Number.isFinite(max) || !(max > min)) return [];
			return [
				{
					min,
					max,
					settings: o.settings && typeof o.settings === 'object' ? { ...o.settings } : {}
				}
			];
		})
	);
}
