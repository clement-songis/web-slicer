// Statistiques d'un G-code ouvert localement (phase 31, « Open G-code » d'Orca) :
// relecture des commentaires de fin écrits par Orca / Bambu / Prusa — mêmes clés
// que le parseur du serveur (backend/src/gcode.rs) — pour alimenter le
// « Résultat du tranchage » d'un fichier qui n'a pas été tranché ici.
import type { GcodeStats } from './stats';

/** Durée Orca (`1d 2h 3m 4s`) → secondes ; null si illisible. */
export function parseOrcaDuration(text: string): number | null {
	let total = 0;
	let found = false;
	for (const m of text.matchAll(/(\d+(?:\.\d+)?)\s*([dhms])/g)) {
		found = true;
		total += Number(m[1]) * { d: 86400, h: 3600, m: 60, s: 1 }[m[2] as 'd' | 'h' | 'm' | 's'];
	}
	return found ? total : null;
}

const list = (v: string) =>
	v
		.split(',')
		.map((x) => Number(x.trim()))
		.filter((x) => Number.isFinite(x));

/**
 * Statistiques lues dans les commentaires du G-code (null si aucune clé connue).
 * Seules les dernières 64 Kio et les premières 16 Kio sont lues (pied et en-tête).
 */
export function parseGcodeFooterStats(gcode: string): GcodeStats | null {
	const text = gcode.length > 80_000 ? gcode.slice(0, 16_384) + '\n' + gcode.slice(-65_536) : gcode;
	const stats: GcodeStats = {};
	let any = false;
	for (const raw of text.split('\n')) {
		const line = raw.trim();
		if (!line.startsWith(';')) continue;
		const c = line.replace(/^;\s*/, '');
		const val = (prefix: string) => (c.startsWith(prefix) ? c.slice(prefix.length).trim() : null);
		let v: string | null;
		if ((v = val('estimated printing time (normal mode) = ')) !== null) {
			const s = parseOrcaDuration(v);
			if (s !== null) {
				stats.estimated_time_seconds = s;
				stats.estimated_time_text = v;
				any = true;
			}
		} else if ((v = val('total filament used [g] = ')) !== null) {
			stats.total_filament_weight_g = Number(v) || 0;
			any = true;
		} else if ((v = val('total filament cost = ')) !== null) {
			stats.total_filament_cost = Number(v) || 0;
			any = true;
		} else if ((v = val('total filament change = ')) !== null) {
			stats.total_toolchanges = Number(v) || 0;
			any = true;
		} else if ((v = val('total layers count = ')) !== null) {
			stats.layer_count = Number(v) || 0;
			any = true;
		} else if ((v = val('filament used [mm] = ')) !== null) {
			stats.filament_used_mm = list(v);
			any = true;
		} else if ((v = val('filament used [cm3] = ')) !== null) {
			stats.filament_used_cm3 = list(v);
			any = true;
		} else if ((v = val('filament used [g] = ')) !== null) {
			stats.filament_used_g = list(v);
			any = true;
		} else if ((v = val('filament cost = ')) !== null) {
			stats.filament_cost = list(v);
			any = true;
		}
	}
	if (!any) return null;
	if (stats.total_filament_weight_g === undefined && stats.filament_used_g?.length)
		stats.total_filament_weight_g = stats.filament_used_g.reduce((a, b) => a + b, 0);
	if (stats.total_filament_cost === undefined && stats.filament_cost?.length)
		stats.total_filament_cost = stats.filament_cost.reduce((a, b) => a + b, 0);
	return stats;
}
