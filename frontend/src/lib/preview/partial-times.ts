// Événements du résumé (Orca GCodeViewer::render_legend, generate_partial_times,
// GCodeViewer.cpp:4194-4235). custom_gcode_times ne liste pas des instants
// mais des tronçons d'impression : chaque entrée = (type, durée du tronçon,
// temps restant au début du tronçon) ; le tronçon final est toujours marqué
// « ColorChange » par GCodeProcessor (GCodeProcessor.cpp:2635) sans
// changement de couleur réel. Orca apparie chaque entrée au premier G-code
// personnalisé de même type restant (custom_gcode_per_print_z) :
// - pause appariée → « Impression » puis « Pause » ;
// - changement de couleur apparié → « Impression » puis « Changement de couleur » ;
// - changement de couleur non apparié (tronçon final) → « Impression » seule ;
// - pause non appariée → rien.
// Pur → testable.

export interface CustomGcodeTime {
	type: string;
	/** Durée du tronçon (s). */
	time: number;
	/** Temps restant au début du tronçon (s). */
	remaining: number;
}

export interface CustomGcodeItem {
	type: string;
	extruder: number;
	color: string;
}

export type PartialKind = 'print' | 'pause' | 'color_change';

export interface PartialTime {
	kind: PartialKind;
	extruder: number;
	/** Durée du tronçon d'impression qui précède (s). */
	duration: number;
	/** Temps restant au début de ce tronçon (s). */
	remaining: number;
	/** Couleur appliquée par le changement (couleur_change seulement). */
	color?: string;
}

export function partialTimes(
	times: readonly CustomGcodeTime[],
	gcodes: readonly CustomGcodeItem[]
): PartialTime[] {
	const pending = [...gcodes];
	const items: PartialTime[] = [];
	let lastExtruder = 1;
	for (const t of times) {
		const at = pending.findIndex((g) => g.type === t.type);
		const g = at >= 0 ? pending[at] : null;
		if (t.type === 'pause_print') {
			if (!g) continue;
			pending.splice(at, 1);
			items.push({ kind: 'print', extruder: g.extruder, duration: t.time, remaining: t.remaining });
			items.push({ kind: 'pause', extruder: g.extruder, duration: t.time, remaining: t.remaining });
		} else if (t.type === 'color_change') {
			if (g) {
				pending.splice(at, 1);
				items.push({
					kind: 'print',
					extruder: g.extruder,
					duration: t.time,
					remaining: t.remaining
				});
				items.push({
					kind: 'color_change',
					extruder: g.extruder,
					duration: t.time,
					remaining: t.remaining,
					color: g.color
				});
				lastExtruder = g.extruder;
			} else
				items.push({
					kind: 'print',
					extruder: lastExtruder,
					duration: t.time,
					remaining: t.remaining
				});
		}
	}
	return items;
}

/** Vrai si les tronçons apportent une information (au moins un événement réel). */
export function hasEvents(items: readonly PartialTime[]): boolean {
	return items.some((i) => i.kind !== 'print');
}
