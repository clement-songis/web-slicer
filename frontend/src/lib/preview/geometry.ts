// Construction de la géométrie de prévisualisation (T068, R6). À partir des
// segments décodés d'une plage de couches, produit les tableaux `positions` et
// `colors` (deux sommets par segment → `THREE.LineSegments`), colorés selon la
// coloration active et filtrés par visibilité de type. Module pur (aucune
// dépendance Three.js) : la géométrie GPU est assemblée dans le composant.

import type { PreviewSegments } from './decode';
import {
	DEFAULT_FILAMENT_COLORS,
	NO_DATA_COLOR,
	OPTION_COLORS,
	RANGE_PALETTE,
	rangeColor,
	rangeValues,
	SEGMENT_ROLE_COLORS,
	SEGMENT_ROLE_NAMES,
	sampleRange,
	type Coloration,
	type Legend,
	type LegendEntry,
	type Rgb
} from './colorations';
import type { PreviewTrack, TrackChannel, TrackMarker } from './track';
import { colorPrintColor } from './engine-data';

/** Bornes (min,max) des attributs continus (issues de `PreviewMeta`). */
export interface PreviewRanges {
	feedrate: [number, number];
	width: [number, number];
	height: [number, number];
	/** Débit volumétrique (mm³/s) ; calculé si absent. */
	flow?: [number, number];
}

/** Options de coloration/visibilité. */
export interface GeometryOptions {
	coloration: Coloration;
	ranges: PreviewRanges;
	/** Ids de types visibles ; si absent, tout est visible. */
	visibleTypes?: Set<number>;
	/** Couleurs de filament par extrudeur (repli : palette par défaut). */
	filamentColors?: Rgb[];
	/**
	 * Plage de couches visibles `[low, high]` (incluses, comparées à
	 * `segments.layer`). Si absente, aucune restriction par couche
	 * (rétrocompatible). Mode une-couche : `low === high`. Bornes inversées
	 * tolérées (normalisées).
	 */
	layerRange?: LayerBounds;
	/** Curseur horizontal : nombre de moves affichés sur la couche haute. */
	moveCursor?: { layer: number; position: number };
	/** Piste G-code alignée (ventilateur, température, temps de couche…). */
	track?: PreviewTrack | null;
}

/** Coloration servie par une grandeur de la piste, ou null. */
const TRACK_COLORATION: Partial<Record<Coloration, TrackChannel>> = {
	fan: 'fan',
	temperature: 'temperature',
	acceleration: 'acceleration',
	jerk: 'jerk',
	pa: 'pa',
	'actual-speed': 'actualSpeed',
	'actual-flow': 'actualFlow'
};

/** Bornes de couches incluses (cf. `LayerRange.low/high` des curseurs). */
export interface LayerBounds {
	low: number;
	high: number;
}

/** Vrai si le segment `i` passe les filtres type + couche des options. */
export function isSegmentVisible(
	segments: PreviewSegments,
	i: number,
	options: Pick<GeometryOptions, 'visibleTypes' | 'layerRange'>
): boolean {
	const { visibleTypes, layerRange } = options;
	if (visibleTypes && !visibleTypes.has(segments.kind[i])) return false;
	if (layerRange) {
		const lo = Math.min(layerRange.low, layerRange.high);
		const hi = Math.max(layerRange.low, layerRange.high);
		const layer = segments.layer[i];
		if (layer < lo || layer > hi) return false;
	}
	return true;
}

/** Géométrie de lignes prête pour `THREE.BufferGeometry`. */
export interface PreviewGeometry {
	/** Positions (x,y,z), 2 sommets par segment visible. */
	positions: Float32Array;
	/** Couleurs (r,g,b) par sommet. */
	colors: Float32Array;
	/** Nombre de segments visibles. */
	visibleCount: number;
	/** Largeur et hauteur (mm) par segment visible (rendu en volumes). */
	sizes?: Float32Array;
}

/** Débit volumétrique approché (mm³/s) : section × vitesse. */
export function flowValue(width: number, height: number, feedrate: number): number {
	return width * height * (feedrate / 60);
}

/** Calcule les bornes (min,max) du débit sur les segments extrudés. */
export function computeFlowRange(segments: PreviewSegments): [number, number] {
	let min = Infinity;
	let max = -Infinity;
	for (let i = 0; i < segments.count; i++) {
		const f = flowValue(segments.width[i], segments.height[i], segments.feedrate[i]);
		if (f > 0) {
			min = Math.min(min, f);
			max = Math.max(max, f);
		}
	}
	return Number.isFinite(min) ? [min, max] : [0, 0];
}

/** Couleur d'un segment `i` selon la coloration active. */
function segmentColor(
	segments: PreviewSegments,
	i: number,
	options: GeometryOptions,
	flowRange: [number, number],
	ordinal: number
): Rgb {
	const { coloration, ranges, track } = options;
	const channel = TRACK_COLORATION[coloration];
	if (channel) {
		const layer = track?.layers[segments.layer[i]];
		if (!track || !layer || !track.present[channel]) return NO_DATA_COLOR;
		const [lo, hi] = track.ranges[channel];
		const value = layer.values[channel][ordinal] ?? lo;
		// Segment sans mouvement moteur correspondant : « aucune donnée ».
		if (Number.isNaN(value)) return NO_DATA_COLOR;
		return rangeColor(value, lo, hi);
	}
	if (coloration === 'layer-time' || coloration === 'layer-time-log') {
		const layer = track?.layers[segments.layer[i]];
		if (!track || !layer) return NO_DATA_COLOR;
		const [lo, hi] = track.ranges.layerTime;
		return rangeColor(layer.time, lo, hi, coloration === 'layer-time-log');
	}
	if (coloration === 'color-print') {
		const c = track
			? colorPrintColor(track, segments.layer[i], ordinal, segments.extruder[i])
			: null;
		if (c) return c;
		// Méta moteur absente : couleur du filament (comme Orca sans changement).
		const palette = options.filamentColors ?? DEFAULT_FILAMENT_COLORS;
		return palette[segments.extruder[i] % palette.length] ?? NO_DATA_COLOR;
	}
	switch (coloration) {
		case 'type':
			return SEGMENT_ROLE_COLORS[segments.kind[i]] ?? NO_DATA_COLOR;
		case 'speed':
			return rangeColor(segments.feedrate[i], ranges.feedrate[0], ranges.feedrate[1]);
		case 'height':
			return rangeColor(segments.height[i], ranges.height[0], ranges.height[1]);
		case 'width':
			return rangeColor(segments.width[i], ranges.width[0], ranges.width[1]);
		case 'flow':
			return rangeColor(
				flowValue(segments.width[i], segments.height[i], segments.feedrate[i]),
				flowRange[0],
				flowRange[1]
			);
		case 'filament': {
			const palette = options.filamentColors ?? DEFAULT_FILAMENT_COLORS;
			return palette[segments.extruder[i] % palette.length] ?? NO_DATA_COLOR;
		}
		default:
			return NO_DATA_COLOR;
	}
}

/**
 * Construit la géométrie de lignes des segments visibles d'une plage de couches.
 * Les segments dont le type est masqué ou dont la couche sort de
 * `options.layerRange` sont écartés (pas de sommet émis).
 */
export function buildPreviewGeometry(
	segments: PreviewSegments,
	options: GeometryOptions
): PreviewGeometry {
	const flowRange = options.ranges.flow ?? computeFlowRange(segments);
	// Le curseur horizontal consomme les moves de la couche haute même si leur
	// type est masqué : sa position reste comparable à la géométrie décodée.
	const visible = (i: number, ordinal: { value: number }) => {
		if (options.moveCursor && segments.layer[i] === options.moveCursor.layer) {
			const index = ordinal.value++;
			if (index >= options.moveCursor.position) return false;
		}
		return isSegmentVisible(segments, i, options);
	};

	// Première passe : compter les segments visibles pour dimensionner.
	let count = 0;
	const first = { value: 0 };
	for (let i = 0; i < segments.count; i++) {
		if (visible(i, first)) count++;
	}

	const positions = new Float32Array(count * 6);
	const colors = new Float32Array(count * 6);
	const sizes = new Float32Array(count * 2);

	let v = 0;
	const second = { value: 0 };
	// Rang du segment dans sa couche (index dans la piste G-code).
	let ordLayer = -1;
	let ord = -1;
	for (let i = 0; i < segments.count; i++) {
		if (segments.layer[i] !== ordLayer) {
			ordLayer = segments.layer[i];
			ord = 0;
		} else ord++;
		if (!visible(i, second)) continue;
		const [r, g, b] = segmentColor(segments, i, options, flowRange, ord);
		const p = v * 6;
		positions[p] = segments.start[i * 3];
		positions[p + 1] = segments.start[i * 3 + 1];
		positions[p + 2] = segments.start[i * 3 + 2];
		positions[p + 3] = segments.end[i * 3];
		positions[p + 4] = segments.end[i * 3 + 1];
		positions[p + 5] = segments.end[i * 3 + 2];
		colors[p] = r;
		colors[p + 1] = g;
		colors[p + 2] = b;
		colors[p + 3] = r;
		colors[p + 4] = g;
		colors[p + 5] = b;
		sizes[v * 2] = segments.width[i] > 0 ? segments.width[i] : 0.4;
		sizes[v * 2 + 1] = segments.height[i] > 0 ? segments.height[i] : 0.2;
		v++;
	}

	return { positions, colors, visibleCount: count, sizes };
}

/** Type présent (id + nom) pour la légende du mode « type ». */
export interface PreviewType {
	id: number;
	name: string;
}

/** Libellés et unités des colorations servies par la piste G-code. */
const TRACK_LEGEND: Record<TrackChannel, { unit: string; digits: number }> = {
	fan: { unit: '%', digits: 0 },
	temperature: { unit: '°C', digits: 0 },
	acceleration: { unit: 'mm/s²', digits: 0 },
	jerk: { unit: 'mm/s', digits: 1 },
	pa: { unit: '', digits: 3 },
	actualSpeed: { unit: 'mm/s', digits: 1 },
	actualFlow: { unit: 'mm³/s', digits: 2 }
};

/** Construit la légende de la coloration active (liste ou échelle). */
export function buildLegend(
	coloration: Coloration,
	ctx: {
		ranges: PreviewRanges;
		typesPresent: PreviewType[];
		extrudersPresent: number[];
		filamentColors?: Rgb[];
		track?: PreviewTrack | null;
	}
): Legend {
	const channel = TRACK_COLORATION[coloration];
	if (channel) {
		if (!ctx.track || !ctx.track.present[channel])
			return {
				kind: 'list',
				entries: [{ label: 'Aucune donnée dans ce G-code', color: NO_DATA_COLOR }]
			};
		const [lo, hi] = ctx.track.ranges[channel];
		return scaleLegend(lo, hi, TRACK_LEGEND[channel].unit);
	}
	if (coloration === 'color-print') {
		if (ctx.track?.colorPrint?.legend.length)
			return { kind: 'list', entries: ctx.track.colorPrint.legend };
		const palette = ctx.filamentColors ?? DEFAULT_FILAMENT_COLORS;
		return {
			kind: 'list',
			entries: ctx.extrudersPresent.map((e) => ({
				label: `Filament ${e + 1}`,
				color: palette[e % palette.length] ?? NO_DATA_COLOR
			}))
		};
	}
	if (coloration === 'layer-time' || coloration === 'layer-time-log') {
		if (!ctx.track)
			return { kind: 'list', entries: [{ label: 'Chargement du G-code…', color: NO_DATA_COLOR }] };
		const [lo, hi] = ctx.track.ranges.layerTime;
		return scaleLegend(lo, hi, 's', coloration === 'layer-time-log');
	}
	switch (coloration) {
		case 'type': {
			const entries: LegendEntry[] = ctx.typesPresent.map((t) => ({
				label: t.name || SEGMENT_ROLE_NAMES[t.id] || `#${t.id}`,
				color: SEGMENT_ROLE_COLORS[t.id] ?? NO_DATA_COLOR
			}));
			return { kind: 'list', entries };
		}
		case 'filament': {
			const palette = ctx.filamentColors ?? DEFAULT_FILAMENT_COLORS;
			const entries: LegendEntry[] = ctx.extrudersPresent.map((e) => ({
				label: `Filament ${e + 1}`,
				color: palette[e % palette.length] ?? NO_DATA_COLOR
			}));
			return { kind: 'list', entries };
		}
		case 'speed':
			return scaleLegend(ctx.ranges.feedrate[0] / 60, ctx.ranges.feedrate[1] / 60, 'mm/s');
		case 'height':
			return scaleLegend(ctx.ranges.height[0], ctx.ranges.height[1], 'mm');
		case 'width':
			return scaleLegend(ctx.ranges.width[0], ctx.ranges.width[1], 'mm');
		case 'flow': {
			const [min, max] = ctx.ranges.flow ?? [0, 0];
			return scaleLegend(min, max, 'mm³/s');
		}
		default:
			return { kind: 'list', entries: [{ label: 'Aucune donnée', color: NO_DATA_COLOR }] };
	}
}

/** Légende continue : palette d'Orca (11 arrêts) et une valeur par arrêt. */
function scaleLegend(min: number, max: number, unit: string, log = false): Legend {
	const values = rangeValues(min, max, log);
	const stops: Rgb[] = values.length === 1 ? [sampleRange(0.5)] : [...RANGE_PALETTE];
	return { kind: 'scale', min, max, unit, stops, log, values };
}

/** Options d'affichage (EOptionType d'Orca). */
export type PreviewOption = 'travel' | 'wipe' | TrackMarker;
export const PREVIEW_OPTIONS: readonly PreviewOption[] = [
	'travel',
	'wipe',
	'retract',
	'unretract',
	'seam',
	'toolchange',
	'colorchange',
	'pause'
];

/** Superposition : déplacements/essuyages (lignes) + marqueurs (points). */
export interface PreviewOverlay {
	lines: PreviewGeometry;
	points: PreviewGeometry;
}

/**
 * Géométrie des options d'affichage pour les couches visibles `[low,high]`
 * (couleurs `DEFAULT_OPTIONS_COLORS` d'Orca). En mode une couche, seule la
 * couche haute est concernée (`low === high`).
 */
export function buildOverlay(
	track: PreviewTrack,
	range: LayerBounds,
	enabled: ReadonlySet<PreviewOption>
): PreviewOverlay {
	const lo = Math.max(0, Math.min(range.low, range.high));
	const hi = Math.min(track.layers.length - 1, Math.max(range.low, range.high));
	const lp: number[] = [];
	const lc: number[] = [];
	const pp: number[] = [];
	const pc: number[] = [];
	for (let l = lo; l <= hi; l++) {
		const layer = track.layers[l];
		for (const kind of ['travel', 'wipe'] as const) {
			if (!enabled.has(kind)) continue;
			const src = kind === 'travel' ? layer.travels : layer.wipes;
			const [r, g, b] = OPTION_COLORS[kind];
			for (let i = 0; i < src.length; i++) lp.push(src[i]);
			for (let i = 0; i < src.length / 3; i++) lc.push(r, g, b);
		}
		for (const [kind, src] of Object.entries(layer.markers) as [TrackMarker, Float32Array][]) {
			if (!enabled.has(kind)) continue;
			const [r, g, b] = OPTION_COLORS[kind];
			for (let i = 0; i < src.length; i++) pp.push(src[i]);
			for (let i = 0; i < src.length / 3; i++) pc.push(r, g, b);
		}
	}
	return {
		lines: {
			positions: Float32Array.from(lp),
			colors: Float32Array.from(lc),
			visibleCount: lp.length / 6
		},
		points: {
			positions: Float32Array.from(pp),
			colors: Float32Array.from(pc),
			visibleCount: pp.length / 3
		}
	};
}
