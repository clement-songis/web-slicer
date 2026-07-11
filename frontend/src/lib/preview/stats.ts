// Statistiques de prévisualisation (T070, FR-043). Assemble un modèle de vue à
// partir des stats figées du G-code (backend `GcodeStats`, endpoint `…/stats`)
// et, si disponible, du modèle de segments décodé (répartition par type de
// ligne : longueur et temps estimé). Module pur et testable ; `StatsPanel.svelte`
// se contente d'afficher.

import type { PreviewSegments } from './decode';
import { SEGMENT_ROLE_COLORS, SEGMENT_ROLE_NAMES, NO_DATA_COLOR, type Rgb } from './colorations';

/** Stats figées renvoyées par `GET /api/gcodes/{id}/stats` (backend `GcodeStats`). */
export interface GcodeStats {
	estimated_time_seconds?: number;
	estimated_time_text?: string;
	total_filament_weight_g?: number;
	total_filament_cost?: number;
	total_toolchanges?: number;
	layer_count?: number;
	filament_used_mm?: number[];
	filament_used_cm3?: number[];
	filament_used_g?: number[];
	filament_cost?: number[];
}

/** Répartition d'un type de ligne (durée + longueur + parts). */
export interface TypeStat {
	kind: number;
	name: string;
	color: Rgb;
	/** Longueur extrudée (mm). */
	length: number;
	/** Temps estimé (s) = longueur / (vitesse/60). */
	timeSeconds: number;
	/** Part du temps total (%). */
	timePercent: number;
	/** Part de la longueur totale (%). */
	lengthPercent: number;
}

/** Consommation de filament d'un extrudeur. */
export interface FilamentStat {
	extruder: number;
	lengthMm: number;
	volumeCm3: number;
	massG: number;
	cost: number;
}

/** Modèle de vue du panneau de statistiques. */
export interface PreviewStats {
	totalTimeSeconds: number;
	totalTimeText: string;
	layerCount: number;
	toolchanges: number;
	totalMassG: number;
	totalCost: number;
	types: TypeStat[];
	filaments: FilamentStat[];
}

/** Met en forme une durée (s) façon Orca : `1d 2h 3m 4s`, `2m 3s`, `0s`. */
export function formatDuration(seconds: number): string {
	const total = Math.max(0, Math.round(seconds));
	if (total === 0) return '0s';
	const d = Math.floor(total / 86400);
	const h = Math.floor((total % 86400) / 3600);
	const m = Math.floor((total % 3600) / 60);
	const s = total % 60;
	const parts: string[] = [];
	if (d) parts.push(`${d}d`);
	if (h) parts.push(`${h}h`);
	if (m) parts.push(`${m}m`);
	if (s) parts.push(`${s}s`);
	return parts.join(' ');
}

/** Longueur euclidienne du segment `i` (mm). */
function segmentLength(segments: PreviewSegments, i: number): number {
	const dx = segments.end[i * 3] - segments.start[i * 3];
	const dy = segments.end[i * 3 + 1] - segments.start[i * 3 + 1];
	const dz = segments.end[i * 3 + 2] - segments.start[i * 3 + 2];
	return Math.hypot(dx, dy, dz);
}

/** Longueur et temps estimé agrégés par type de ligne. */
export function perTypeStats(
	segments: PreviewSegments
): Map<number, { length: number; time: number }> {
	const acc = new Map<number, { length: number; time: number }>();
	for (let i = 0; i < segments.count; i++) {
		const len = segmentLength(segments, i);
		const feed = segments.feedrate[i];
		const time = feed > 0 ? len / (feed / 60) : 0;
		const kind = segments.kind[i];
		const cur = acc.get(kind) ?? { length: 0, time: 0 };
		cur.length += len;
		cur.time += time;
		acc.set(kind, cur);
	}
	return acc;
}

/**
 * Construit le modèle de vue. Le temps total et les totaux filament proviennent
 * en priorité des stats figées (moteur, FR-043) ; la répartition par type est
 * dérivée des segments décodés (si fournis).
 */
export function buildPreviewStats(stats: GcodeStats, segments?: PreviewSegments): PreviewStats {
	const perType = segments ? perTypeStats(segments) : new Map();
	const totalTypeTime = [...perType.values()].reduce((a, v) => a + v.time, 0);
	const totalTypeLength = [...perType.values()].reduce((a, v) => a + v.length, 0);

	const types: TypeStat[] = [...perType.entries()]
		.map(([kind, v]) => ({
			kind,
			name: SEGMENT_ROLE_NAMES[kind] ?? `#${kind}`,
			color: SEGMENT_ROLE_COLORS[kind] ?? NO_DATA_COLOR,
			length: v.length,
			timeSeconds: v.time,
			timePercent: totalTypeTime > 0 ? (v.time / totalTypeTime) * 100 : 0,
			lengthPercent: totalTypeLength > 0 ? (v.length / totalTypeLength) * 100 : 0
		}))
		.sort((a, b) => b.timeSeconds - a.timeSeconds);

	const filaments = buildFilaments(stats);

	const totalTimeSeconds = stats.estimated_time_seconds ?? Math.round(totalTypeTime);
	const totalTimeText = stats.estimated_time_text ?? formatDuration(totalTimeSeconds);
	const totalMassG = stats.total_filament_weight_g ?? sum(stats.filament_used_g);
	const totalCost = stats.total_filament_cost ?? sum(stats.filament_cost);

	return {
		totalTimeSeconds,
		totalTimeText,
		layerCount: stats.layer_count ?? 0,
		toolchanges: stats.total_toolchanges ?? 0,
		totalMassG,
		totalCost,
		types,
		filaments
	};
}

/** Zippe les tableaux par extrudeur en lignes de consommation. */
function buildFilaments(stats: GcodeStats): FilamentStat[] {
	const n = Math.max(
		stats.filament_used_mm?.length ?? 0,
		stats.filament_used_cm3?.length ?? 0,
		stats.filament_used_g?.length ?? 0,
		stats.filament_cost?.length ?? 0
	);
	const rows: FilamentStat[] = [];
	for (let i = 0; i < n; i++) {
		rows.push({
			extruder: i,
			lengthMm: stats.filament_used_mm?.[i] ?? 0,
			volumeCm3: stats.filament_used_cm3?.[i] ?? 0,
			massG: stats.filament_used_g?.[i] ?? 0,
			cost: stats.filament_cost?.[i] ?? 0
		});
	}
	return rows;
}

const sum = (arr?: number[]) => (arr ?? []).reduce((a, b) => a + b, 0);
