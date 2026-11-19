// « Statistiques de tous les plateaux » (phase 31, Orca
// GCodeViewer::render_all_plates_stats) : totaux des plateaux tranchés —
// temps, filament (g) et coût — à partir des stats mémorisées par plateau.
import type { GcodeStats } from './stats';

export interface PlatesTotals {
	/** Plateaux tranchés pris en compte. */
	plates: number;
	timeSeconds: number;
	grams: number;
	cost: number;
}

/** Totaux des plateaux `ids` dont le résultat porte des statistiques. */
export function platesTotals(
	ids: readonly string[],
	stats: (id: string) => unknown
): PlatesTotals | null {
	let plates = 0;
	let timeSeconds = 0;
	let grams = 0;
	let cost = 0;
	for (const id of ids) {
		const s = stats(id) as GcodeStats | null | undefined;
		if (!s || typeof s !== 'object') continue;
		plates++;
		timeSeconds += s.estimated_time_seconds ?? 0;
		grams += s.total_filament_weight_g ?? (s.filament_used_g ?? []).reduce((a, b) => a + b, 0);
		cost += s.total_filament_cost ?? (s.filament_cost ?? []).reduce((a, b) => a + b, 0);
	}
	return plates ? { plates, timeSeconds, grams, cost } : null;
}
