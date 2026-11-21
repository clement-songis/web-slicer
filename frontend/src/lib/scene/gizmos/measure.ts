// Outil de mesure (T057) : distances et angles sur le maillage. Géométrie pure
// (points et vecteurs 3D en mm) → testable. La sélection des points/facettes se
// fait par raycast côté gizmo, qui appelle ces fonctions.

export type Vec3 = [number, number, number];

/** Distance euclidienne entre deux points (mm). */
export function distance(a: Vec3, b: Vec3): number {
	return Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]);
}

/** Composantes ΔX/ΔY/ΔZ entre deux points (utile pour l'affichage Orca). */
export function delta(a: Vec3, b: Vec3): Vec3 {
	return [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
}

function dot(a: Vec3, b: Vec3): number {
	return a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
}

/** Angle (degrés) entre deux vecteurs. 0 pour un vecteur nul. */
export function angleDeg(u: Vec3, v: Vec3): number {
	const lu = Math.hypot(u[0], u[1], u[2]);
	const lv = Math.hypot(v[0], v[1], v[2]);
	if (lu === 0 || lv === 0) return 0;
	const cos = Math.min(1, Math.max(-1, dot(u, v) / (lu * lv)));
	return (Math.acos(cos) * 180) / Math.PI;
}

/** Angle (degrés) entre deux facettes, à partir de leurs normales. */
export function angleBetweenNormals(n1: Vec3, n2: Vec3): number {
	return angleDeg(n1, n2);
}

/** Mesure entre deux points : distance et composantes. */
export interface PointMeasurement {
	distance: number;
	delta: Vec3;
}

export function measurePoints(a: Vec3, b: Vec3): PointMeasurement {
	return { distance: distance(a, b), delta: delta(a, b) };
}

// — Audit 41 T2 : mesure face à face / point à face (Orca 2.4.1 Measure.cpp,
//   get_measurement : plans parallèles → distance ; sinon angle ; point–plan →
//   distance au plan). Une face = point (centre de la face plane) + normale.
export interface MeasureFace {
	point: Vec3;
	normal: Vec3;
}

export interface FeatureMeasurement {
	/** Distance (mm) : plans parallèles ou point–plan ; null si non définie. */
	distance: number | null;
	/** Angle (degrés) entre deux faces non parallèles ; null sinon. */
	angle: number | null;
}

const unit = (v: Vec3): Vec3 => {
	const l = Math.hypot(v[0], v[1], v[2]);
	return l === 0 ? [0, 0, 0] : [v[0] / l, v[1] / l, v[2] / l];
};

/** Measure.cpp are_parallel : |n1·n2| ≈ 1. */
export function areParallel(n1: Vec3, n2: Vec3): boolean {
	return Math.abs(dot(unit(n1), unit(n2))) > 1 - 1e-3;
}

/** Distance d'un point au plan (point, normale). */
export function pointPlaneDistance(p: Vec3, plane: MeasureFace): number {
	const n = unit(plane.normal);
	return Math.abs(dot(n, delta(plane.point, p)));
}

/**
 * Deux faces : distance si parallèles (Hyperplane::absDistance), sinon angle
 * dièdre (Orca : acos(n2 · −n1), 90° pour deux faces adjacentes d'un cube).
 */
export function measureFaces(a: MeasureFace, b: MeasureFace): FeatureMeasurement {
	if (areParallel(a.normal, b.normal))
		return { distance: pointPlaneDistance(a.point, b), angle: null };
	const n1 = unit(a.normal);
	const n2 = unit(b.normal);
	const cos = Math.min(1, Math.max(-1, -dot(n1, n2)));
	return { distance: null, angle: (Math.acos(cos) * 180) / Math.PI };
}

/** Point puis face (ou l'inverse) : distance du point au plan de la face. */
export function measurePointFace(p: Vec3, face: MeasureFace): FeatureMeasurement {
	return { distance: pointPlaneDistance(p, face), angle: null };
}
