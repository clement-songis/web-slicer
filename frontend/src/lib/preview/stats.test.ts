// Tests du modèle de vue des statistiques (T070) : formatage des durées,
// répartition par type de ligne (temps/longueur), consommation de filament,
// et priorité des totaux figés.
import { describe, expect, test } from 'vitest';
import { decodePreview } from './decode';
import { encodePreview } from './decode.test';
import { buildPreviewStats, formatDuration, perTypeStats, type GcodeStats } from './stats';
import { SEGMENT_ROLE_NAMES } from './colorations';

// Deux segments outer wall (kind 2) + un sparse infill (kind 4), longueurs et
// vitesses connues pour vérifier les temps.
const SEGMENTS = decodePreview(
	encodePreview(0, 0, [
		// longueur 10 mm à 600 mm/min → 1 s
		{
			start: [0, 0, 0],
			end: [10, 0, 0],
			feedrate: 600,
			width: 0.45,
			height: 0.2,
			kind: 2,
			extruder: 0,
			layer: 0
		},
		// longueur 10 mm à 600 mm/min → 1 s
		{
			start: [0, 0, 0],
			end: [0, 10, 0],
			feedrate: 600,
			width: 0.45,
			height: 0.2,
			kind: 2,
			extruder: 0,
			layer: 0
		},
		// longueur 20 mm à 1200 mm/min → 1 s
		{
			start: [0, 0, 0],
			end: [20, 0, 0],
			feedrate: 1200,
			width: 0.45,
			height: 0.2,
			kind: 4,
			extruder: 0,
			layer: 0
		}
	])
);

describe('formatDuration', () => {
	test('formats d/h/m/s and drops zero components', () => {
		expect(formatDuration(0)).toBe('0s');
		expect(formatDuration(3723)).toBe('1h 2m 3s');
		expect(formatDuration(90061)).toBe('1d 1h 1m 1s');
		expect(formatDuration(45)).toBe('45s');
	});
});

describe('perTypeStats', () => {
	test('aggregates length and time per line type', () => {
		const per = perTypeStats(SEGMENTS);
		// Outer wall (2) : 2 segments de 10 mm → 20 mm, 2 s.
		expect(per.get(2)!.length).toBeCloseTo(20);
		expect(per.get(2)!.time).toBeCloseTo(2);
		// Sparse infill (4) : 20 mm, 1 s.
		expect(per.get(4)!.length).toBeCloseTo(20);
		expect(per.get(4)!.time).toBeCloseTo(1);
	});
});

describe('buildPreviewStats', () => {
	const STATS: GcodeStats = {
		estimated_time_seconds: 3,
		estimated_time_text: '3s',
		total_filament_weight_g: 5.5,
		total_filament_cost: 0.25,
		total_toolchanges: 1,
		layer_count: 12,
		filament_used_mm: [1200, 40],
		filament_used_cm3: [2.9, 0.1],
		filament_used_g: [3.6, 0.12],
		filament_cost: [0.2, 0.05]
	};

	test('uses frozen totals and derives per-type rows sorted by time', () => {
		const view = buildPreviewStats(STATS, SEGMENTS);
		expect(view.totalTimeText).toBe('3s');
		expect(view.layerCount).toBe(12);
		expect(view.toolchanges).toBe(1);
		expect(view.totalMassG).toBeCloseTo(5.5);
		expect(view.totalCost).toBeCloseTo(0.25);

		// Outer wall (2 s) trié avant Sparse infill (1 s).
		expect(view.types.map((t) => t.name)).toEqual([SEGMENT_ROLE_NAMES[2], SEGMENT_ROLE_NAMES[4]]);
		// Parts de temps : 2/3 et 1/3.
		expect(view.types[0].timePercent).toBeCloseTo(66.7, 1);
		expect(view.types[1].timePercent).toBeCloseTo(33.3, 1);
	});

	test('zips filament arrays into per-extruder rows', () => {
		const view = buildPreviewStats(STATS, SEGMENTS);
		expect(view.filaments).toHaveLength(2);
		expect(view.filaments[0]).toEqual({
			extruder: 0,
			lengthMm: 1200,
			volumeCm3: 2.9,
			massG: 3.6,
			cost: 0.2
		});
	});

	test('falls back to computed totals when frozen stats are absent', () => {
		const view = buildPreviewStats({}, SEGMENTS);
		// Sans temps figé, total = somme des temps par type (2 + 1 = 3 s).
		expect(view.totalTimeSeconds).toBe(3);
		expect(view.totalTimeText).toBe('3s');
		expect(view.filaments).toHaveLength(0);
	});

	test('handles stats without any preview segments', () => {
		const view = buildPreviewStats(STATS);
		expect(view.types).toHaveLength(0);
		expect(view.totalTimeText).toBe('3s');
	});
});
