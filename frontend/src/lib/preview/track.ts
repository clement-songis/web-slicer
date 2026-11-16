// Piste G-code complète de l'aperçu (parité GCodeViewer d'Orca) : le modèle
// WSPv du backend ne porte que les extrusions (vitesse, largeur, hauteur, rôle,
// extrudeur). Cette piste relit le G-code texte côté client et produit, pour
// chaque segment d'extrusion — dans le MÊME ordre et le même découpage en
// couches que `backend/src/gcode.rs::parse_preview` (règles reproduites à
// l'identique, valeurs en f32) — les grandeurs manquantes : ventilateur,
// température, accélération, jerk, pressure advance ; et, par couche, le temps
// nominal, les déplacements, essuyages et marqueurs (rétractions, réamorçages,
// coutures, changements d'outil / de couleur, pauses).
//
// Module pur (testé). Temps : durée nominale (distance / vitesse commandée,
// sans accélérations) — Orca simule la cinématique ; l'ordre de grandeur et
// les écarts entre couches restent fidèles, la valeur absolue est optimiste.

/** Grandeurs portées par segment d'extrusion. */
export type TrackChannel =
	'fan' | 'temperature' | 'acceleration' | 'jerk' | 'pa' | 'actualSpeed' | 'actualFlow';
export const TRACK_CHANNELS: readonly TrackChannel[] = [
	'fan',
	'temperature',
	'acceleration',
	'jerk',
	'pa',
	'actualSpeed',
	'actualFlow'
];
/** Grandeurs que seule la simulation du moteur (GCodeProcessor) fournit :
 *  vitesse et débit réels (cinématique), absentes de la relecture texte. */
export const ENGINE_ONLY_CHANNELS: readonly TrackChannel[] = ['actualSpeed', 'actualFlow'];

/** Marqueurs ponctuels (EOptionType d'Orca hors déplacements/essuyages). */
export type TrackMarker = 'retract' | 'unretract' | 'seam' | 'toolchange' | 'colorchange' | 'pause';
export const TRACK_MARKERS: readonly TrackMarker[] = [
	'retract',
	'unretract',
	'seam',
	'toolchange',
	'colorchange',
	'pause'
];

export interface TrackLayer {
	/** Hauteur annoncée (`;Z:`) ou Z de la première extrusion. */
	z: number;
	/** Nombre de segments d'extrusion (= `count` de la couche WSPv). */
	segments: number;
	/** Valeurs par segment, dans l'ordre du G-code. */
	values: Record<TrackChannel, Float32Array>;
	/** Temps nominal de la couche (s), extrusions + déplacements. */
	time: number;
	/** Déplacements : x0,y0,z0,x1,y1,z1 à plat. */
	travels: Float32Array;
	/** Essuyages (`;WIPE_START`…`;WIPE_END`) : idem. */
	wipes: Float32Array;
	/** Marqueurs : x,y,z à plat. */
	markers: Record<TrackMarker, Float32Array>;
	/** Ligne G-code (1-based) de chaque segment d'extrusion (corrélation moteur). */
	lines: Uint32Array;
	/** Ligne du changement de couche (début de la couche, 1-based). */
	startLine: number;
}

export interface PreviewTrack {
	layers: TrackLayer[];
	/** Bornes par grandeur (segments d'extrusion) et temps de couche. */
	ranges: Record<TrackChannel | 'layerTime', [number, number]>;
	/** Grandeurs réellement présentes dans le G-code (sinon « aucune donnée »). */
	present: Record<TrackChannel, boolean>;
	/** Origine : relecture texte (repli) ou simulation GCodeProcessor du moteur. */
	source?: 'text' | 'engine';
}

const f32 = Math.fround;

interface BuildLayer {
	z: number;
	lines: number[];
	startLine: number;
	values: Record<TrackChannel, number[]>;
	time: number;
	travels: number[];
	wipes: number[];
	markers: Record<TrackMarker, number[]>;
}

function newLayer(startLine = 1): BuildLayer {
	return {
		z: 0,
		lines: [],
		startLine,
		values: {
			fan: [],
			temperature: [],
			acceleration: [],
			jerk: [],
			pa: [],
			actualSpeed: [],
			actualFlow: []
		},
		time: 0,
		travels: [],
		wipes: [],
		markers: { retract: [], unretract: [], seam: [], toolchange: [], colorchange: [], pause: [] }
	};
}

/** Paramètre `KEY=valeur` d'une macro Klipper (insensible à la casse). */
function macroParam(code: string, key: string): number | null {
	const m = code.match(new RegExp('\\b' + key + '=([-+0-9.eE]+)', 'i'));
	return m ? Number(m[1]) : null;
}

/** Paramètre lettre d'une commande (`S`, `P`, `T`…). */
function letterParam(tokens: string[], letter: string): number | null {
	for (const t of tokens) {
		if (t.length > 1 && t[0].toUpperCase() === letter) {
			const v = Number(t.slice(1));
			if (Number.isFinite(v)) return v;
		}
	}
	return null;
}

const OUTER_WALL = /^(outer wall|external perimeter)$/i;

/**
 * Relit un G-code et produit la piste d'aperçu alignée sur le modèle WSPv.
 */
export function parsePreviewTrack(gcode: string): PreviewTrack {
	const layers: BuildLayer[] = [newLayer()];
	const counts: number[] = [0];
	let pos: [number, number, number] = [0, 0, 0];
	let e = 0;
	let feedrate = 0;
	let extruder = 0;
	let relativeE = false;
	let outerWall = false;
	let inWipe = false;
	let lastWasOuterExtrusion = false;
	let fan = 0;
	const temps: number[] = [];
	let acceleration = 0;
	let jerk = 0;
	let pa = 0;
	const present: Record<TrackChannel, boolean> = {
		fan: false,
		temperature: false,
		acceleration: false,
		jerk: false,
		pa: false,
		actualSpeed: false,
		actualFlow: false
	};
	let lineNo = 0;

	const cur = () => layers[layers.length - 1];
	const push3 = (arr: number[], p: readonly number[]) => arr.push(p[0], p[1], p[2]);

	for (const line of gcode.split(/\r?\n/)) {
		lineNo++;
		const semi = line.indexOf(';');
		const code = (semi >= 0 ? line.slice(0, semi) : line).trim();
		const comment = semi >= 0 ? line.slice(semi + 1).trim() : null;

		if (comment !== null) {
			const feature = comment.startsWith('TYPE:')
				? comment.slice(5)
				: comment.trimStart().startsWith('FEATURE:')
					? comment.trimStart().slice(8)
					: null;
			if (feature !== null) {
				const next = OUTER_WALL.test(feature.trim());
				if (next !== outerWall) lastWasOuterExtrusion = false;
				outerWall = next;
			} else if (comment === 'LAYER_CHANGE' || comment === 'CHANGE_LAYER') {
				layers.push(newLayer(lineNo));
				counts.push(0);
			} else if (comment.startsWith('Z:')) {
				const z = Number(comment.slice(2).trim());
				if (Number.isFinite(z)) cur().z = f32(z);
			} else if (comment === 'WIPE_START') inWipe = true;
			else if (comment === 'WIPE_END') inWipe = false;
		}
		if (!code) continue;

		const tokens = code.split(/\s+/);
		// Comme le backend : commandes G/M sensibles à la casse ; macros Klipper en majuscules.
		const cmd = tokens[0];
		const layer = cur();
		if (cmd === 'G0' || cmd === 'G1') {
			const next: [number, number, number] = [pos[0], pos[1], pos[2]];
			let eParam: number | null = null;
			for (const tok of tokens.slice(1)) {
				const axis = tok[0];
				const val = Number(tok.slice(1));
				if (!tok.slice(1) || !Number.isFinite(val)) continue;
				if (axis === 'X') next[0] = f32(val);
				else if (axis === 'Y') next[1] = f32(val);
				else if (axis === 'Z') next[2] = f32(val);
				else if (axis === 'E') eParam = f32(val);
				else if (axis === 'F') feedrate = f32(val);
			}
			const moved = next[0] !== pos[0] || next[1] !== pos[1] || next[2] !== pos[2];
			const de = eParam === null ? 0 : relativeE ? eParam : eParam - e;
			const extruding = eParam !== null && (relativeE ? eParam > 0 : eParam > e);
			const dist = Math.hypot(next[0] - pos[0], next[1] - pos[1], next[2] - pos[2]);
			if (feedrate > 0) layer.time += (moved ? dist : Math.abs(de)) / (feedrate / 60);

			if (extruding) {
				if (layer.z === 0) layer.z = next[2];
				counts[counts.length - 1]++;
				layer.values.fan.push(fan);
				layer.values.temperature.push(temps[extruder] ?? 0);
				layer.values.acceleration.push(acceleration);
				layer.values.jerk.push(jerk);
				layer.values.pa.push(pa);
				layer.values.actualSpeed.push(NaN);
				layer.values.actualFlow.push(NaN);
				layer.lines.push(lineNo);
				if (!moved) push3(layer.markers.unretract, pos);
				else if (outerWall && !lastWasOuterExtrusion) push3(layer.markers.seam, pos);
				lastWasOuterExtrusion = outerWall && moved;
			} else {
				// Rétraction = un événement (les mouvements d'essuyage rétractent en roulant :
				// comptés comme essuyage, pas comme N rétractions — comme Orca).
				if (eParam !== null && de < 0 && !inWipe) push3(layer.markers.retract, pos);
				if (moved) {
					const target = inWipe ? layer.wipes : layer.travels;
					push3(target, pos);
					push3(target, next);
					lastWasOuterExtrusion = false;
				}
			}
			pos = next;
			if (eParam !== null && !relativeE) e = eParam;
		} else if (cmd === 'G92') {
			const v = letterParam(tokens.slice(1), 'E');
			if (v !== null) e = f32(v);
		} else if (cmd === 'M82') relativeE = false;
		else if (cmd === 'M83') relativeE = true;
		else if (cmd === 'M106') {
			const p = letterParam(tokens.slice(1), 'P');
			if (p === null || p === 0) {
				fan = ((letterParam(tokens.slice(1), 'S') ?? 255) / 255) * 100;
				present.fan = true;
			}
		} else if (cmd === 'M107') {
			const p = letterParam(tokens.slice(1), 'P');
			if (p === null || p === 0) fan = 0;
		} else if (cmd === 'M104' || cmd === 'M109') {
			const s = letterParam(tokens.slice(1), 'S');
			const t = letterParam(tokens.slice(1), 'T') ?? extruder;
			if (s !== null) {
				temps[t] = s;
				present.temperature = true;
			}
		} else if (cmd === 'M204') {
			const v = letterParam(tokens.slice(1), 'S') ?? letterParam(tokens.slice(1), 'P');
			if (v !== null) {
				acceleration = v;
				present.acceleration = true;
			}
		} else if (cmd === 'M205') {
			const v = letterParam(tokens.slice(1), 'X');
			if (v !== null) {
				jerk = v;
				present.jerk = true;
			}
		} else if (cmd === 'M900') {
			const v = letterParam(tokens.slice(1), 'K');
			if (v !== null) {
				pa = v;
				present.pa = true;
			}
		} else if (cmd === 'SET_VELOCITY_LIMIT') {
			const a = macroParam(code, 'ACCEL');
			if (a !== null) {
				acceleration = a;
				present.acceleration = true;
			}
			const j = macroParam(code, 'SQUARE_CORNER_VELOCITY');
			if (j !== null) {
				jerk = j;
				present.jerk = true;
			}
		} else if (cmd === 'SET_PRESSURE_ADVANCE') {
			const v = macroParam(code, 'ADVANCE');
			if (v !== null) {
				pa = v;
				present.pa = true;
			}
		} else if (cmd === 'M600') push3(layer.markers.colorchange, pos);
		else if (cmd === 'M601' || cmd === 'PAUSE' || cmd === 'M0' || cmd === 'M1')
			push3(layer.markers.pause, pos);
		else if (/^T\d+$/.test(cmd)) {
			extruder = Number(cmd.slice(1));
			push3(layer.markers.toolchange, pos);
		}
	}

	// Mêmes couches que le backend : seules celles portant des extrusions.
	const kept: TrackLayer[] = [];
	layers.forEach((l, i) => {
		if (counts[i] === 0) return;
		kept.push({
			z: l.z,
			segments: counts[i],
			values: {
				fan: Float32Array.from(l.values.fan),
				temperature: Float32Array.from(l.values.temperature),
				acceleration: Float32Array.from(l.values.acceleration),
				jerk: Float32Array.from(l.values.jerk),
				pa: Float32Array.from(l.values.pa),
				actualSpeed: Float32Array.from(l.values.actualSpeed),
				actualFlow: Float32Array.from(l.values.actualFlow)
			},
			lines: Uint32Array.from(l.lines),
			startLine: l.startLine,
			time: l.time,
			travels: Float32Array.from(l.travels),
			wipes: Float32Array.from(l.wipes),
			markers: {
				retract: Float32Array.from(l.markers.retract),
				unretract: Float32Array.from(l.markers.unretract),
				seam: Float32Array.from(l.markers.seam),
				toolchange: Float32Array.from(l.markers.toolchange),
				colorchange: Float32Array.from(l.markers.colorchange),
				pause: Float32Array.from(l.markers.pause)
			}
		});
	});

	const ranges = {} as PreviewTrack['ranges'];
	for (const ch of TRACK_CHANNELS) {
		let lo = Infinity;
		let hi = -Infinity;
		for (const l of kept)
			for (const v of l.values[ch]) {
				if (Number.isNaN(v)) continue;
				if (v < lo) lo = v;
				if (v > hi) hi = v;
			}
		ranges[ch] = Number.isFinite(lo) ? [lo, hi] : [0, 0];
	}
	const times = kept.map((l) => l.time);
	ranges.layerTime = times.length ? [Math.min(...times), Math.max(...times)] : [0, 0];
	return { layers: kept, ranges, present, source: 'text' };
}

/**
 * Vrai si la piste correspond au modèle WSPv chargé (même nombre de couches) :
 * sinon les valeurs par segment seraient décalées, on ne l'utilise pas.
 */
export function trackMatches(track: PreviewTrack, layerCount: number): boolean {
	return track.layers.length === layerCount;
}
