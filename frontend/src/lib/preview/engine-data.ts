// Données d'aperçu d'Orca servies par le moteur (lot 31, phase 33) :
// vue « Couleur » (ColorPrint : `meta.color_print`, cp_color_id = octet 3 de
// WSPm), temps estimé à la position de l'outil (`layers[].elapsed` + temps des
// mouvements, get_estimated_time_at), coutures EMoveType::Seam, tableau
// « Filament » (modèle / support / purge / tour) par plateau et tous plateaux,
// G-code externe simulé (POST /api/gcode/process). Module pur (testé).
//
// Corrélation des couches : les couches de `/preview/processed/meta` (règle
// libvgcode) peuvent différer d'une unité de celles de `/preview/meta` (WSPv)
// sur un G-code à pause → rapprochement par `first_line` (jamais par index).
import type { Rgb } from './colorations';
import type { EngineCustomGcode, EngineMeta, EngineMoves } from './engine-track';
import { COL, MOVE_BYTES, MOVE_FLOATS, MOVE_INTS, MOVE_TYPE } from './engine-track';
import { parseOrcaColor } from './filament-colors';
import type { GcodeStats } from './stats';
import type { PreviewTrack, TrackLayer } from './track';

/** Gris des couches à pause / G-code perso (libvgcode DUMMY_COLOR). */
export const DUMMY_COLOR: Rgb = [0.25, 0.25, 0.25];
/** cp_color_id inconnu (mouvement pas encore chargé). */
export const NO_COLOR_ID = 255;

export interface ColorPrintItem {
	color_id: number;
	layer: number;
	time: [number, number];
}
export interface ColorPrintMarker {
	type: 'color_change' | 'pause_print' | 'custom' | string;
	layer: number;
	move: number;
	extruder: number;
	x: number;
	y: number;
	z: number;
	time: [number, number];
}
/** `meta.color_print` (lot 31 §2). */
export interface EngineColorPrint {
	palette: string[];
	viewer_palette?: string[];
	option_layers: number[];
	extruders: { extruder: number; items: ColorPrintItem[] }[];
	markers: ColorPrintMarker[];
	custom_gcodes: (EngineCustomGcode & { layer: number })[];
}

export type FilamentColumn = 'model' | 'support' | 'flushed' | 'tower';
export const FILAMENT_COLUMNS: readonly FilamentColumn[] = ['model', 'support', 'flushed', 'tower'];
export const FILAMENT_COLUMN_LABELS: Record<FilamentColumn, string> = {
	model: 'Modèle',
	support: 'Support',
	flushed: 'Purge',
	tower: 'Tour'
};
export interface FilamentUsage {
	volume?: number;
	m: number;
	g: number;
}
export interface FilamentTableRow {
	extruder: number;
	color: string;
	model: FilamentUsage;
	support: FilamentUsage;
	flushed: FilamentUsage;
	tower: FilamentUsage;
	total: FilamentUsage;
}
/** `meta.filament_table` / `plates-stats.all` (lot 31 §4). */
export interface FilamentTable {
	columns: Record<FilamentColumn, boolean>;
	rows: FilamentTableRow[];
	total: Record<FilamentColumn | 'total', FilamentUsage>;
}
export interface PlatesStatsResponse {
	plates: {
		gcode_id: string;
		time: [number, number];
		total_cost: number;
		filament_table: FilamentTable;
	}[];
	all: FilamentTable & { time: [number, number]; plate_count: number; total_cost: number };
}

export interface EngineSeam {
	layer: number;
	move: number;
	x: number;
	y: number;
	z: number;
}

/** `stats` de POST /api/gcode/process (GCodeProcessor). */
export interface ProcessedStats {
	source?: string;
	estimated_time_seconds?: number;
	estimated_time_exact_seconds?: number;
	estimated_time_stealth_seconds?: number;
	prepare_time_seconds?: number;
	layer_count?: number;
	filament_used_mm?: number[];
	filament_used_cm3?: number[];
	filament_used_g?: number[];
	filament_cost?: number[];
	total_filament_weight_g?: number;
	total_filament_cost?: number;
	total_toolchanges?: number;
	total_extruder_changes?: number;
}
export interface ProcessedGcode {
	token: string;
	stats: ProcessedStats;
	footer: GcodeStats | null;
	meta: EngineMeta;
}

/**
 * Statistiques d'un G-code externe : simulation GCodeProcessor (comme la
 * visionneuse d'Orca) complétée par le pied relu côté serveur pour les clés
 * absentes. Le texte de durée du pied est écarté s'il diffère de la simulation.
 */
export function statsFromProcessed(p: Pick<ProcessedGcode, 'stats' | 'footer'>): GcodeStats {
	const s = p.stats ?? {};
	const out: GcodeStats = { ...(p.footer ?? {}) };
	const keys = [
		'estimated_time_seconds',
		'layer_count',
		'filament_used_mm',
		'filament_used_cm3',
		'filament_used_g',
		'filament_cost',
		'total_filament_weight_g',
		'total_filament_cost',
		'total_toolchanges'
	] as const;
	for (const k of keys) if (s[k] !== undefined) (out as Record<string, unknown>)[k] = s[k];
	if (
		s.estimated_time_seconds !== undefined &&
		p.footer?.estimated_time_seconds !== s.estimated_time_seconds
	)
		delete out.estimated_time_text;
	return out;
}

/**
 * Couche d'aperçu (WSPv / piste texte) de chaque couche moteur, par
 * `first_line` : dernière couche d'aperçu dont la ligne de départ ≤ first_line.
 */
export function engineToPreviewLayers(track: PreviewTrack, meta: EngineMeta): Int32Array {
	const out = new Int32Array(meta.layers.length);
	let li = 0;
	meta.layers.forEach((el, k) => {
		const line = el.first_line ?? 0;
		while (li + 1 < track.layers.length && track.layers[li + 1].startLine <= line) li++;
		out[k] = li;
	});
	return out;
}

/** Couche moteur contenant le mouvement `m` (first_move ≤ m < end_move). */
export function layerIndexOfMove(meta: EngineMeta, m: number): number {
	const layers = meta.layers;
	let lo = 0;
	let hi = layers.length - 1;
	let best = -1;
	while (lo <= hi) {
		const mid = (lo + hi) >> 1;
		if (layers[mid].first_move <= m) {
			best = mid;
			lo = mid + 1;
		} else hi = mid - 1;
	}
	return best;
}

/** Couleurs hexadécimales → RGB (gris si illisible). */
export function paletteRgb(hex: readonly string[] | undefined): Rgb[] {
	return (hex ?? []).map((h) => parseOrcaColor(h) ?? [0.5, 0.5, 0.5]);
}

/** Entrée de légende de la vue Couleur. */
export interface ColorPrintLegendEntry {
	label: string;
	color: Rgb;
}

/**
 * Légende ColorPrint (render_legend d'Orca) : une ligne par extrudeur utilisé
 * sans changement, sinon une ligne par couleur avec sa plage de couches
 * (numéros d'aperçu, 1-based) ; « Pause / G-code personnalisé » en gris.
 */
export function colorPrintLegend(
	cp: EngineColorPrint,
	palette: readonly Rgb[],
	toPreview: (engineLayer: number) => number,
	layerCount: number
): ColorPrintLegendEntry[] {
	const out: ColorPrintLegendEntry[] = [];
	const color = (id: number) => palette[id % Math.max(1, palette.length)] ?? DUMMY_COLOR;
	for (const ex of cp.extruders) {
		const items = ex.items.length ? ex.items : [{ color_id: ex.extruder, layer: 0, time: [0, 0] }];
		if (items.length === 1) {
			out.push({ label: `Filament ${ex.extruder + 1}`, color: color(items[0].color_id) });
			continue;
		}
		items.forEach((it, i) => {
			const from = i === 0 ? 1 : toPreview(it.layer) + 1;
			const next = items[i + 1];
			const to = next ? Math.max(from, toPreview(next.layer)) : layerCount;
			out.push({
				label: `Filament ${ex.extruder + 1} · couches ${from}–${to}`,
				color: color(it.color_id)
			});
		});
	}
	if (cp.option_layers.length)
		out.push({ label: 'Pause / G-code personnalisé', color: DUMMY_COLOR });
	return out;
}

/** Piste enrichie de la vue Couleur. */
export interface ColorPrintTrack {
	palette: Rgb[];
	legend: ColorPrintLegendEntry[];
}

/**
 * Superpose la vue Couleur à la piste : cp_color_id par segment (mouvements
 * chargés, même ligne G-code), couleur par extrudeur au début de chaque couche
 * en repli (séquence `extruders[].items`), couches à pause / G-code perso
 * (`option_layers`) et marqueurs changement de couleur / pause du moteur.
 * `palette` = `palette` (plateau tranché) ou `viewer_palette` (G-code ouvert).
 */
export function withColorPrint(
	track: PreviewTrack,
	meta: EngineMeta,
	chunks: readonly EngineMoves[],
	paletteHex?: readonly string[]
): PreviewTrack {
	const cp = meta.color_print;
	if (!cp || !track.layers.length) return track;
	const palette = paletteRgb(paletteHex ?? cp.palette);
	const map = engineToPreviewLayers(track, meta);
	const toPreview = (l: number) => (l >= 0 && l < map.length ? map[l] : track.layers.length - 1);
	const byLine = new Map<number, number>();
	for (const m of chunks)
		for (let i = 0; i < m.count; i++)
			if (m.bytes[i * MOVE_BYTES] === MOVE_TYPE.extrude) {
				const line = m.ints[i * MOVE_INTS + 1];
				if (!byLine.has(line)) byLine.set(line, m.bytes[i * MOVE_BYTES + 3]);
			}
	// Dernière couche moteur de chaque couche d'aperçu.
	const lastEngine = new Int32Array(track.layers.length).fill(-1);
	map.forEach((li, k) => (lastEngine[li] = k));
	for (let li = 1; li < lastEngine.length; li++)
		if (lastEngine[li] < 0) lastEngine[li] = lastEngine[li - 1];
	const option = new Set(cp.option_layers.map(toPreview));
	const markerPos = (li: number, type: string) => {
		const out: number[] = [];
		for (const mk of cp.markers)
			if (mk.type === type && toPreview(mk.layer) === li) out.push(mk.x, mk.y, mk.z);
		return Float32Array.from(out);
	};
	const layers: TrackLayer[] = track.layers.map((layer, li) => {
		const colorIds = new Uint8Array(layer.segments).fill(NO_COLOR_ID);
		layer.lines.forEach((line, s) => {
			const c = byLine.get(line);
			if (c !== undefined) colorIds[s] = c;
		});
		const colorByExtruder: number[] = [];
		for (const ex of cp.extruders) {
			let id = ex.extruder;
			for (const it of ex.items) if (it.layer <= lastEngine[li]) id = it.color_id;
			colorByExtruder[ex.extruder] = id;
		}
		return {
			...layer,
			colorIds,
			colorByExtruder,
			optionLayer: option.has(li),
			markers: {
				...layer.markers,
				colorchange: markerPos(li, 'color_change'),
				pause: markerPos(li, 'pause_print')
			}
		};
	});
	const legend = colorPrintLegend(cp, palette, toPreview, track.layers.length);
	return { ...track, layers, colorPrint: { palette, legend } };
}

/** Couleur ColorPrint d'un segment (rang `ordinal` dans sa couche). */
export function colorPrintColor(
	track: PreviewTrack,
	layerIndex: number,
	ordinal: number,
	extruder: number
): Rgb | null {
	const palette = track.colorPrint?.palette;
	const layer = track.layers[layerIndex];
	if (!palette?.length || !layer) return null;
	if (layer.optionLayer) return DUMMY_COLOR;
	let id = layer.colorIds?.[ordinal];
	if (id === undefined || id === NO_COLOR_ID) id = layer.colorByExtruder?.[extruder] ?? extruder;
	return palette[id % palette.length];
}

/** Coutures du moteur à la place des coutures déduites du texte. */
export function withEngineSeams(
	track: PreviewTrack,
	meta: EngineMeta,
	seams: readonly EngineSeam[]
): PreviewTrack {
	if (!track.layers.length) return track;
	const map = engineToPreviewLayers(track, meta);
	const per: number[][] = track.layers.map(() => []);
	for (const s of seams) {
		const li = s.layer >= 0 && s.layer < map.length ? map[s.layer] : -1;
		if (li >= 0) per[li].push(s.x, s.y, s.z);
	}
	return {
		...track,
		seamSource: 'engine',
		layers: track.layers.map((l, li) => ({
			...l,
			markers: { ...l.markers, seam: Float32Array.from(per[li]) }
		}))
	};
}

/** Temps estimé au sommet courant (get_estimated_time_at d'Orca). */
export interface EstimatedTime {
	/** Temps écoulé depuis le début (s). */
	time: number;
	/** Temps du mouvement seul (s), null au départ de couche. */
	moveTime: number | null;
	/** Index global du mouvement moteur. */
	move: number;
}

/**
 * Temps au mouvement `m` : `layers[L].elapsed[mode] + Σ time[k]` pour
 * `k = max(first_move, 1) … m` (mouvements de la couche L, chargés).
 */
export function estimatedTimeAtMove(
	meta: EngineMeta,
	chunks: readonly EngineMoves[],
	m: number,
	mode = 0
): EstimatedTime | null {
	const L = layerIndexOfMove(meta, m);
	const layer = meta.layers[L];
	const elapsed = layer?.elapsed?.[mode];
	if (!layer || elapsed === undefined) return null;
	const chunk = chunks.find((c) => c.firstMove <= m && m < c.firstMove + c.count);
	if (!chunk) return null;
	const start = Math.max(layer.first_move, 1);
	if (start < chunk.firstMove) return null;
	const col = mode === 1 ? COL.timeStealth : COL.timeNormal;
	let time = elapsed;
	for (let k = start; k <= m; k++) time += chunk.floats[(k - chunk.firstMove) * MOVE_FLOATS + col];
	const moveTime = m >= 1 ? chunk.floats[(m - chunk.firstMove) * MOVE_FLOATS + col] : 0;
	return { time, moveTime, move: m };
}

/** Mouvement d'extrusion moteur de la ligne G-code `line` (index global). */
export function moveOfLine(chunks: readonly EngineMoves[], line: number): number | null {
	for (const c of chunks)
		for (let i = 0; i < c.count; i++)
			if (c.ints[i * MOVE_INTS + 1] === line && c.bytes[i * MOVE_BYTES] === MOVE_TYPE.extrude)
				return c.firstMove + i;
	return null;
}

/**
 * Temps estimé au sommet de la carte « Position de l'outil » : segment
 * `index` (rang dans la couche d'aperçu `layer`, -1 = départ de couche).
 * Départ de couche : temps écoulé avant la 1re couche moteur de la couche.
 */
export function estimatedTimeAtTool(
	track: PreviewTrack,
	meta: EngineMeta,
	chunks: readonly EngineMoves[],
	layer: number,
	index: number,
	mode = 0
): EstimatedTime | null {
	const tl = track.layers[layer];
	if (!tl || !meta.layers.length) return null;
	if (index < 0) {
		const map = engineToPreviewLayers(track, meta);
		const k = map.findIndex((li) => li === layer);
		const el = k >= 0 ? meta.layers[k] : undefined;
		const e = el?.elapsed?.[mode];
		return el && e !== undefined ? { time: e, moveTime: null, move: el.first_move } : null;
	}
	const line = tl.lines[index];
	if (line === undefined) return null;
	const m = moveOfLine(chunks, line);
	return m === null ? null : estimatedTimeAtMove(meta, chunks, m, mode);
}

/**
 * Temps à la fin de la couche d'aperçu `layer` (curseur horizontal en fin de
 * couche, position par défaut) : `elapsed + time` de sa dernière couche
 * moteur, depuis la méta seule (aucun mouvement à charger), = temps au dernier
 * sommet de la couche chez Orca.
 */
export function layerEndTime(
	track: PreviewTrack,
	meta: EngineMeta,
	layer: number,
	mode = 0
): EstimatedTime | null {
	const map = engineToPreviewLayers(track, meta);
	let k = -1;
	map.forEach((li, i) => {
		if (li === layer) k = i;
	});
	const el = k >= 0 ? meta.layers[k] : undefined;
	const e = el?.elapsed?.[mode];
	if (!el || e === undefined) return null;
	return { time: e + (el.time[mode] ?? 0), moveTime: null, move: Math.max(0, el.end_move - 1) };
}

/** Repère du curseur vertical (changement de couleur, pause, G-code perso). */
export interface LayerTick {
	/** Couche d'aperçu (0-based). */
	layer: number;
	type: string;
	label: string;
	time: [number, number];
}
const TICK_LABELS: Record<string, string> = {
	color_change: 'Changement de couleur',
	pause_print: 'Pause',
	custom: 'G-code personnalisé',
	tool_change: 'Changement d’outil',
	template: 'Modèle'
};
/** Repères du curseur de couches (markers du moteur, couches d'aperçu). */
export function layerTicks(track: PreviewTrack, meta: EngineMeta): LayerTick[] {
	const cp = meta.color_print;
	if (!cp?.markers?.length || !track.layers.length) return [];
	const map = engineToPreviewLayers(track, meta);
	return cp.markers.map((mk) => ({
		layer: mk.layer >= 0 && mk.layer < map.length ? map[mk.layer] : track.layers.length - 1,
		type: mk.type,
		label: TICK_LABELS[mk.type] ?? mk.type,
		time: mk.time
	}));
}

/** Colonnes affichées du tableau Filament (render_legend : entrée non nulle). */
export function visibleFilamentColumns(table: FilamentTable): FilamentColumn[] {
	return FILAMENT_COLUMNS.filter((c) => table.columns?.[c]);
}

/** Vrai si le tableau apporte plus que la seule colonne « Modèle ». */
export function hasFilamentBreakdown(table: FilamentTable | undefined | null): boolean {
	return !!table && visibleFilamentColumns(table).some((c) => c !== 'model');
}
