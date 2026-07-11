// Construction de la géométrie de prévisualisation (T068, R6). À partir des
// segments décodés d'une plage de couches, produit les tableaux `positions` et
// `colors` (deux sommets par segment → `THREE.LineSegments`), colorés selon la
// coloration active et filtrés par visibilité de type. Module pur (aucune
// dépendance Three.js) : la géométrie GPU est assemblée dans le composant.

import type { PreviewSegments } from './decode';
import {
	DEFAULT_FILAMENT_COLORS,
	NO_DATA_COLOR,
	rangeColor,
	SEGMENT_ROLE_COLORS,
	SEGMENT_ROLE_NAMES,
	sampleRange,
	type Coloration,
	type Legend,
	type LegendEntry,
	type Rgb
} from './colorations';

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
}

/** Géométrie de lignes prête pour `THREE.BufferGeometry`. */
export interface PreviewGeometry {
	/** Positions (x,y,z), 2 sommets par segment visible. */
	positions: Float32Array;
	/** Couleurs (r,g,b) par sommet. */
	colors: Float32Array;
	/** Nombre de segments visibles. */
	visibleCount: number;
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
	flowRange: [number, number]
): Rgb {
	const { coloration, ranges } = options;
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
		case 'temperature':
			// La température par segment n'est pas portée par le buffer de préviz
			// (émise en M104/M109 épars) : coloration neutre en attendant la piste
			// température côté moteur.
			return NO_DATA_COLOR;
	}
}

/**
 * Construit la géométrie de lignes des segments visibles d'une plage de couches.
 * Les segments dont le type est masqué sont écartés (pas de sommet émis).
 */
export function buildPreviewGeometry(
	segments: PreviewSegments,
	options: GeometryOptions
): PreviewGeometry {
	const { visibleTypes } = options;
	const flowRange = options.ranges.flow ?? computeFlowRange(segments);

	// Première passe : compter les segments visibles pour dimensionner.
	let visible = 0;
	for (let i = 0; i < segments.count; i++) {
		if (!visibleTypes || visibleTypes.has(segments.kind[i])) visible++;
	}

	const positions = new Float32Array(visible * 6);
	const colors = new Float32Array(visible * 6);

	let v = 0;
	for (let i = 0; i < segments.count; i++) {
		if (visibleTypes && !visibleTypes.has(segments.kind[i])) continue;
		const [r, g, b] = segmentColor(segments, i, options, flowRange);
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
		v++;
	}

	return { positions, colors, visibleCount: visible };
}

/** Type présent (id + nom) pour la légende du mode « type ». */
export interface PreviewType {
	id: number;
	name: string;
}

/** Construit la légende de la coloration active (liste ou échelle). */
export function buildLegend(
	coloration: Coloration,
	ctx: {
		ranges: PreviewRanges;
		typesPresent: PreviewType[];
		extrudersPresent: number[];
		filamentColors?: Rgb[];
	}
): Legend {
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
		case 'temperature':
			return {
				kind: 'list',
				entries: [{ label: 'Température indisponible', color: NO_DATA_COLOR }]
			};
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
	}
}

/** Légende continue : dégradé échantillonné + bornes/unité. */
function scaleLegend(min: number, max: number, unit: string): Legend {
	const stops: Rgb[] = [];
	const n = 8;
	for (let i = 0; i < n; i++) stops.push(sampleRange(i / (n - 1)));
	return { kind: 'scale', min, max, unit, stops };
}
