// Outil de coupe (T055) : plan positionnable qui sépare un maillage en deux
// moitiés, et grille de connecteurs (tenons) le long de la coupe. Géométrie
// pure (clipping triangle↔plan par Sutherland–Hodgman) → testable sous bun.
import { meshFromTriangleSoup } from '../loaders';
import type { SceneMesh } from '../mesh';

type Vec3 = [number, number, number];

/** Plan de coupe : un point et une normale (repère plateau). */
export interface CutPlane {
	point: Vec3;
	normal: Vec3;
}

function normalize(v: Vec3): Vec3 {
	const len = Math.hypot(v[0], v[1], v[2]) || 1;
	return [v[0] / len, v[1] / len, v[2] / len];
}

/** Distance signée d'un point au plan (positive du côté de la normale). */
export function signedDistance(plane: CutPlane, p: Vec3): number {
	const n = normalize(plane.normal);
	return (
		(p[0] - plane.point[0]) * n[0] + (p[1] - plane.point[1]) * n[1] + (p[2] - plane.point[2]) * n[2]
	);
}

function lerp(a: Vec3, b: Vec3, t: number): Vec3 {
	return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
}

/** Clippe un polygone contre un demi-espace du plan (garde le côté demandé). */
function clip(points: Vec3[], plane: CutPlane, keepPositive: boolean): Vec3[] {
	const out: Vec3[] = [];
	const side = (p: Vec3) => (keepPositive ? 1 : -1) * signedDistance(plane, p);
	for (let i = 0; i < points.length; i++) {
		const cur = points[i];
		const nxt = points[(i + 1) % points.length];
		const dc = side(cur);
		const dn = side(nxt);
		if (dc >= 0) out.push(cur);
		if (dc >= 0 !== dn >= 0) {
			out.push(lerp(cur, nxt, dc / (dc - dn)));
		}
	}
	return out;
}

function triangleAt(mesh: SceneMesh, t: number): [Vec3, Vec3, Vec3] {
	const idx = (k: number) => mesh.indices[t * 3 + k];
	const at = (i: number): Vec3 => [
		mesh.positions[i * 3],
		mesh.positions[i * 3 + 1],
		mesh.positions[i * 3 + 2]
	];
	return [at(idx(0)), at(idx(1)), at(idx(2))];
}

/** Sépare un maillage par un plan en deux moitiés (au-dessus / en dessous). */
export function splitByPlane(
	mesh: SceneMesh,
	plane: CutPlane
): { above: SceneMesh; below: SceneMesh } {
	const above: number[] = [];
	const below: number[] = [];
	const triangles = mesh.indices.length / 3;
	for (let t = 0; t < triangles; t++) {
		const tri = triangleAt(mesh, t);
		for (const [poly, sink] of [
			[clip(tri, plane, true), above],
			[clip(tri, plane, false), below]
		] as const) {
			// Triangulation en éventail du polygone clippé.
			for (let k = 1; k + 1 < poly.length; k++) {
				sink.push(...poly[0], ...poly[k], ...poly[k + 1]);
			}
		}
	}
	return { above: meshFromTriangleSoup(above), below: meshFromTriangleSoup(below) };
}

/**
 * Grille de positions de connecteurs sur un plan de coupe horizontal, dans
 * l'emprise `[min, max]` (mm), au pas `spacing`. Les tenons sont centrés dans
 * chaque cellule ; la version géométrique (dovetails 3D) est posée par le tool.
 */
export function connectorGrid(min: Vec3, max: Vec3, spacing: number, z: number): Vec3[] {
	const points: Vec3[] = [];
	if (spacing <= 0) return points;
	for (let x = min[0] + spacing / 2; x < max[0]; x += spacing) {
		for (let y = min[1] + spacing / 2; y < max[1]; y += spacing) {
			points.push([x, y, z]);
		}
	}
	return points;
}
