// Oreilles de bord (T057, gizmo BrimEars ; audit 45 EARS-a/b) — Orca
// GLGizmoBrimEars + libslic3r BrimPoint : points posés sur l'objet, stockés
// dans le repère de l'objet (maillage recentré, avant la transformation de
// l'instance) avec leur rayon de tête (`head_front_radius` = diamètre / 2),
// donc ils suivent l'objet quand on le déplace, le tourne ou le dispose.
// Auto-génération (generate_points : Douglas-Peucker + sommets convexes /
// concaves selon l'angle max), oreilles invalides (find_single : disque qui
// ne touche ni la 1ʳᵉ couche ni une oreille valide). Pur → testable.
import { Matrix4 } from 'three/src/math/Matrix4.js';
import { Vector3 } from 'three/src/math/Vector3.js';
import type { Transform } from '../transform';
import { worldMatrix } from '../tools/mesh-ops';

type Vec2 = [number, number];
type Vec3 = [number, number, number];

/** Point d'oreille (Orca BrimPoint) : position dans le repère de l'objet (mm)
 *  et rayon de tête (mm). */
export interface BrimEarPoint {
	x: number;
	y: number;
	z: number;
	r: number;
}

/** Paramètres du registre pilotant les oreilles de bord. */
export const BRIM_EARS_KEYS = [
	'brim_ears',
	'brim_ears_detection_length',
	'brim_ears_max_angle'
] as const;

/** Bornes du curseur « Diamètre de tête » (GLGizmoBrimEars : 5 – 20). */
export const HEAD_DIAMETER_MIN = 5;
export const HEAD_DIAMETER_MAX = 20;
/** Défauts du gizmo (GLGizmoBrimEars.hpp : m_max_angle 125, m_detection_radius 1). */
export const DEFAULT_MAX_ANGLE = 125;
export const DEFAULT_DETECTION_RADIUS = 1;
/** Rayon d'une oreille relue d'un ancien document (sans rayon) : 0,5 × 16 / 2. */
export const LEGACY_HEAD_RADIUS = 4;

/**
 * Diamètre de tête par défaut (get_brim_default_radius) :
 * initial_layer_line_width (absolu ou % de la buse) × 16.
 */
export function defaultHeadDiameter(
	initialLayerLineWidth: unknown,
	nozzleDiameter: unknown
): number {
	const nozzle = Number(String(nozzleDiameter ?? '0.4').split(/[;,]/)[0]) || 0.4;
	const raw = String(initialLayerLineWidth ?? '').trim();
	let width = raw.endsWith('%') ? (nozzle * Number(raw.slice(0, -1))) / 100 : Number(raw);
	if (!Number.isFinite(width) || width <= 0) width = 0.5;
	return Math.round(width * 16 * 100) / 100;
}

/** Document sérialisé des oreilles de bord d'un objet. */
export interface BrimEarsDocument {
	/** Position plateau (XY monde, mm) et diamètre de tête de chaque oreille,
	 *  recalculés à chaque enregistrement : `[x, y, diamètre]` — format lu par
	 *  le serveur (lot 47, `scene_brim_ears`) et écrit par l'import 3MF ;
	 *  `[x, y]` dans les anciens documents (diamètre par défaut). */
	points: ([number, number] | [number, number, number])[];
	/** Oreilles d'Orca (BrimPoint) : [x, y, z] dans le repère de l'objet
	 *  (maillage recentré, cf. worldMatrix) + rayon de tête, mm. Fait foi. */
	ears?: [number, number, number, number][];
}

function matrixOf(t: Transform): Matrix4 {
	return new Matrix4().fromArray(worldMatrix(t));
}

/** Repère objet → monde. */
export function earToWorld(p: { x: number; y: number; z: number }, t: Transform): Vec3 {
	const v = new Vector3(p.x, p.y, p.z).applyMatrix4(matrixOf(t));
	return [v.x, v.y, v.z];
}

/** Monde → repère objet. */
export function worldToEar(world: Vec3, t: Transform): Vec3 {
	const v = new Vector3(...world).applyMatrix4(matrixOf(t).invert());
	return [v.x, v.y, v.z];
}

const finite = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v);

/** Ensemble de points d'oreilles de bord d'un objet. */
export class BrimEars {
	private points: BrimEarPoint[] = [];

	/** Ajoute un point (repère objet) ; un doublon exact est ignoré
	 *  (add_point_to_cache). Renvoie le point ou null. */
	add(x: number, y: number, z = 0, r = LEGACY_HEAD_RADIUS): BrimEarPoint | null {
		if (this.points.some((p) => p.x === x && p.y === y && p.z === z && p.r === r)) return null;
		const p = { x, y, z, r };
		this.points.push(p);
		return p;
	}

	/** Supprime le point d'indice `i` (ignore un indice hors limites). */
	removeAt(i: number): void {
		if (i >= 0 && i < this.points.length) this.points.splice(i, 1);
	}

	/** Supprime plusieurs points (« Retirer : sélection »). */
	removeMany(indices: Iterable<number>): void {
		const drop = new Set(indices);
		this.points = this.points.filter((_, i) => !drop.has(i));
	}

	/** Change le rayon de tête de points (curseur appliqué à la sélection). */
	setRadius(indices: Iterable<number>, r: number): void {
		for (const i of indices) if (this.points[i]) this.points[i] = { ...this.points[i], r };
	}

	clear(): void {
		this.points = [];
	}

	/** Copie de la liste (une nouvelle référence à chaque appel : la vue se
	 *  met à jour après chaque ajout — audit 45 EARS-b, 1 oreille sur 3). */
	list(): BrimEarPoint[] {
		return this.points.map((p) => ({ ...p }));
	}

	get count(): number {
		return this.points.length;
	}

	/** Copie indépendante (état réactif de la page). */
	clone(): BrimEars {
		const out = new BrimEars();
		out.points = this.list();
		return out;
	}

	/** Sérialise pour le document scène ; `transform` = transformation courante
	 *  de l'objet (positions plateau de `points`). */
	serialize(transform?: Transform): BrimEarsDocument {
		return {
			points: this.points.map((p) => {
				const w = transform ? earToWorld(p, transform) : [p.x, p.y, p.z];
				return [w[0], w[1], p.r * 2] as [number, number, number];
			}),
			ears: this.points.map((p) => [p.x, p.y, p.z, p.r])
		};
	}

	/** Reconstruit depuis un document scène (`ears` si présent, sinon les
	 *  positions plateau d'un ancien document ramenées dans le repère objet). */
	static deserialize(doc: BrimEarsDocument | null | undefined, transform?: Transform): BrimEars {
		const ears = new BrimEars();
		const local = Array.isArray(doc?.ears) ? doc.ears : null;
		if (local && local.length) {
			for (const e of local) {
				if (!Array.isArray(e) || !finite(e[0]) || !finite(e[1])) continue;
				ears.add(
					e[0],
					e[1],
					finite(e[2]) ? e[2] : 0,
					finite(e[3]) && e[3] > 0 ? e[3] : LEGACY_HEAD_RADIUS
				);
			}
			return ears;
		}
		for (const pt of doc?.points ?? []) {
			if (!Array.isArray(pt) || !finite(pt[0]) || !finite(pt[1])) continue;
			const d = (pt as number[])[2];
			const r = finite(d) && d > 0 ? d / 2 : LEGACY_HEAD_RADIUS;
			if (transform) {
				const l = worldToEar([pt[0], pt[1], 0], transform);
				ears.add(l[0], l[1], l[2], r);
			} else ears.add(pt[0], pt[1], 0, r);
		}
		return ears;
	}
}

// — Géométrie de la 1ʳᵉ couche (GLGizmoBrimEars::first_layer_slicer) —

/** Contour fermé (XY monde, mm), sans répétition du premier point. */
export type Contour = Vec2[];

function key(p: Vec2): string {
	return `${Math.round(p[0] * 1e4)},${Math.round(p[1] * 1e4)}`;
}

/**
 * Coupe d'un maillage par le plan Z = `z` (monde) : contours fermés XY.
 * `positions` / `indices` = maillage recentré, `transform` = instance.
 */
export function sliceMeshAt(
	positions: ArrayLike<number>,
	indices: ArrayLike<number>,
	transform: Transform,
	z: number
): Contour[] {
	const m = matrixOf(transform);
	const n = positions.length / 3;
	const world = new Float64Array(positions.length);
	const v = new Vector3();
	for (let i = 0; i < n; i++) {
		v.set(positions[3 * i], positions[3 * i + 1], positions[3 * i + 2]).applyMatrix4(m);
		world[3 * i] = v.x;
		world[3 * i + 1] = v.y;
		world[3 * i + 2] = v.z;
	}
	const segs: [Vec2, Vec2][] = [];
	const at = (i: number): Vec3 => [world[3 * i], world[3 * i + 1], world[3 * i + 2]];
	const tri = indices.length ? indices.length / 3 : n / 3;
	for (let t = 0; t < tri; t++) {
		const ids = indices.length
			? [indices[3 * t], indices[3 * t + 1], indices[3 * t + 2]]
			: [3 * t, 3 * t + 1, 3 * t + 2];
		const p = ids.map(at);
		const pts: Vec2[] = [];
		for (let e = 0; e < 3; e++) {
			const a = p[e];
			const b = p[(e + 1) % 3];
			const da = a[2] - z;
			const db = b[2] - z;
			if ((da < 0 && db >= 0) || (da >= 0 && db < 0)) {
				const s = da / (da - db);
				pts.push([a[0] + s * (b[0] - a[0]), a[1] + s * (b[1] - a[1])]);
			}
		}
		if (pts.length === 2 && key(pts[0]) !== key(pts[1])) segs.push([pts[0], pts[1]]);
	}
	// Chaînage des segments par extrémités communes.
	const byStart = new Map<string, number[]>();
	segs.forEach((s, i) => {
		for (const end of [s[0], s[1]]) {
			const k = key(end);
			const list = byStart.get(k);
			if (list) list.push(i);
			else byStart.set(k, [i]);
		}
	});
	const used = new Uint8Array(segs.length);
	const out: Contour[] = [];
	for (let i = 0; i < segs.length; i++) {
		if (used[i]) continue;
		used[i] = 1;
		const loop: Vec2[] = [segs[i][0], segs[i][1]];
		const first = key(segs[i][0]);
		let guard = 0;
		while (guard++ < segs.length) {
			const last = loop[loop.length - 1];
			const k = key(last);
			if (k === first) {
				loop.pop();
				break;
			}
			const next = (byStart.get(k) ?? []).find((j) => !used[j]);
			if (next === undefined) break;
			used[next] = 1;
			const s = segs[next];
			loop.push(key(s[0]) === k ? s[1] : s[0]);
		}
		if (loop.length >= 3) out.push(loop);
	}
	return out;
}

/**
 * 1ʳᵉ couche de l'objet posé (first_layer_slicer : coupe à 0,1 mm au-dessus
 * du point le plus bas de l'instance).
 */
export function firstLayerContours(
	positions: ArrayLike<number>,
	indices: ArrayLike<number>,
	transform: Transform,
	height = 0.1
): Contour[] {
	const m = matrixOf(transform);
	const v = new Vector3();
	let minZ = Infinity;
	for (let i = 0; i < positions.length; i += 3) {
		v.set(positions[i], positions[i + 1], positions[i + 2]).applyMatrix4(m);
		if (v.z < minZ) minZ = v.z;
	}
	if (!Number.isFinite(minZ)) return [];
	return sliceMeshAt(positions, indices, transform, minZ + height);
}

/** Aire signée (> 0 : sens trigonométrique). */
export function signedArea(c: Contour): number {
	let a = 0;
	for (let i = 0; i < c.length; i++) {
		const p = c[i];
		const q = c[(i + 1) % c.length];
		a += p[0] * q[1] - q[0] * p[1];
	}
	return a / 2;
}

function pointInContour(p: Vec2, c: Contour): boolean {
	let inside = false;
	for (let i = 0, j = c.length - 1; i < c.length; j = i++) {
		const [xi, yi] = c[i];
		const [xj, yj] = c[j];
		if (yi > p[1] !== yj > p[1] && p[0] < ((xj - xi) * (p[1] - yi)) / (yj - yi) + xi)
			inside = !inside;
	}
	return inside;
}

/** Point dans la 1ʳᵉ couche (pair-impair sur tous les contours). */
export function pointInLayer(p: Vec2, contours: readonly Contour[]): boolean {
	let inside = false;
	for (const c of contours) if (pointInContour(p, c)) inside = !inside;
	return inside;
}

/** Contours extérieurs (profondeur paire) / trous (impaire), orientés comme
 *  les ExPolygons d'Orca (extérieur trigonométrique, trous inversés). */
export function classifyContours(contours: readonly Contour[]): {
	outer: Contour[];
	holes: Contour[];
} {
	const outer: Contour[] = [];
	const holes: Contour[] = [];
	contours.forEach((c, i) => {
		let depth = 0;
		for (let j = 0; j < contours.length; j++)
			if (j !== i && pointInContour(c[0], contours[j])) depth++;
		const ccw = signedArea(c) > 0 ? c : [...c].reverse();
		if (depth % 2 === 0) outer.push(ccw);
		// Trou : stocké horaire puis inversé par polygons_reverse → trigonométrique.
		else holes.push(ccw);
	});
	return { outer, holes };
}

function distToSegmentSq(p: Vec2, a: Vec2, b: Vec2): number {
	const dx = b[0] - a[0];
	const dy = b[1] - a[1];
	const len = dx * dx + dy * dy;
	let t = len > 0 ? ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len : 0;
	t = Math.max(0, Math.min(1, t));
	const x = a[0] + t * dx - p[0];
	const y = a[1] + t * dy - p[1];
	return x * x + y * y;
}

/** MultiPoint::_douglas_peucker (tolérance en mm). */
export function douglasPeucker(pts: readonly Vec2[], tolerance: number): Vec2[] {
	if (pts.length === 0) return [];
	const tol2 = tolerance * tolerance;
	const out: Vec2[] = [pts[0]];
	let anchor = 0;
	let floater = pts.length - 1;
	if (anchor === floater) return out;
	const stack = [floater];
	for (;;) {
		let max = 0;
		let far = anchor;
		for (let i = anchor + 1; i < floater; i++) {
			const d = distToSegmentSq(pts[i], pts[anchor], pts[floater]);
			if (d > max) {
				max = d;
				far = i;
			}
		}
		if (max <= tol2) {
			out.push(pts[floater]);
			anchor = floater;
			stack.pop();
			if (!stack.length) break;
			floater = stack[stack.length - 1];
		} else {
			floater = far;
			stack.push(floater);
		}
	}
	return out;
}

/** filter_convex_concave_points_by_angle_threshold (Polygon.cpp). */
function convexConcave(poly: readonly Vec2[], angleThreshold: number, convex: boolean): Vec2[] {
	if (poly.length < 3) return [];
	const cosT = Math.cos(angleThreshold + Number.EPSILON);
	const out: Vec2[] = [];
	let p1 = poly[poly.length - 1];
	let v1: Vec2 = [p1[0] - poly[poly.length - 2][0], p1[1] - poly[poly.length - 2][1]];
	for (const p2 of poly) {
		const v2: Vec2 = [p2[0] - p1[0], p2[1] - p1[1]];
		const cross = v1[0] * v2[1] - v1[1] * v2[0];
		let keep = convex ? cross > 0 : cross < 0;
		if (keep && angleThreshold > 1e-9) {
			const n1 = Math.hypot(...v1) || 1;
			const n2 = Math.hypot(...v2) || 1;
			keep = (v1[0] * v2[0] + v1[1] * v2[1]) / (n1 * n2) < cosT;
		}
		if (keep) out.push(p1);
		v1 = v2;
		p1 = p2;
	}
	return out;
}

/** GLGizmoBrimEars::generate_points pour un contour orienté. */
export function generatePoints(
	contour: Contour,
	detectionRadius: number,
	maxAngle: number,
	isOuter: boolean
): Vec2[] {
	const threshold = ((180 - maxAngle) * Math.PI) / 180;
	let poly: Vec2[] = [...contour];
	if (detectionRadius > 0) {
		let pts = douglasPeucker([...poly, poly[0]], detectionRadius);
		if (pts.length > 4) pts = pts.slice(0, -1);
		poly = pts;
	}
	return convexConcave(poly, threshold, isOuter);
}

/** Auto-génération (GLGizmoBrimEars::auto_generate) : points XY monde. */
export function autoGeneratePoints(
	contours: readonly Contour[],
	detectionRadius: number,
	maxAngle: number
): Vec2[] {
	const { outer, holes } = classifyContours(contours);
	const out: Vec2[] = [];
	for (const c of outer) out.push(...generatePoints(c, detectionRadius, maxAngle, true));
	for (const h of holes) out.push(...generatePoints(h, detectionRadius, maxAngle, false));
	return out;
}

function discTouchesLayer(c: Vec2, r: number, contours: readonly Contour[]): boolean {
	if (pointInLayer(c, contours)) return true;
	const r2 = r * r;
	for (const poly of contours)
		for (let i = 0; i < poly.length; i++)
			if (distToSegmentSq(c, poly[i], poly[(i + 1) % poly.length]) < r2) return true;
	return false;
}

/**
 * Oreilles invalides (GLGizmoBrimEars::find_single) : disque qui ne recouvre
 * ni la 1ʳᵉ couche ni, de proche en proche, une oreille valide. Indices.
 */
export function invalidEars(
	ears: readonly { center: Vec2; r: number }[],
	contours: readonly Contour[]
): number[] {
	const pending = new Set(ears.map((_, i) => i));
	const valid: number[] = [];
	let changed = true;
	while (changed && pending.size) {
		changed = false;
		for (const i of [...pending]) {
			const e = ears[i];
			const ok =
				discTouchesLayer(e.center, e.r, contours) ||
				valid.some((j) => {
					const o = ears[j];
					return Math.hypot(o.center[0] - e.center[0], o.center[1] - e.center[1]) < o.r + e.r;
				});
			if (ok) {
				pending.delete(i);
				valid.push(i);
				changed = true;
			}
		}
	}
	return [...pending].sort((a, b) => a - b);
}

/** Borne du curseur « Rayon de détection » (get_detection_radius_max) :
 *  tolérance de Douglas-Peucker au-delà de laquelle le contour extérieur ne
 *  se simplifie plus, 100 mm au plus. */
export function detectionRadiusMax(contours: readonly Contour[]): number {
	let maxDist = 0;
	for (const c of classifyContours(contours).outer) {
		const pts = [...c, c[0]];
		let tolerance = 0;
		let minNum = douglasPeucker(pts, 0).length;
		let repeat = 0;
		for (let guard = 1; ; guard++) {
			tolerance += 10;
			const num = douglasPeucker(pts, tolerance).length;
			if (num === minNum) {
				repeat++;
				if (repeat > 1) break;
			}
			minNum = num;
			if (guard > 100) break;
		}
		for (let guard = 1; ; guard++) {
			tolerance -= 1;
			const num = douglasPeucker(pts, tolerance).length;
			if (num <= minNum) minNum = num;
			else break;
			if (guard > 100) break;
		}
		tolerance += 1;
		if (tolerance > maxDist) maxDist = tolerance;
	}
	return maxDist > 100 || maxDist <= 0 ? 100 : maxDist;
}
