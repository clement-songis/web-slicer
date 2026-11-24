import { beforeAll, afterAll, describe, expect, it } from 'vitest';
import { setLocale } from '#lib/i18n';
import {
	colorPrintColor,
	DUMMY_COLOR,
	engineToPreviewLayers,
	estimatedTimeAtMove,
	estimatedTimeAtTool,
	hasFilamentBreakdown,
	layerEndTime,
	layerIndexOfMove,
	layerTicks,
	statsFromProcessed,
	visibleFilamentColumns,
	withColorPrint,
	withEngineSeams,
	type EngineColorPrint,
	type FilamentTable
} from './engine-data';
import {
	COL,
	MOVE_BYTES,
	MOVE_FLOATS,
	MOVE_INTS,
	MOVE_TYPE,
	type EngineMeta,
	type EngineMoves
} from './engine-track';
import type { PreviewTrack, TrackLayer } from './track';

// Piste d'aperçu : couches de 2 segments, lignes 10/11, 20/21, 30/31.
function layer(startLine: number, lines: number[]): TrackLayer {
	const empty = new Float32Array(0);
	return {
		z: 0,
		segments: lines.length,
		values: {} as TrackLayer['values'],
		time: 0,
		travels: empty,
		wipes: empty,
		markers: {
			retract: empty,
			unretract: empty,
			seam: Float32Array.from([9, 9, 9]),
			toolchange: empty,
			colorchange: empty,
			pause: empty
		},
		lines: Uint32Array.from(lines),
		startLine
	};
}
const track: PreviewTrack = {
	layers: [layer(1, [10, 11]), layer(19, [20, 21]), layer(29, [30, 31])],
	ranges: {} as PreviewTrack['ranges'],
	present: {} as PreviewTrack['present']
};

// Mouvements moteur 0..6 : 0 = départ (noop), 1-2 couche 0, 3-4 couche 1, 5-6 couche 2.
function moves(colorIds: number[], first = 0): EngineMoves {
	const lines = [0, 10, 11, 20, 21, 30, 31];
	const count = lines.length - first;
	const bytes = new Uint8Array(count * MOVE_BYTES);
	const floats = new Float32Array(count * MOVE_FLOATS);
	const ints = new Uint32Array(count * MOVE_INTS);
	for (let k = 0; k < count; k++) {
		const g = k + first;
		bytes[k * MOVE_BYTES] = g === 0 ? MOVE_TYPE.noop : MOVE_TYPE.extrude;
		bytes[k * MOVE_BYTES + 3] = colorIds[g] ?? 0;
		floats[k * MOVE_FLOATS + COL.timeNormal] = g; // temps du mouvement g = g s
		floats[k * MOVE_FLOATS + COL.timeStealth] = 2 * g;
		ints[k * MOVE_INTS + 1] = lines[g];
	}
	return { from: 0, to: 2, firstMove: first, count, bytes, floats, ints };
}

const cp: EngineColorPrint = {
	palette: ['#FF0000', '#00FF00'],
	viewer_palette: ['#0000FF', '#00FF00'],
	option_layers: [2],
	extruders: [
		{
			extruder: 0,
			items: [
				{ color_id: 0, layer: 0, time: [0, 0] },
				{ color_id: 1, layer: 1, time: [3, 6] }
			]
		}
	],
	markers: [
		{ type: 'color_change', layer: 1, move: 3, extruder: 0, x: 1, y: 2, z: 3, time: [3, 6] },
		{ type: 'pause_print', layer: 2, move: 5, extruder: 0, x: 4, y: 5, z: 6, time: [10, 20] }
	],
	custom_gcodes: []
};
const meta = {
	layers: [
		{ z: 0.2, first_move: 0, end_move: 3, first_line: 0, time: [3, 6], elapsed: [0, 0] },
		{ z: 0.4, first_move: 3, end_move: 5, first_line: 19, time: [7, 14], elapsed: [3, 6] },
		{ z: 0.6, first_move: 5, end_move: 7, first_line: 29, time: [11, 22], elapsed: [10, 20] }
	],
	color_print: cp
} as unknown as EngineMeta;

describe('engine-data (lot 31)', () => {
	it('rapproche les couches moteur et aperçu par first_line, pas par index', () => {
		expect([...engineToPreviewLayers(track, meta)]).toEqual([0, 1, 2]);
		// Couche moteur supplémentaire (pause : ré-émission à l'id précédent).
		const shifted = {
			...meta,
			layers: [meta.layers[0], { ...meta.layers[1], first_line: 15 }, ...meta.layers.slice(1)]
		} as EngineMeta;
		expect([...engineToPreviewLayers(track, shifted)]).toEqual([0, 0, 1, 2]);
		expect(layerIndexOfMove(meta, 0)).toBe(0);
		expect(layerIndexOfMove(meta, 4)).toBe(1);
		expect(layerIndexOfMove(meta, 6)).toBe(2);
	});

	it('vue Couleur : cp_color_id par segment, repli par extrudeur, couche à pause grisée', () => {
		const t = withColorPrint(track, meta, [moves([0, 0, 1, 1, 1, 1, 1])]);
		expect(t.colorPrint?.palette[0]).toEqual([1, 0, 0]);
		// Segment 1 de la couche 0 : octet 3 = 1 → vert (changement en cours de couche).
		expect(colorPrintColor(t, 0, 0, 0)).toEqual([1, 0, 0]);
		expect(colorPrintColor(t, 0, 1, 0)).toEqual([0, 1, 0]);
		expect(colorPrintColor(t, 2, 0, 0)).toEqual(DUMMY_COLOR);
		// Sans mouvements chargés : couleur de l'extrudeur au début de la couche.
		const lazy = withColorPrint(track, meta, []);
		expect(colorPrintColor(lazy, 0, 1, 0)).toEqual([1, 0, 0]);
		expect(colorPrintColor(lazy, 1, 0, 0)).toEqual([0, 1, 0]);
		// Marqueurs du moteur (changement de couleur couche 1, pause couche 2).
		expect([...t.layers[1].markers.colorchange]).toEqual([1, 2, 3]);
		expect([...t.layers[2].markers.pause]).toEqual([4, 5, 6]);
		expect(t.colorPrint?.legend.map((e) => e.label)).toEqual([
			'Filament 1 · couches 1–1',
			'Filament 1 · couches 2–3',
			'Pause / G-code personnalisé'
		]);
		// Visionneuse (G-code ouvert) : viewer_palette.
		const v = withColorPrint(track, meta, [], cp.viewer_palette);
		expect(v.colorPrint?.palette[0]).toEqual([0, 0, 1]);
	});

	it('temps estimé au mouvement = elapsed de la couche + Σ temps (get_estimated_time_at)', () => {
		const m = [moves([])];
		// Mouvement 4 (couche 1) : 3 + (3 + 4) = 10 s ; silencieux : 6 + (6 + 8).
		expect(estimatedTimeAtMove(meta, m, 4, 0)).toEqual({ time: 10, moveTime: 4, move: 4 });
		expect(estimatedTimeAtMove(meta, m, 4, 1)?.time).toBe(20);
		// Couche 0 : le mouvement 0 n'est pas compté.
		expect(estimatedTimeAtMove(meta, m, 2, 0)?.time).toBe(3);
		// Dernier mouvement = total (3 + 7 + 11 = 21).
		expect(estimatedTimeAtMove(meta, m, 6, 0)?.time).toBe(21);
		// Bloc chargé partiel (débute couche 1) : couche 1 OK, couche 0 inconnue.
		const part = [moves([], 3)];
		expect(estimatedTimeAtMove(meta, part, 4, 0)?.time).toBe(10);
		expect(estimatedTimeAtMove(meta, part, 2, 0)).toBeNull();
		// Par sommet d'aperçu : segment 1 de la couche 1 = ligne 21 = mouvement 4.
		expect(estimatedTimeAtTool(track, meta, m, 1, 1, 0)?.time).toBe(10);
		// Départ de couche : temps écoulé avant la couche.
		expect(estimatedTimeAtTool(track, meta, m, 2, -1, 0)).toEqual({
			time: 10,
			moveTime: null,
			move: 5
		});
		// Fin de couche depuis la méta seule : elapsed + temps de couche.
		expect(layerEndTime(track, meta, 1, 0)).toEqual({ time: 10, moveTime: null, move: 4 });
		expect(layerEndTime(track, meta, 2, 1)?.time).toBe(42);
		// Méta sans elapsed (révision < 3) : inconnu.
		const old = {
			...meta,
			layers: meta.layers.map((l) => ({ ...l, elapsed: undefined }))
		} as EngineMeta;
		expect(estimatedTimeAtMove(old, m, 4, 0)).toBeNull();
	});

	it('coutures du moteur à la place des coutures déduites du texte', () => {
		const t = withEngineSeams(track, meta, [
			{ layer: 0, move: 1, x: 1, y: 1, z: 0.2 },
			{ layer: 2, move: 5, x: 2, y: 2, z: 0.6 }
		]);
		expect(t.seamSource).toBe('engine');
		expect([...t.layers[0].markers.seam]).toEqual([1, 1, Math.fround(0.2)]);
		expect(t.layers[1].markers.seam.length).toBe(0);
		expect(t.layers[2].markers.seam.length).toBe(3);
	});

	it('repères du curseur de couches', () => {
		expect(layerTicks(track, meta)).toEqual([
			{ layer: 1, type: 'color_change', label: 'Changement de couleur', time: [3, 6] },
			{ layer: 2, type: 'pause_print', label: 'Pause', time: [10, 20] }
		]);
	});

	it('tableau Filament : colonnes affichées et répartition', () => {
		const u = { m: 0, g: 0 };
		const table: FilamentTable = {
			columns: { model: true, support: false, flushed: true, tower: true },
			rows: [],
			total: { model: u, support: u, flushed: u, tower: u, total: u }
		};
		expect(visibleFilamentColumns(table)).toEqual(['model', 'flushed', 'tower']);
		expect(hasFilamentBreakdown(table)).toBe(true);
		expect(
			hasFilamentBreakdown({
				...table,
				columns: { model: true, support: false, flushed: false, tower: false }
			})
		).toBe(false);
		expect(hasFilamentBreakdown(undefined)).toBe(false);
	});

	it('G-code externe : statistiques simulées prioritaires sur le pied', () => {
		const s = statsFromProcessed({
			stats: { estimated_time_seconds: 600, total_filament_weight_g: 3.9, layer_count: 100 },
			footer: {
				estimated_time_seconds: 595,
				estimated_time_text: '9m 55s',
				total_filament_cost: 0.08
			}
		});
		expect(s.estimated_time_seconds).toBe(600);
		expect(s.estimated_time_text).toBeUndefined();
		expect(s.total_filament_cost).toBe(0.08);
		expect(s.layer_count).toBe(100);
		const same = statsFromProcessed({
			stats: { estimated_time_seconds: 595 },
			footer: { estimated_time_seconds: 595, estimated_time_text: '9m 55s' }
		});
		expect(same.estimated_time_text).toBe('9m 55s');
		expect(statsFromProcessed({ stats: { layer_count: 3 }, footer: null }).layer_count).toBe(3);
	});
});

// Les libellés de ces modules sont traduits (tr) : valeurs attendues en français.
beforeAll(() => setLocale('fr'));
afterAll(() => setLocale('en'));
