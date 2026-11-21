// Alerte « objets superposés » (phase 38) : empreinte au sol réelle des objets,
// rotation et échelle comprises — enveloppe convexe XY du maillage placé —
// au lieu de boîtes XY sans rotation (faux positif après un arrangement
// d'objets tournés, phase 36). Orca n'a pas cette alerte ; son arrangeur
// raisonne aussi sur des enveloppes convexes. Pur → testable.
import { ConvexHull } from 'three/examples/jsm/math/ConvexHull.js';
import { Euler } from 'three/src/math/Euler.js';
import { Matrix4 } from 'three/src/math/Matrix4.js';
import { Quaternion } from 'three/src/math/Quaternion.js';
import { Vector3 } from 'three/src/math/Vector3.js';
import type { SceneMesh, SceneObject } from './mesh';

export type Point2 = [number, number];

const DEG = Math.PI / 180;
/** Points de l'enveloppe 3D du maillage (indépendants du placement), en cache. */
const hullCache = new WeakMap<SceneMesh, Float32Array>();
/** Au-delà, l'enveloppe 3D est calculée sur un échantillon régulier de sommets. */
const MAX_HULL_INPUT = 200_000;

export function hullPoints(mesh: SceneMesh): Float32Array {
	const cached = hullCache.get(mesh);
	if (cached) return cached;
	const pos = mesh.positions;
	const n = pos.length / 3;
	let out: Float32Array = pos;
	if (n > 64) {
		const stride = Math.max(1, Math.ceil(n / MAX_HULL_INPUT));
		const pts: Vector3[] = [];
		for (let i = 0; i < n; i += stride)
			pts.push(new Vector3(pos[i * 3], pos[i * 3 + 1], pos[i * 3 + 2]));
		try {
			const hull = new ConvexHull().setFromPoints(pts);
			const seen = new Set<Vector3>();
			for (const face of hull.faces) {
				let edge = face.edge;
				do {
					seen.add(edge.head().point);
					edge = edge.next;
				} while (edge !== face.edge);
			}
			if (seen.size >= 3) {
				out = new Float32Array(seen.size * 3);
				let k = 0;
				for (const p of seen) {
					out[k++] = p.x;
					out[k++] = p.y;
					out[k++] = p.z;
				}
			}
		} catch {
			// Maillage dégénéré (plan) : tous les sommets échantillonnés.
			out = new Float32Array(pts.flatMap((p) => [p.x, p.y, p.z]));
		}
	}
	hullCache.set(mesh, out);
	return out;
}

/** Enveloppe convexe 2D (chaîne monotone, sens trigonométrique). */
export function convexHull2D(points: readonly Point2[]): Point2[] {
	const pts = [...points].sort((a, b) => a[0] - b[0] || a[1] - b[1]);
	if (pts.length < 3) return pts;
	const cross = (o: Point2, a: Point2, b: Point2) =>
		(a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
	const lower: Point2[] = [];
	for (const p of pts) {
		while (lower.length >= 2 && cross(lower[lower.length - 2], lower[lower.length - 1], p) <= 0)
			lower.pop();
		lower.push(p);
	}
	const upper: Point2[] = [];
	for (let i = pts.length - 1; i >= 0; i--) {
		const p = pts[i];
		while (upper.length >= 2 && cross(upper[upper.length - 2], upper[upper.length - 1], p) <= 0)
			upper.pop();
		upper.push(p);
	}
	upper.pop();
	lower.pop();
	return lower.concat(upper);
}

/** Empreinte au sol (polygone convexe XY) d'un objet placé. */
export function footprintPolygon(o: SceneObject, fallback: Point2): Point2[] {
	const r = o.rotation ?? [0, 0, 0];
	const s = o.scale ?? [1, 1, 1];
	const m = new Matrix4().compose(
		new Vector3(o.position?.[0] ?? fallback[0], o.position?.[1] ?? fallback[1], 0),
		new Quaternion().setFromEuler(new Euler(r[0] * DEG, r[1] * DEG, r[2] * DEG)),
		new Vector3(s[0], s[1], s[2])
	);
	const pts = hullPoints(o.mesh);
	const v = new Vector3();
	const flat: Point2[] = [];
	for (let i = 0; i < pts.length; i += 3) {
		v.set(pts[i], pts[i + 1], pts[i + 2]).applyMatrix4(m);
		flat.push([v.x, v.y]);
	}
	return convexHull2D(flat);
}

/**
 * Pénétration minimale de deux polygones convexes (théorème des axes
 * séparateurs) : ≤ 0 → disjoints ; sinon profondeur de recouvrement (mm).
 */
export function overlapDepth(a: readonly Point2[], b: readonly Point2[]): number {
	if (a.length < 3 || b.length < 3) return 0;
	let min = Infinity;
	for (const poly of [a, b]) {
		for (let i = 0; i < poly.length; i++) {
			const p = poly[i];
			const q = poly[(i + 1) % poly.length];
			const len = Math.hypot(q[0] - p[0], q[1] - p[1]);
			if (len < 1e-9) continue;
			const nx = -(q[1] - p[1]) / len;
			const ny = (q[0] - p[0]) / len;
			let aMin = Infinity;
			let aMax = -Infinity;
			for (const [x, y] of a) {
				const d = x * nx + y * ny;
				aMin = Math.min(aMin, d);
				aMax = Math.max(aMax, d);
			}
			let bMin = Infinity;
			let bMax = -Infinity;
			for (const [x, y] of b) {
				const d = x * nx + y * ny;
				bMin = Math.min(bMin, d);
				bMax = Math.max(bMax, d);
			}
			const depth = Math.min(aMax, bMax) - Math.max(aMin, bMin);
			if (depth <= 0) return depth;
			min = Math.min(min, depth);
		}
	}
	return min === Infinity ? 0 : min;
}

/**
 * Nombre d'objets dont l'empreinte en recouvre une autre de plus de
 * `minDepth` mm (0,5 mm, comme l'ancienne alerte).
 */
export function overlappingCount(
	objects: readonly SceneObject[],
	fallback: Point2,
	minDepth = 0.5
): number {
	const polys = objects.map((o) => footprintPolygon(o, fallback));
	const hit = new Set<number>();
	for (let i = 0; i < polys.length; i++)
		for (let j = i + 1; j < polys.length; j++)
			if (overlapDepth(polys[i], polys[j]) > minDepth) {
				hit.add(i);
				hit.add(j);
			}
	return hit.size;
}
