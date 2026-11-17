// Données de la simulation GCodeProcessor d'Orca (moteur, libslic3r) pour
// l'aperçu : mouvements simulés (`WSPm`, cf. engine/src/api/gcode_process.rs)
// et résumé (temps par mode / couche / rôle / extrudeur, volumes, G-code
// personnalisés). `withEngine` superpose ces valeurs à la piste texte
// (track.ts, alignée sur les segments WSPv) par numéro de ligne G-code : chaque
// segment d'extrusion reçoit la valeur du `MoveVertex` de la même ligne. La
// piste texte reste le repli si le moteur échoue. Module pur (testé).
import type { PreviewTrack, TrackChannel, TrackLayer } from './track';
import { TRACK_CHANNELS } from './track';

/** Colonnes par mouvement (miroir de BYTES/FLOATS/INTS_PER_MOVE). */
export const MOVE_BYTES = 4;
export const MOVE_FLOATS = 16;
export const MOVE_INTS = 2;
/** Indices des flottants (engine::api::gcode_process::col). */
export const COL = {
	x: 0,
	y: 1,
	z: 2,
	feedrate: 3,
	actualFeedrate: 4,
	mm3PerMm: 5,
	width: 6,
	height: 7,
	fan: 8,
	temperature: 9,
	acceleration: 10,
	jerk: 11,
	pa: 12,
	timeNormal: 13,
	timeStealth: 14,
	deltaE: 15
} as const;
/** EMoveType d'Orca. */
export const MOVE_TYPE = {
	noop: 0,
	retract: 1,
	unretract: 2,
	seam: 3,
	toolChange: 4,
	colorChange: 5,
	pausePrint: 6,
	customGcode: 7,
	travel: 8,
	wipe: 9,
	extrude: 10
} as const;

export interface EngineMoves {
	from: number;
	to: number;
	firstMove: number;
	count: number;
	bytes: Uint8Array;
	floats: Float32Array;
	ints: Uint32Array;
}

export interface EngineLayer {
	z: number;
	first_move: number;
	end_move: number;
	/** Ligne G-code du premier mouvement (méta v2). */
	first_line?: number;
	time: [number, number];
}

export interface EngineCustomGcode {
	print_z: number;
	type: 'color_change' | 'pause_print' | 'tool_change' | 'template' | 'custom' | 'unknown';
	extruder: number;
	color: string;
	extra: string;
}

export interface EngineMode {
	time: number;
	prepare_time: number;
	custom_gcode_times: { type: string; time: number; remaining: number }[];
}

/** Méta de `/preview/processed/meta`. */
export interface EngineMeta {
	version: number;
	move_count: number;
	layer_count: number;
	layers: EngineLayer[];
	/** [Normal, Silencieux] par rôle d'extrusion (ExtrusionRole d'Orca). */
	role_times: [number, number][];
	travel_time: [number, number];
	wipe_time: [number, number];
	other_time: [number, number];
	extruder_times: Record<string, [number, number]>;
	max_actual_feedrate: number;
	max_actual_volumetric_rate: number;
	/** Bornes globales des grandeurs par segment d'extrusion (méta v2). */
	ranges?: Partial<Record<TrackChannel, [number, number]>>;
	summary: {
		modes: EngineMode[];
		total_volumes_per_extruder: Record<string, number>;
		model_volumes_per_extruder: Record<string, number>;
		support_volumes_per_extruder: Record<string, number>;
		wipe_tower_volumes_per_extruder: Record<string, number>;
		flush_per_filament: Record<string, number>;
		used_filaments_per_role: Record<string, { meters: number; grams: number }>;
		total_filament_changes: number;
		/** Changements d'extrudeur (changeur d'outils, ex. Prusa XL), compteur
		 *  distinct des changements de filament comme dans Orca. */
		total_extruder_changes: number;
		total_travel_distance: number;
		total_travel_moves: number;
		initial_layer_time: number;
		filament_diameters: number[];
		filament_densities: number[];
		filament_costs: number[];
		extruder_colors: string[];
		custom_gcode_per_print_z: EngineCustomGcode[];
	};
}

export class EngineFormatError extends Error {}

/** Décode un buffer `WSPm`. */
export function decodeEngineMoves(buffer: ArrayBuffer): EngineMoves {
	if (buffer.byteLength < 24) throw new EngineFormatError('buffer WSPm tronqué');
	const view = new DataView(buffer);
	const magic = String.fromCharCode(...new Uint8Array(buffer, 0, 4));
	if (magic !== 'WSPm') throw new EngineFormatError('magic WSPm absent');
	const from = view.getUint32(8, true);
	const to = view.getUint32(12, true);
	const firstMove = view.getUint32(16, true);
	const count = view.getUint32(20, true);
	let o = 24;
	const bytes = new Uint8Array(buffer.slice(o, o + count * MOVE_BYTES));
	o += count * MOVE_BYTES;
	o = Math.ceil(o / 4) * 4;
	const fEnd = o + count * MOVE_FLOATS * 4;
	const iEnd = fEnd + count * MOVE_INTS * 4;
	if (iEnd > buffer.byteLength) throw new EngineFormatError('buffer WSPm incohérent');
	const floats = new Float32Array(buffer.slice(o, fEnd));
	const ints = new Uint32Array(buffer.slice(fEnd, iEnd));
	return { from, to, firstMove, count, bytes, floats, ints };
}

/** Valeur d'une grandeur de piste pour le mouvement `i`. */
function channelValue(m: EngineMoves, i: number, ch: TrackChannel): number {
	const f = (c: number) => m.floats[i * MOVE_FLOATS + c];
	switch (ch) {
		case 'fan':
			return f(COL.fan);
		case 'temperature':
			return f(COL.temperature);
		case 'acceleration':
			return f(COL.acceleration);
		case 'jerk':
			return f(COL.jerk);
		case 'pa':
			return f(COL.pa);
		case 'actualSpeed':
			return f(COL.actualFeedrate);
		case 'actualFlow':
			return f(COL.actualFeedrate) * f(COL.mm3PerMm);
	}
}

/**
 * Superpose la simulation moteur à la piste texte : valeurs par segment
 * (mouvement d'extrusion de la même ligne G-code) et temps de couche (somme
 * des temps simulés des lignes de la couche, mode `mode` : 0 Normal, 1 Silencieux).
 */
export function withEngine(track: PreviewTrack, moves: EngineMoves, mode = 0): PreviewTrack {
	const byLine = new Map<number, number>();
	const lines: number[] = [];
	const times: number[] = [];
	const tcol = mode === 1 ? COL.timeStealth : COL.timeNormal;
	for (let i = 0; i < moves.count; i++) {
		const line = moves.ints[i * MOVE_INTS + 1];
		if (moves.bytes[i * MOVE_BYTES] === MOVE_TYPE.extrude && !byLine.has(line)) byLine.set(line, i);
		lines.push(line);
		times.push(moves.floats[i * MOVE_FLOATS + tcol]);
	}
	// Temps cumulé par ligne (mouvements triés par ligne).
	const order = lines.map((_, i) => i).sort((a, b) => lines[a] - lines[b]);
	const sortedLines = order.map((i) => lines[i]);
	const prefix = new Float64Array(order.length + 1);
	order.forEach((i, k) => (prefix[k + 1] = prefix[k] + times[i]));
	const elapsedBefore = (line: number) => {
		let lo = 0;
		let hi = sortedLines.length;
		while (lo < hi) {
			const mid = (lo + hi) >> 1;
			if (sortedLines[mid] < line) lo = mid + 1;
			else hi = mid;
		}
		return prefix[lo];
	};

	const present = Object.fromEntries(TRACK_CHANNELS.map((c) => [c, false])) as Record<
		TrackChannel,
		boolean
	>;
	let matched = 0;
	const layers: TrackLayer[] = track.layers.map((layer, li) => {
		const values = {} as Record<TrackChannel, Float32Array>;
		for (const ch of TRACK_CHANNELS) values[ch] = new Float32Array(layer.segments).fill(NaN);
		layer.lines.forEach((line, s) => {
			const i = byLine.get(line);
			if (i === undefined) return;
			matched++;
			for (const ch of TRACK_CHANNELS) {
				const v = channelValue(moves, i, ch);
				values[ch][s] = v;
				if (v !== 0) present[ch] = true;
			}
		});
		const start = li === 0 ? 0 : layer.startLine;
		const next = track.layers[li + 1];
		const end = next ? next.startLine : Number.MAX_SAFE_INTEGER;
		const time = elapsedBefore(end) - elapsedBefore(start);
		return { ...layer, values, time };
	});
	const total = track.layers.reduce((n, l) => n + l.segments, 0);
	// Corrélation insuffisante (G-code différent) : la piste texte reste.
	if (total > 0 && matched / total < 0.9) return track;
	const ranges = { ...track.ranges };
	for (const ch of TRACK_CHANNELS) {
		let lo = Infinity;
		let hi = -Infinity;
		for (const l of layers)
			for (const v of l.values[ch]) {
				if (Number.isNaN(v)) continue;
				if (v < lo) lo = v;
				if (v > hi) hi = v;
			}
		ranges[ch] = Number.isFinite(lo) ? [lo, hi] : [0, 0];
	}
	const lt = layers.map((l) => l.time);
	ranges.layerTime = lt.length ? [Math.min(...lt), Math.max(...lt)] : [0, 0];
	return { layers, ranges, present, source: 'engine' };
}

// ── Chargement par fenêtre de couches (gros G-code) ───────────────────────
//
// Méta seul au départ (temps de couche, bornes globales) ; les mouvements ne
// sont demandés (`/processed/moves?from=&to=`) que pour les couches affichées,
// par blocs de `CHUNK_LAYERS` couches moteur, gardés dans un LRU borné en octets.

export const CHUNK_LAYERS = 16;
/** Budget mémoire des blocs de mouvements gardés (76 o / mouvement). */
export const CHUNK_BUDGET_BYTES = 64 * 1024 * 1024;

/** Couche moteur contenant la ligne `line` (dernière dont first_line ≤ line). */
function engineLayerAt(meta: EngineMeta, line: number): number {
	const layers = meta.layers;
	let lo = 0;
	let hi = layers.length - 1;
	let best = 0;
	while (lo <= hi) {
		const mid = (lo + hi) >> 1;
		if ((layers[mid].first_line ?? 0) <= line) {
			best = mid;
			lo = mid + 1;
		} else hi = mid - 1;
	}
	return best;
}

/** Couches moteur [from, to] couvrant les couches d'aperçu [low, high]. */
export function engineLayersFor(
	track: PreviewTrack,
	meta: EngineMeta,
	low: number,
	high: number
): [number, number] {
	if (!meta.layers.length || !track.layers.length) return [0, 0];
	const lo = Math.max(0, Math.min(low, track.layers.length - 1));
	const hi = Math.max(lo, Math.min(high, track.layers.length - 1));
	const first = track.layers[lo].lines[0] ?? track.layers[lo].startLine;
	const lastLayer = track.layers[hi];
	const last = lastLayer.lines[lastLayer.lines.length - 1] ?? lastLayer.startLine;
	return [engineLayerAt(meta, first), engineLayerAt(meta, last)];
}

/**
 * Temps de chaque couche d'aperçu depuis le méta seul : somme des temps des
 * couches moteur dont la 1re ligne tombe dans la couche d'aperçu (les deux
 * découpages suivent les mêmes marqueurs de changement de couche d'Orca).
 */
export function engineLayerTimes(track: PreviewTrack, meta: EngineMeta, mode = 0): number[] {
	const times = new Array<number>(track.layers.length).fill(0);
	if (!track.layers.length) return times;
	let li = 0;
	for (const el of meta.layers) {
		const line = el.first_line ?? 0;
		while (li + 1 < track.layers.length && track.layers[li + 1].startLine <= line) li++;
		times[li] += el.time[mode] ?? 0;
	}
	return times;
}

/** LRU de blocs de mouvements par index de bloc (couches moteur / CHUNK_LAYERS). */
export class EngineMoveCache {
	private readonly chunks = new Map<number, EngineMoves>();
	private readonly pending = new Map<number, Promise<EngineMoves>>();
	private bytes = 0;

	constructor(
		private readonly layerCount: number,
		private readonly fetchRange: (from: number, to: number) => Promise<EngineMoves>,
		private readonly budget = CHUNK_BUDGET_BYTES
	) {}

	get loadedBytes(): number {
		return this.bytes;
	}

	/** Blocs couvrant les couches moteur [from, to] (chargés au besoin). */
	async ensure(from: number, to: number): Promise<EngineMoves[]> {
		const out: EngineMoves[] = [];
		const last = Math.min(to, this.layerCount - 1);
		for (let k = Math.floor(from / CHUNK_LAYERS); k * CHUNK_LAYERS <= last; k++) {
			out.push(await this.chunk(k));
		}
		return out;
	}

	/** Blocs déjà en mémoire (rendu sans attendre). */
	loaded(): EngineMoves[] {
		return [...this.chunks.values()];
	}

	private async chunk(k: number): Promise<EngineMoves> {
		const hit = this.chunks.get(k);
		if (hit) {
			this.chunks.delete(k);
			this.chunks.set(k, hit);
			return hit;
		}
		let p = this.pending.get(k);
		if (!p) {
			const from = k * CHUNK_LAYERS;
			p = this.fetchRange(from, Math.min(from + CHUNK_LAYERS, this.layerCount) - 1);
			this.pending.set(k, p);
		}
		try {
			const moves = await p;
			if (!this.chunks.has(k)) {
				this.chunks.set(k, moves);
				this.bytes += moves.floats.byteLength + moves.ints.byteLength + moves.bytes.byteLength;
				for (const [key, old] of this.chunks) {
					if (this.bytes <= this.budget || key === k) break;
					this.chunks.delete(key);
					this.bytes -= old.floats.byteLength + old.ints.byteLength + old.bytes.byteLength;
				}
			}
			return moves;
		} finally {
			this.pending.delete(k);
		}
	}
}

/**
 * Piste d'aperçu à partir du méta et des blocs de mouvements chargés : temps
 * de couche (mode Normal / Silencieux) et bornes globales pour toutes les
 * couches ; valeurs par segment pour les couches dont les mouvements sont en
 * mémoire (NaN = « aucune donnée » ailleurs, le temps d'un chargement).
 */
export function applyEngine(
	track: PreviewTrack,
	meta: EngineMeta,
	chunks: readonly EngineMoves[],
	mode = 0
): PreviewTrack {
	const located = new Map<number, [EngineMoves, number]>();
	for (const m of chunks)
		for (let i = 0; i < m.count; i++)
			if (m.bytes[i * MOVE_BYTES] === MOVE_TYPE.extrude) {
				const line = m.ints[i * MOVE_INTS + 1];
				if (!located.has(line)) located.set(line, [m, i]);
			}
	const times = engineLayerTimes(track, meta, mode);
	let considered = 0;
	let matched = 0;
	const layers: TrackLayer[] = track.layers.map((layer, li) => {
		const values = {} as Record<TrackChannel, Float32Array>;
		for (const ch of TRACK_CHANNELS) values[ch] = new Float32Array(layer.segments).fill(NaN);
		let any = false;
		layer.lines.forEach((line, s) => {
			const hit = located.get(line);
			if (!hit) return;
			any = true;
			matched++;
			for (const ch of TRACK_CHANNELS) values[ch][s] = channelValue(hit[0], hit[1], ch);
		});
		if (any) considered += layer.segments;
		return { ...layer, values, time: times[li] };
	});
	// Corrélation insuffisante sur les couches chargées : G-code différent.
	if (considered > 0 && matched / considered < 0.9) return track;
	const ranges = { ...track.ranges };
	const present = { ...track.present };
	for (const ch of TRACK_CHANNELS) {
		const r = meta.ranges?.[ch];
		if (r) {
			ranges[ch] = r;
			present[ch] = r[0] !== 0 || r[1] !== 0;
		}
	}
	ranges.layerTime = times.length ? [Math.min(...times), Math.max(...times)] : [0, 0];
	return { layers, ranges, present, source: 'engine' };
}

/** Lignes du panneau « Résumé » (GCodeViewer::render_legend d'Orca). */
export interface SummaryRow {
	label: string;
	time: number;
	percent: number;
}

export interface EngineSummaryView {
	total: [number, number];
	prepare: [number, number];
	/**
	 * Temps de la première couche (s) [Normal, Silencieux] : couche 1 d'Orca
	 * (TimeBlock.layer_id = max(1, couche), démarrage inclus) = `initial_layer_time`.
	 * N.B. l'en-tête G-code « estimated first layer printing time » d'Orca
	 * contient en fait `prepare_time` (GCodeProcessor.cpp:1059) → `prepare`.
	 */
	firstLayer: [number, number];
	hasStealth: boolean;
	roles: SummaryRow[];
	extruders: { extruder: number; time: number; volumeMm3: number; meters: number; grams: number }[];
	customGcodes: EngineCustomGcode[];
	colorChangeTimes: { type: string; time: number; remaining: number }[];
	travelDistance: number;
	filamentChanges: number;
	extruderChanges: number;
}

/** Construit la vue Résumé (mode 0 Normal / 1 Silencieux). */
export function summaryView(
	meta: EngineMeta,
	roleNames: Record<number, string>,
	mode = 0
): EngineSummaryView {
	const s = meta.summary;
	const total = (s.modes[mode]?.time ?? 0) || 1;
	const roles: SummaryRow[] = [];
	meta.role_times.forEach((t, role) => {
		if (t[mode] > 0)
			roles.push({
				label: roleNames[role] ?? `Rôle ${role}`,
				time: t[mode],
				percent: (100 * t[mode]) / total
			});
	});
	for (const [label, t] of [
		['Déplacements', meta.travel_time],
		['Essuyages', meta.wipe_time]
	] as const)
		if (t[mode] > 0) roles.push({ label, time: t[mode], percent: (100 * t[mode]) / total });
	roles.sort((a, b) => b.time - a.time);
	const extruders = Object.entries(s.total_volumes_per_extruder ?? {}).map(([k, vol]) => {
		const e = Number(k);
		const d = s.filament_diameters[e] ?? 1.75;
		const density = s.filament_densities[e] ?? 0;
		const meters = vol / (Math.PI * (d / 2) ** 2) / 1000;
		return {
			extruder: e,
			time: meta.extruder_times[k]?.[mode] ?? 0,
			volumeMm3: vol,
			meters,
			grams: (vol / 1000) * density
		};
	});
	return {
		total: [s.modes[0]?.time ?? 0, s.modes[1]?.time ?? 0],
		prepare: [s.modes[0]?.prepare_time ?? 0, s.modes[1]?.prepare_time ?? 0],
		firstLayer: [
			s.initial_layer_time ?? meta.layers?.[0]?.time[0] ?? 0,
			meta.layers?.[0]?.time[1] ?? 0
		],
		hasStealth: (s.modes[1]?.time ?? 0) > 0 && s.modes[1].time !== s.modes[0].time,
		roles,
		extruders,
		customGcodes: s.custom_gcode_per_print_z ?? [],
		colorChangeTimes: s.modes[mode]?.custom_gcode_times ?? [],
		travelDistance: s.total_travel_distance ?? 0,
		filamentChanges: s.total_filament_changes ?? 0,
		extruderChanges: s.total_extruder_changes ?? 0
	};
}
