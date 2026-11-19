// Position de l'outil au curseur horizontal (phase 31) — pendant du marqueur
// et de la fenêtre « ToolPosition » d'Orca (GCodeViewer::SequentialView :
// render_marker + propriétés du sommet courant). Module pur : à partir des
// segments WSPv de la couche haute et de la position du curseur (nombre de
// moves affichés), renvoie le point atteint et les grandeurs du segment.
import type { PreviewSegments } from './decode';
import type { PreviewTrack, TrackChannel } from './track';
import { TRACK_CHANNELS } from './track';
import { flowValue } from './geometry';

/** Sommet courant de l'aperçu (fin du dernier segment affiché de la couche). */
export interface ToolPosition {
	/** Couche (0-based). */
	layer: number;
	/** Rang du segment dans la couche (0-based) ; -1 = aucun segment affiché. */
	index: number;
	/** Position de la buse (mm). */
	position: [number, number, number];
	/** Rôle d'extrusion (null au départ de la couche). */
	kind: number | null;
	/** Extrudeur / outil (0-based). */
	extruder: number | null;
	/** Vitesse commandée (mm/s). */
	speed: number | null;
	/** Largeur / hauteur de ligne (mm). */
	width: number | null;
	height: number | null;
	/** Débit volumétrique (mm³/s). */
	flow: number | null;
	/** Grandeurs de la piste G-code (ventilateur, température, accélération…). */
	channels: Partial<Record<TrackChannel, number>>;
	/** Temps nominal de la couche (s), si la piste est chargée. */
	layerTime: number | null;
}

/**
 * Sommet courant pour `position` moves affichés dans la couche `layer`
 * (même décompte que `buildPreviewGeometry` : tous les segments de la couche,
 * types masqués compris). `null` si la couche n'a aucun segment chargé.
 */
export function toolPositionAt(
	chunks: readonly PreviewSegments[],
	layer: number,
	position: number,
	track?: PreviewTrack | null
): ToolPosition | null {
	let ordinal = 0;
	let first: { seg: PreviewSegments; i: number } | null = null;
	let hit: { seg: PreviewSegments; i: number; ordinal: number } | null = null;
	for (const seg of chunks) {
		for (let i = 0; i < seg.count; i++) {
			if (seg.layer[i] !== layer) continue;
			first ??= { seg, i };
			if (ordinal < position) hit = { seg, i, ordinal };
			ordinal++;
		}
	}
	if (!first) return null;
	const trackLayer = track?.layers[layer];
	const layerTime = trackLayer && Number.isFinite(trackLayer.time) ? trackLayer.time : null;
	if (!hit) {
		const { seg, i } = first;
		return {
			layer,
			index: -1,
			position: [seg.start[i * 3], seg.start[i * 3 + 1], seg.start[i * 3 + 2]],
			kind: null,
			extruder: seg.extruder[i],
			speed: null,
			width: null,
			height: null,
			flow: null,
			channels: {},
			layerTime
		};
	}
	const { seg, i } = hit;
	const channels: Partial<Record<TrackChannel, number>> = {};
	if (trackLayer && trackLayer.segments === ordinal) {
		for (const ch of TRACK_CHANNELS) {
			const v = trackLayer.values[ch]?.[hit.ordinal];
			if (v !== undefined && Number.isFinite(v)) channels[ch] = v;
		}
	}
	return {
		layer,
		index: hit.ordinal,
		position: [seg.end[i * 3], seg.end[i * 3 + 1], seg.end[i * 3 + 2]],
		kind: seg.kind[i],
		extruder: seg.extruder[i],
		speed: seg.feedrate[i] / 60,
		width: seg.width[i],
		height: seg.height[i],
		flow: flowValue(seg.width[i], seg.height[i], seg.feedrate[i]),
		channels,
		layerTime
	};
}
