// Gizmo « Poser sur une face » (Orca `GLGizmoFlatten::update_planes`) : faces
// candidates = plans de l'enveloppe convexe de l'objet (échelle et rotation de
// l'instance appliquées), facettes coplanaires regroupées, plans trop petits
// (< 5 mm²), trop fins (côté < 1 mm) ou dégénérés écartés, tri par aire
// décroissante (254 au plus). Pur → testable sous vitest.
import { ConvexHull } from 'three/examples/jsm/math/ConvexHull.js';
import { Euler } from 'three/src/math/Euler.js';
import { Matrix4 } from 'three/src/math/Matrix4.js';
import { Quaternion } from 'three/src/math/Quaternion.js';
import { Vector3 } from 'three/src/math/Vector3.js';
import type { SceneMesh } from '../mesh';

export type Vec3 = [number, number, number];

/** Face candidate, en repère « monde sans translation » de l'objet. */
export interface FlattenPlane {
	/** Normale sortante unitaire. */
	normal: Vec3;
	/** Contour convexe du plan (ordre du contour 2D). */
	polygon: Vec3[];
	/** Aire du contour (mm²). */
	area: number;
	/** Centre du contour. */
	centroid: Vec3;
}

const MINIMAL_AREA = 5; // mm²
const MINIMAL_SIDE = 1; // mm
const MINIMAL_ANGLE = 1; // degré
const MAX_PLANES = 254;
const DEG = Math.PI / 180;

function instanceMatrix(rotation: Vec3, scale: Vec3): Matrix4 {
	const q = new Quaternion().setFromEuler(
		new Euler(rotation[0] * DEG, rotation[1] * DEG, rotation[2] * DEG, 'XYZ')
	);
	return new Matrix4().compose(new Vector3(), q, new Vector3(...scale));
}

/** Enveloppe convexe 2D (chaîne monotone d'Andrew), sens trigonométrique. */
function hull2d(points: [number, number, number][]): [number, number, number][] {
	const pts = [...points].sort((a, b) => a[0] - b[0] || a[1] - b[1]);
	if (pts.length < 3) return pts;
	const cross = (o: number[], a: number[], b: number[]) =>
		(a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
	const lower: [number, number, number][] = [];
	for (const p of pts) {
		while (lower.length >= 2 && cross(lower[lower.length - 2], lower[lower.length - 1], p) <= 1e-12)
			lower.pop();
		lower.push(p);
	}
	const upper: [number, number, number][] = [];
	for (const p of [...pts].reverse()) {
		while (upper.length >= 2 && cross(upper[upper.length - 2], upper[upper.length - 1], p) <= 1e-12)
			upper.pop();
		upper.push(p);
	}
	return [...lower.slice(0, -1), ...upper.slice(0, -1)];
}

/** Faces candidates de l'objet (maillage local, rotation en degrés, échelle). */
export function flattenPlanes(
	mesh: SceneMesh,
	rotation: Vec3 = [0, 0, 0],
	scale: Vec3 = [1, 1, 1]
): FlattenPlane[] {
	const m = instanceMatrix(rotation, scale);
	const points: Vector3[] = [];
	for (let i = 0; i < mesh.positions.length; i += 3)
		points.push(
			new Vector3(mesh.positions[i], mesh.positions[i + 1], mesh.positions[i + 2]).applyMatrix4(m)
		);
	if (points.length < 4) return [];
	const hull = new ConvexHull().setFromPoints(points);
	// Regroupement des facettes de même normale (|Δ| < 0,001 par composante).
	const groups: { normal: Vector3; vertices: Vector3[] }[] = [];
	for (const face of hull.faces) {
		const n = face.normal;
		let group = groups.find(
			(g) =>
				Math.abs(g.normal.x - n.x) < 1e-3 &&
				Math.abs(g.normal.y - n.y) < 1e-3 &&
				Math.abs(g.normal.z - n.z) < 1e-3
		);
		if (!group) {
			group = { normal: n.clone(), vertices: [] };
			groups.push(group);
		}
		let edge = face.edge;
		do {
			group.vertices.push(edge.head().point.clone());
			edge = edge.next;
		} while (edge !== face.edge);
	}
	const angleThreshold = Math.cos(MINIMAL_ANGLE * DEG);
	const planes: FlattenPlane[] = [];
	for (const { normal, vertices } of groups) {
		if (
			vertices.length === 3 &&
			(vertices[0].distanceTo(vertices[1]) < MINIMAL_SIDE ||
				vertices[0].distanceTo(vertices[2]) < MINIMAL_SIDE ||
				vertices[1].distanceTo(vertices[2]) < MINIMAL_SIDE)
		)
			continue;
		// Projection dans le plan (normale → +Z), contour convexe 2D, aire (lacet).
		const q = new Quaternion().setFromUnitVectors(normal, new Vector3(0, 0, 1));
		const flat = vertices.map((v) => v.clone().applyQuaternion(q).toArray());
		const polygon = hull2d(flat);
		if (polygon.length < 3) continue;
		let area = 0;
		for (let i = 0; i < polygon.length; i++) {
			const a = polygon[i];
			const b = polygon[(i + 1) % polygon.length];
			area += a[0] * b[1] - b[0] * a[1];
		}
		area = Math.abs(area) / 2;
		if (area < MINIMAL_AREA) continue;
		let degenerate = false;
		for (let i = 0; i < polygon.length; i++) {
			const prev = new Vector3(...polygon[(i + polygon.length - 1) % polygon.length]);
			const curr = new Vector3(...polygon[i]);
			const next = new Vector3(...polygon[(i + 1) % polygon.length]);
			if (prev.sub(curr).normalize().dot(next.sub(curr).normalize()) > angleThreshold) {
				degenerate = true;
				break;
			}
		}
		if (degenerate) continue;
		const back = q.clone().invert();
		const world = polygon.map((p) => new Vector3(...p).applyQuaternion(back));
		const centroid = world.reduce((acc, v) => acc.add(v), new Vector3()).divideScalar(world.length);
		planes.push({
			normal: normal.toArray() as Vec3,
			polygon: world.map((v) => v.toArray() as Vec3),
			area,
			centroid: centroid.toArray() as Vec3
		});
	}
	planes.sort((a, b) => b.area - a.area);
	return planes.slice(0, MAX_PLANES);
}

/**
 * Rotation (Euler XYZ, degrés) qui pose la face de normale `normal` (exprimée
 * avec la rotation `rotation` déjà appliquée) à plat sur le plateau
 * (Orca `Selection::flattening_rotate` : normale → −Z).
 */
export function flattenRotation(rotation: Vec3, normal: Vec3): Vec3 {
	const current = new Quaternion().setFromEuler(
		new Euler(rotation[0] * DEG, rotation[1] * DEG, rotation[2] * DEG, 'XYZ')
	);
	const turn = new Quaternion().setFromUnitVectors(
		new Vector3(...normal).normalize(),
		new Vector3(0, 0, -1)
	);
	const e = new Euler().setFromQuaternion(turn.multiply(current), 'XYZ');
	const clean = (r: number) => {
		const d = r / DEG;
		const rounded = Math.round(d * 1e6) / 1e6;
		return Object.is(rounded, -0) ? 0 : rounded;
	};
	return [clean(e.x), clean(e.y), clean(e.z)];
}
