import { describe, expect, test } from 'vitest';
import { parsePreviewTrack } from './track';
import {
	COL,
	MOVE_FLOATS,
	applyEngine,
	decodeEngineMoves,
	EngineMoveCache,
	engineLayersFor,
	engineLayerTimes,
	summaryView,
	withEngine,
	type EngineMeta
} from './engine-track';

/** Encode un buffer WSPm (miroir de ProcessedGcode::encode_range). */
function wspm(
	moves: {
		type: number;
		line: number;
		layer: number;
		f: Partial<Record<keyof typeof COL, number>>;
	}[]
) {
	const n = moves.length;
	const bytesLen = Math.ceil((n * 4) / 4) * 4;
	const buf = new ArrayBuffer(24 + bytesLen + n * MOVE_FLOATS * 4 + n * 8);
	const v = new DataView(buf);
	'WSPm'.split('').forEach((c, i) => v.setUint8(i, c.charCodeAt(0)));
	v.setUint16(4, 1, true);
	v.setUint32(16, 0, true);
	v.setUint32(20, n, true);
	moves.forEach((m, i) => v.setUint8(24 + i * 4, m.type));
	const fo = 24 + bytesLen;
	moves.forEach((m, i) => {
		for (const [k, val] of Object.entries(m.f))
			v.setFloat32(fo + (i * MOVE_FLOATS + COL[k as keyof typeof COL]) * 4, val!, true);
	});
	const io = fo + n * MOVE_FLOATS * 4;
	moves.forEach((m, i) => {
		v.setUint32(io + i * 8, m.layer, true);
		v.setUint32(io + i * 8 + 4, m.line, true);
	});
	return buf;
}

const GCODE = [
	'G90', // 1
	';LAYER_CHANGE', // 2
	';Z:0.2', // 3
	'G1 X10 Y0 Z0.2 F600', // 4 déplacement
	'G1 X20 Y0 E1 F1200', // 5 extrusion
	';LAYER_CHANGE', // 6
	';Z:0.4', // 7
	'G1 Z0.4', // 8
	'G1 X30 Y0 E2' // 9 extrusion
].join('\n');

describe('engine-track', () => {
	test('décode WSPm et superpose par ligne G-code', () => {
		const moves = decodeEngineMoves(
			wspm([
				{ type: 8, line: 4, layer: 0, f: { timeNormal: 1 } },
				{
					type: 10,
					line: 5,
					layer: 0,
					f: { timeNormal: 2, actualFeedrate: 15, mm3PerMm: 0.1, fan: 50 }
				},
				{ type: 8, line: 8, layer: 1, f: { timeNormal: 0.5 } },
				{ type: 10, line: 9, layer: 1, f: { timeNormal: 4, actualFeedrate: 18, mm3PerMm: 0.2 } }
			])
		);
		expect(moves.count).toBe(4);
		const text = parsePreviewTrack(GCODE);
		expect(text.source).toBe('text');
		expect(Array.from(text.layers[0].lines)).toEqual([5]);
		const t = withEngine(text, moves);
		expect(t.source).toBe('engine');
		expect(t.layers[0].time).toBeCloseTo(3);
		expect(t.layers[1].time).toBeCloseTo(4.5);
		expect(t.layers[0].values.actualSpeed[0]).toBeCloseTo(15);
		expect(t.layers[1].values.actualFlow[0]).toBeCloseTo(3.6);
		expect(t.present.actualSpeed).toBe(true);
		expect(t.ranges.actualSpeed).toEqual([15, 18]);
	});

	test('corrélation insuffisante : la piste texte est conservée', () => {
		const text = parsePreviewTrack(GCODE);
		const moves = decodeEngineMoves(wspm([{ type: 10, line: 99, layer: 0, f: {} }]));
		expect(withEngine(text, moves)).toBe(text);
	});

	test('vue Résumé : temps par rôle et filament', () => {
		const meta = {
			role_times: [
				[0, 0],
				[60, 80],
				[30, 40]
			],
			travel_time: [10, 12],
			wipe_time: [0, 0],
			extruder_times: { '0': [100, 132] },
			summary: {
				modes: [
					{ time: 100, prepare_time: 5, custom_gcode_times: [] },
					{ time: 132, prepare_time: 5, custom_gcode_times: [] }
				],
				total_volumes_per_extruder: { '0': 2405.28 },
				filament_diameters: [1.75],
				filament_densities: [1.24],
				custom_gcode_per_print_z: [
					{ print_z: 2, type: 'pause_print', extruder: 1, color: '', extra: '' }
				],
				total_travel_distance: 1500,
				initial_layer_time: 12.5,
				total_filament_changes: 0
			}
		} as unknown as EngineMeta;
		const v = summaryView(meta, { 1: 'Paroi intérieure', 2: 'Paroi extérieure' });
		expect(v.hasStealth).toBe(true);
		expect(v.roles[0]).toMatchObject({ label: 'Paroi intérieure', time: 60, percent: 60 });
		expect(v.extruders[0].meters).toBeCloseTo(1.0, 2);
		expect(v.extruders[0].grams).toBeCloseTo(2.98, 2);
		expect(v.customGcodes).toHaveLength(1);
		expect(v.firstLayer[0]).toBe(12.5);
		expect(v.prepare[0]).toBe(5);
		expect(summaryView(meta, {}, 1).roles[0].time).toBe(80);
		// Compteurs distincts (Orca) : filament / extrudeur (changeur d'outils).
		expect(v.extruderChanges).toBe(0);
		meta.summary.total_extruder_changes = 104;
		expect(summaryView(meta, {}).extruderChanges).toBe(104);
	});
});

describe('fenêtrage par couches (méta v2)', () => {
	const meta = {
		layer_count: 2,
		layers: [
			{ z: 0.2, first_move: 0, end_move: 2, first_line: 4, time: [3, 6] },
			{ z: 0.4, first_move: 2, end_move: 4, first_line: 8, time: [4.5, 9] }
		],
		ranges: { actualSpeed: [15, 18], fan: [0, 0] }
	} as unknown as EngineMeta;
	const layer1 = () =>
		decodeEngineMoves(
			wspm([
				{ type: 8, line: 8, layer: 1, f: { timeNormal: 0.5 } },
				{ type: 10, line: 9, layer: 1, f: { timeNormal: 4, actualFeedrate: 18, mm3PerMm: 0.2 } }
			])
		);

	test('temps de couche depuis le méta seul, mode Normal/Silencieux', () => {
		const text = parsePreviewTrack(GCODE);
		expect(engineLayerTimes(text, meta, 0)).toEqual([3, 4.5]);
		expect(engineLayerTimes(text, meta, 1)).toEqual([6, 9]);
		const t = applyEngine(text, meta, [], 1);
		expect(t.source).toBe('engine');
		expect(t.layers.map((l) => l.time)).toEqual([6, 9]);
		expect(t.ranges.layerTime).toEqual([6, 9]);
		expect(t.ranges.actualSpeed).toEqual([15, 18]);
		expect(t.present.actualSpeed).toBe(true);
		expect(t.present.fan).toBe(false);
		expect(Number.isNaN(t.layers[0].values.actualSpeed[0])).toBe(true);
	});

	test('couches moteur de la fenêtre et valeurs des seuls blocs chargés', () => {
		const text = parsePreviewTrack(GCODE);
		expect(engineLayersFor(text, meta, 1, 1)).toEqual([1, 1]);
		expect(engineLayersFor(text, meta, 0, 1)).toEqual([0, 1]);
		const t = applyEngine(text, meta, [layer1()], 0);
		expect(t.layers[1].values.actualSpeed[0]).toBeCloseTo(18);
		expect(Number.isNaN(t.layers[0].values.actualSpeed[0])).toBe(true);
	});

	test('LRU de blocs : une requête par bloc, budget respecté', async () => {
		const calls: [number, number][] = [];
		const cache = new EngineMoveCache(
			40,
			async (from, to) => {
				calls.push([from, to]);
				return layer1();
			},
			1
		);
		await cache.ensure(0, 20);
		await cache.ensure(5, 10);
		expect(calls).toEqual([
			[0, 15],
			[16, 31],
			[0, 15]
		]);
		expect(cache.loaded().length).toBe(1);
	});
});
