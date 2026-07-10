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
