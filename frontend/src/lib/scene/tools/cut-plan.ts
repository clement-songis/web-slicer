// Découpe complète (phase 28, lot 26) : géométrie pure de l'outil Coupe —
// plan incliné (rotation Orca), repère du plan, section de l'objet par le plan
// (face de coupe où l'on pose les connecteurs), connecteurs en coordonnées du
// plan et corps de POST /api/models/{id}/cut. Aucun état Svelte : testable.
//
// Repères :
// - repère « recentré » : celui du maillage de scène (SceneObject.mesh) ;
// - repère « C » (monde centré sur l'objet) : R·(S ⊙ v) = monde − position.
//   C'est le repère de instance_matrix côté serveur ; pour un objet sans
//   rotation ni échelle, C = recentré (l'ancien chemin cut_mesh est gardé).
// Le plan d'Orca (GLGizmoCut3D) est orienté dans le monde : rotation
// rotation_transform(r) = Rz·Ry·Rx appliquée à +Z, centre au centre de la
// boîte de l'instance, déplacé le long de sa normale.
import type { CutConnectorShape } from '../../../generated/api/CutConnectorShape';
import type { CutConnectorStyle } from '../../../generated/api/CutConnectorStyle';
import type { CutConnectorType } from '../../../generated/api/CutConnectorType';
import type { CutRequest } from '../../../generated/api/CutRequest';
import { rotateVectorByEulerDeg } from '../transform';
import { tr } from '#lib/i18n';

export type Vec3 = [number, number, number];
export type Axis = 'x' | 'y' | 'z';

const DEG = Math.PI / 180;

/** Transformation d'objet de scène (Euler XYZ three.js en degrés). */
export interface ObjectXf {
	position: Vec3;
	rotation: Vec3;
	scale: Vec3;
}

/** Matrice 3×3 (lignes) de Geometry::rotation_transform d'Orca : Rz·Ry·Rx. */
export function orcaRotation(rot: Vec3): number[][] {
	const [x, y, z] = [rot[0] * DEG, rot[1] * DEG, rot[2] * DEG];
	const [cx, sx, cy, sy, cz, sz] = [
		Math.cos(x),
		Math.sin(x),
		Math.cos(y),
		Math.sin(y),
		Math.cos(z),
		Math.sin(z)
	];
	return [
		[cz * cy, cz * sy * sx - sz * cx, cz * sy * cx + sz * sx],
		[sz * cy, sz * sy * sx + cz * cx, sz * sy * cx - cz * sx],
		[-sy, cy * sx, cy * cx]
	];
}

const clean = (v: number) => (Math.abs(v) < 1e-12 ? 0 : Math.round(v * 1e9) / 1e9);

/** Repère du plan : axes X, Y du plan et normale (colonnes de la rotation). */
export interface PlaneBasis {
	x: Vec3;
	y: Vec3;
	n: Vec3;
}

export function planeBasis(rot: Vec3): PlaneBasis {
	const m = orcaRotation(rot);
	const col = (j: number): Vec3 => [clean(m[0][j]), clean(m[1][j]), clean(m[2][j])];
	return { x: col(0), y: col(1), n: col(2) };
}

/** Quaternion three.js (x, y, z, w) de la rotation du plan (dessin, gizmo). */
export function planeQuaternion(rot: Vec3): [number, number, number, number] {
	const [x, y, z] = [rot[0] * DEG * 0.5, rot[1] * DEG * 0.5, rot[2] * DEG * 0.5];
	const [c1, s1, c2, s2, c3, s3] = [
		Math.cos(x),
		Math.sin(x),
		Math.cos(y),
		Math.sin(y),
		Math.cos(z),
		Math.sin(z)
	];
	// Ordre 'ZYX' de three.js (Quaternion.setFromEuler) = Rz·Ry·Rx.
	return [
		s1 * c2 * c3 - c1 * s2 * s3,
		c1 * s2 * c3 + s1 * c2 * s3,
		c1 * c2 * s3 - s1 * s2 * c3,
		c1 * c2 * c3 + s1 * s2 * s3
	];
}

/** Rotations des trois plans « droits » (normale = axe). */
export const AXIS_ROTATION: Record<Axis, Vec3> = {
	x: [0, 90, 0],
	y: [-90, 0, 0],
	z: [0, 0, 0]
};

/** Axe dont la normale du plan est (quasi) l'axe, sinon null (plan incliné). */
export function axisOfRotation(rot: Vec3): Axis | null {
	const n = planeBasis(rot).n;
	const axes: [Axis, number][] = [
		['x', 0],
		['y', 1],
		['z', 2]
	];
	for (const [a, i] of axes) if (Math.abs(n[i]) > 1 - 1e-9) return a;
	return null;
}

/** Axe dominant de la normale (libellé « Hauteur » si Z). */
export function dominantAxis(n: Vec3): 0 | 1 | 2 {
	const a = n.map(Math.abs);
	return a[2] >= a[0] && a[2] >= a[1] ? 2 : a[0] >= a[1] ? 0 : 1;
}

/** Normalise un angle en degrés dans ]−180, 180]. */
export function wrapDeg(v: number): number {
	let r = ((v % 360) + 360) % 360;
	if (r > 180) r -= 360;
	return Math.round(r * 1000) / 1000;
}

/** Objet sans rotation ni échelle : le repère C est le repère recentré. */
export function isPlainXf(xf: ObjectXf): boolean {
	return (
		xf.rotation.every((r) => Math.abs(r) < 1e-9) && xf.scale.every((s) => Math.abs(s - 1) < 1e-9)
	);
}

/** Sommets du maillage dans le repère C (R·(S ⊙ v)). */
export function centeredWorld(positions: ArrayLike<number>, xf: ObjectXf): Float32Array {
	const out = new Float32Array(positions.length);
	const plain = isPlainXf(xf);
	for (let i = 0; i < positions.length; i += 3) {
		const v: Vec3 = [
			positions[i] * xf.scale[0],
			positions[i + 1] * xf.scale[1],
			positions[i + 2] * xf.scale[2]
		];
		const w = plain ? v : rotateVectorByEulerDeg(xf.rotation, v);
		out[i] = w[0];
		out[i + 1] = w[1];
		out[i + 2] = w[2];
	}
	return out;
}

export function boundsOf(p: ArrayLike<number>): { min: Vec3; max: Vec3 } {
	const min: Vec3 = [Infinity, Infinity, Infinity];
	const max: Vec3 = [-Infinity, -Infinity, -Infinity];
	for (let i = 0; i < p.length; i += 3)
		for (let a = 0; a < 3; a++) {
			if (p[i + a] < min[a]) min[a] = p[i + a];
			if (p[i + a] > max[a]) max[a] = p[i + a];
		}
	if (!Number.isFinite(min[0])) return { min: [0, 0, 0], max: [0, 0, 0] };
	return { min, max };
}

export const boxCenter = (b: { min: Vec3; max: Vec3 }): Vec3 => [
	(b.min[0] + b.max[0]) / 2,
	(b.min[1] + b.max[1]) / 2,
	(b.min[2] + b.max[2]) / 2
];

const dot = (a: ArrayLike<number>, b: ArrayLike<number>, ao = 0) =>
	a[ao] * b[0] + a[ao + 1] * b[1] + a[ao + 2] * b[2];

/** Étendue de l'objet le long de la normale, depuis center (mm). */
export function offsetRange(p: ArrayLike<number>, center: Vec3, n: Vec3): [number, number] {
	let lo = Infinity;
	let hi = -Infinity;
	const c = dot(center, n);
	for (let i = 0; i < p.length; i += 3) {
		const d = dot(p, n, i) - c;
		if (d < lo) lo = d;
		if (d > hi) hi = d;
	}
	return Number.isFinite(lo) ? [lo, hi] : [0, 0];
}

/** Point du plan (repère C) : centre de la boîte + décalage le long de la normale. */
export function planePoint(center: Vec3, n: Vec3, offset: number): Vec3 {
	return [center[0] + n[0] * offset, center[1] + n[1] * offset, center[2] + n[2] * offset];
}

/** Coordonnées (u, v) du plan → point du repère C. */
export function uvToPoint(origin: Vec3, b: PlaneBasis, uv: [number, number]): Vec3 {
	return [0, 1, 2].map((i) => origin[i] + b.x[i] * uv[0] + b.y[i] * uv[1]) as Vec3;
}

/** Point du repère C → (u, v) de sa projection sur le plan. */
export function pointToUv(origin: Vec3, b: PlaneBasis, p: Vec3): [number, number] {
	const d: Vec3 = [p[0] - origin[0], p[1] - origin[1], p[2] - origin[2]];
	return [dot(d, b.x), dot(d, b.y)];
}

/**
 * Section de l'objet par le plan (face de coupe) : segments 2D [u0, v0, u1,
 * v1, …] dans le repère du plan, et leur emprise. p = sommets dans le
 * repère C, indices = triangles.
 */
export function sectionSegments(
	p: ArrayLike<number>,
	indices: ArrayLike<number>,
	origin: Vec3,
	b: PlaneBasis
): { segments: number[]; min: [number, number]; max: [number, number] } {
	const segments: number[] = [];
	const min: [number, number] = [Infinity, Infinity];
	const max: [number, number] = [-Infinity, -Infinity];
	const c = dot(origin, b.n);
	const ds = [0, 0, 0];
	const tri = [0, 0, 0];
	const q: Vec3 = [0, 0, 0];
	const push = (i: number, j: number) => {
		const t = ds[i] / (ds[i] - ds[j]);
		for (let a = 0; a < 3; a++) {
			const pi = p[tri[i] * 3 + a];
			q[a] = pi + (p[tri[j] * 3 + a] - pi) * t - origin[a];
		}
		const u = dot(q, b.x);
		const v = dot(q, b.y);
		segments.push(u, v);
		if (u < min[0]) min[0] = u;
		if (v < min[1]) min[1] = v;
		if (u > max[0]) max[0] = u;
		if (v > max[1]) max[1] = v;
	};
	for (let t = 0; t + 2 < indices.length; t += 3) {
		let above = 0;
		for (let k = 0; k < 3; k++) {
			tri[k] = indices[t + k];
			ds[k] = dot(p, b.n, tri[k] * 3) - c;
			if (ds[k] > 0) above++;
		}
		if (above === 0 || above === 3) continue;
		// Sommet isolé du côté minoritaire : ses deux arêtes traversent le plan.
		let lone = 0;
		for (let k = 0; k < 3; k++) if (ds[k] > 0 === (above === 1)) lone = k;
		push(lone, (lone + 1) % 3);
		push(lone, (lone + 2) % 3);
	}
	if (!Number.isFinite(min[0])) return { segments, min: [0, 0], max: [0, 0] };
	return { segments, min, max };
}

/** Le point (u, v) est-il à l'intérieur de la section (parité de croisements) ? */
export function insideSection(segments: readonly number[], u: number, v: number): boolean {
	let inside = false;
	for (let i = 0; i + 3 < segments.length; i += 4) {
		const [u0, v0, u1, v1] = [segments[i], segments[i + 1], segments[i + 2], segments[i + 3]];
		if (v0 > v !== v1 > v && u < u0 + ((v - v0) * (u1 - u0)) / (v1 - v0)) inside = !inside;
	}
	return inside;
}

/** Dimensions du plan dessiné : emprise de la boîte dans le repère du plan + marge. */
export function planeSize(
	box: { min: Vec3; max: Vec3 },
	b: PlaneBasis,
	margin = 4
): [number, number] {
	const half = [0, 1, 2].map((i) => (box.max[i] - box.min[i]) / 2);
	const ext = (axis: Vec3) =>
		2 * (Math.abs(axis[0]) * half[0] + Math.abs(axis[1]) * half[1] + Math.abs(axis[2]) * half[2]);
	return [ext(b.x) + 2 * margin, ext(b.y) + 2 * margin];
}

// — Connecteurs (GLGizmoCut3D : CutConnector + attributs) —

export interface ConnectorDraft {
	id: string;
	/** Position dans le repère du plan (mm) : suit le plan quand il bouge. */
	u: number;
	v: number;
	type: CutConnectorType;
	style: CutConnectorStyle;
	shape: CutConnectorShape;
	/** Diamètre (mm). */
	size: number;
	/** Profondeur (mm). */
	depth: number;
	sizeTolerance: number;
	depthTolerance: number;
	/** Rotation autour de l'axe du connecteur (0–180°). */
	rotation: number;
}

export type ConnectorParams = Omit<ConnectorDraft, 'id' | 'u' | 'v'>;

/** Défauts du gizmo d'Orca (m_connector_size 2,5, profondeur 3, tolérances 0 / 0,1). */
export function defaultConnectorParams(): ConnectorParams {
	return {
		type: 'plug',
		style: 'prism',
		shape: 'circle',
		size: 2.5,
		depth: 3,
		sizeTolerance: 0,
		depthTolerance: 0.1,
		rotation: 0
	};
}

/**
 * Règles du gizmo (render_connectors_input_window) : goujon → prisme ; clip →
 * cercle et profondeur ≥ taille ; valeurs bornées comme la validation serveur.
 */
export function normalizeConnector<T extends ConnectorParams>(c: T): T {
	const out = { ...c };
	if (out.type === 'dowel') out.style = 'prism';
	if (out.type === 'snap') {
		out.shape = 'circle';
		out.style = 'prism';
		if (out.depth < out.size) out.depth = out.size;
	}
	const fin = (v: number, d: number) => (Number.isFinite(v) ? v : d);
	out.size = Math.min(200, Math.max(0.1, fin(out.size, 2.5)));
	out.depth = Math.min(200, Math.max(0.1, fin(out.depth, 3)));
	out.sizeTolerance = Math.min(10, Math.max(0, fin(out.sizeTolerance, 0)));
	out.depthTolerance = Math.min(10, Math.max(0, fin(out.depthTolerance, 0.1)));
	out.rotation = Math.min(180, Math.max(0, fin(out.rotation, 0)));
	return out;
}

/** Connecteur le plus proche de (u, v) à moins de son rayon (+1 mm), sinon null. */
export function connectorAt(
	connectors: readonly ConnectorDraft[],
	u: number,
	v: number
): ConnectorDraft | null {
	let best: ConnectorDraft | null = null;
	let bestD = Infinity;
	for (const c of connectors) {
		const d = Math.hypot(c.u - u, c.v - v);
		if (d <= c.size / 2 + 1 && d < bestD) {
			best = c;
			bestD = d;
		}
	}
	return best;
}

// — Queue d'aronde (CutMode::cutTongueAndGroove) —

export interface GrooveDraft {
	depth: number;
	width: number;
	flapsAngle: number;
	angle: number;
	depthTolerance: number;
	widthTolerance: number;
	count: number;
	gap: number;
}

/**
 * Défauts du gizmo : profondeur = max(1, ½ · taille moyenne des poignées)
 * (get_grabber_mean_size = (sx + sy + sz) / 30), largeur 4 × profondeur,
 * rabats 60°, angle 0°, tolérances 0,1 mm, 1 queue, écart 10 mm.
 */
export function defaultGroove(box: { min: Vec3; max: Vec3 }): GrooveDraft {
	const s = [0, 1, 2].reduce((acc, i) => acc + (box.max[i] - box.min[i]), 0);
	const depth = Math.round(Math.max(1, (0.5 * s) / 30) * 100) / 100;
	return {
		depth,
		width: Math.round(4 * depth * 100) / 100,
		flapsAngle: 60,
		angle: 0,
		depthTolerance: 0.1,
		widthTolerance: 0.1,
		count: 1,
		gap: 10
	};
}

// — Corps de la requête —

export type CutMode = 'planar' | 'groove';

export interface PartAction {
	keep: boolean;
	placeOnCut: boolean;
	flip: boolean;
}

export interface CutDraft {
	mode: CutMode;
	rotation: Vec3;
	/** Décalage le long de la normale depuis le centre de la boîte (repère C). */
	offset: number;
	upper: PartAction;
	lower: PartAction;
	asParts: boolean;
	connectors: ConnectorDraft[];
	groove: GrooveDraft | null;
}

/**
 * Contraintes d'Orca entre options : connecteurs → deux moitiés gardées, pas
 * de pièces ; « pièces » → ni placer ni retourner ; queue d'aronde → ni pièces
 * ni connecteurs ; « placer sur la coupe » et « retourner » exclusifs.
 */
export function effectiveDraft(d: CutDraft): CutDraft {
	const out: CutDraft = {
		...d,
		upper: { ...d.upper },
		lower: { ...d.lower },
		connectors: d.mode === 'groove' ? [] : d.connectors
	};
	if (out.mode === 'groove') out.asParts = false;
	if (out.connectors.length > 0) {
		out.asParts = false;
		out.upper.keep = true;
		out.lower.keep = true;
	}
	if (out.asParts) {
		out.upper = { keep: true, placeOnCut: false, flip: false };
		out.lower = { keep: true, placeOnCut: false, flip: false };
	}
	for (const part of [out.upper, out.lower]) {
		if (!part.keep) part.placeOnCut = part.flip = false;
		if (part.placeOnCut && part.flip) part.flip = false;
	}
	return out;
}

/** Raison d'empêcher la coupe, ou null (au moins une moitié, plan dans l'objet). */
export function cutBlocker(d: CutDraft, range: [number, number]): string | null {
	const e = effectiveDraft(d);
	if (!e.upper.keep && !e.lower.keep) return tr('Keep at least one of the two parts.');
	if (!(d.offset > range[0] + 1e-6 && d.offset < range[1] - 1e-6))
		return tr('The plane does not cross the object.');
	if (e.mode === 'groove' && e.groove && !(e.groove.depth > 0 && e.groove.width > 0))
		return tr('Dovetail depth and width must be positive.');
	return null;
}

const r6 = (v: number) => {
	const r = Math.round(v * 1e6) / 1e6;
	return r === 0 ? 0 : r;
};
const vec6 = (v: Vec3): Vec3 => [r6(v[0]), r6(v[1]), r6(v[2])];

/**
 * Corps de POST /api/models/{id}/cut. center = centre de la boîte de l'objet
 * dans le repère C ; xf = transformation de l'objet (matrice d'instance
 * envoyée seulement si rotation/échelle). Les options d'Orca inutilisées ne
 * sont pas envoyées : une coupe plane droite garde le chemin historique.
 */
export function buildCutRequest(
	draft: CutDraft,
	center: Vec3,
	xf: ObjectXf,
	instanceMatrix: () => number[]
): CutRequest {
	const d = effectiveDraft(draft);
	const b = planeBasis(d.rotation);
	const origin = planePoint(center, b.n, d.offset);
	const body: CutRequest = {
		plane: { point: vec6(origin), normal: vec6(b.n) },
		keep_upper: d.upper.keep,
		keep_lower: d.lower.keep
	};
	if (d.rotation.some((r) => Math.abs(r) > 1e-9)) body.rotation_deg = [...d.rotation];
	if (!isPlainXf(xf)) body.instance_matrix = instanceMatrix();
	if (d.asParts) body.keep_as_parts = true;
	if (d.upper.placeOnCut) body.place_on_cut_upper = true;
	if (d.lower.placeOnCut) body.place_on_cut_lower = true;
	if (d.upper.flip) body.flip_upper = true;
	if (d.lower.flip) body.flip_lower = true;
	if (d.connectors.length > 0)
		body.connectors = d.connectors.map((c) => {
			const n = normalizeConnector(c);
			return {
				pos: vec6(uvToPoint(origin, b, [c.u, c.v])),
				type: n.type,
				style: n.style,
				shape: n.shape,
				size_mm: n.size,
				depth_mm: n.depth,
				size_tolerance_mm: n.sizeTolerance,
				depth_tolerance_mm: n.depthTolerance,
				rotation_deg: n.rotation
			};
		});
	if (d.mode === 'groove' && d.groove)
		body.groove = {
			depth_mm: d.groove.depth,
			width_mm: d.groove.width,
			flaps_angle_deg: d.groove.flapsAngle,
			angle_deg: d.groove.angle,
			depth_tolerance_mm: d.groove.depthTolerance,
			width_tolerance_mm: d.groove.widthTolerance,
			count: Math.max(1, Math.round(d.groove.count)),
			gap_mm: d.groove.gap
		};
	return body;
}

/** Libellé lisible d'une pièce produite (label serveur). */
export function cutPartLabel(label: string): string {
	if (label === 'upper') return tr('upper part');
	if (label === 'lower') return tr('lower part');
	if (label === 'parts') return tr('parts||cut');
	const m = /^dowel_(\d+)$/.exec(label);
	if (m) return tr('dowel {n}', { n: m[1] });
	return label;
}
