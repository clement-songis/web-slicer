// Maths de transformation d'objet (T052) : type `Transform` (position mm,
// rotation en degrés Euler XYZ, échelle) et helpers purs pour le panneau
// numérique et la pose à plat. Les formules quaternion/Euler suivent celles de
// Three.js (ordre XYZ) pour rester cohérentes avec le rendu. Pur → testable.

/** Transformation d'un objet dans le repère plateau. */
export interface Transform {
	/** Translation (mm) dans le repère plateau. */
	position: [number, number, number];
	/** Rotation Euler XYZ en degrés. */
	rotation: [number, number, number];
	/** Facteurs d'échelle par axe. */
	scale: [number, number, number];
}

/** Transformation neutre. */
export const IDENTITY: Transform = {
	position: [0, 0, 0],
	rotation: [0, 0, 0],
	scale: [1, 1, 1]
};

const DEG = Math.PI / 180;

/** Échelle minimale autorisée (évite 0 / valeurs négatives qui aplatissent l'objet). */
export const MIN_SCALE = 0.001;

/** Ramène un facteur d'échelle au minimum autorisé s'il est trop petit. */
export function clampScale(value: number): number {
	return Number.isFinite(value) && value >= MIN_SCALE ? value : MIN_SCALE;
}

/** Normalise un angle en degrés dans l'intervalle (-180, 180]. */
export function normalizeAngle(deg: number): number {
	let a = deg % 360;
	if (a > 180) a -= 360;
	if (a <= -180) a += 360;
	return a;
}

/** Applique une échelle uniforme (multiplie les trois axes par `factor`). */
export function uniformScale(t: Transform, factor: number): Transform {
	return {
		...t,
		scale: [
			clampScale(t.scale[0] * factor),
			clampScale(t.scale[1] * factor),
			clampScale(t.scale[2] * factor)
		]
	};
}

type Vec3 = [number, number, number];
type Quat = [number, number, number, number]; // x, y, z, w

function normalize(v: Vec3): Vec3 {
	const len = Math.hypot(v[0], v[1], v[2]) || 1;
	return [v[0] / len, v[1] / len, v[2] / len];
}

/**
 * Quaternion (Three.js `setFromUnitVectors`) faisant tourner le vecteur unitaire
 * `from` vers `to`. Gère le cas antiparallèle (choix d'un axe orthogonal).
 */
export function quaternionFromUnitVectors(from: Vec3, to: Vec3): Quat {
	const f = normalize(from);
	const t = normalize(to);
	let r = f[0] * t[0] + f[1] * t[1] + f[2] * t[2] + 1;
	let x: number, y: number, z: number;
	if (r < 1e-8) {
		// Vecteurs opposés : rotation de 180° autour d'un axe orthogonal à `from`.
		r = 0;
		if (Math.abs(f[0]) > Math.abs(f[2])) {
			x = -f[1];
			y = f[0];
			z = 0;
		} else {
			x = 0;
			y = -f[2];
			z = f[1];
		}
	} else {
		x = f[1] * t[2] - f[2] * t[1];
		y = f[2] * t[0] - f[0] * t[2];
		z = f[0] * t[1] - f[1] * t[0];
	}
	const len = Math.hypot(x, y, z, r) || 1;
	return [x / len, y / len, z / len, r / len];
}

/** Convertit un quaternion en angles Euler XYZ (degrés), formules Three.js. */
export function eulerDegFromQuaternion(q: Quat): Vec3 {
	const [x, y, z, w] = q;
	const m11 = 1 - 2 * (y * y + z * z);
	const m12 = 2 * (x * y - w * z);
	const m13 = 2 * (x * z + w * y);
	const m22 = 1 - 2 * (x * x + z * z);
	const m23 = 2 * (y * z - w * x);
	const m32 = 2 * (y * z + w * x);
	const m33 = 1 - 2 * (x * x + y * y);

	const clamp13 = Math.min(1, Math.max(-1, m13));
	const ey = Math.asin(clamp13);
	let ex: number, ez: number;
	if (Math.abs(m13) < 0.9999999) {
		ex = Math.atan2(-m23, m33);
		ez = Math.atan2(-m12, m11);
	} else {
		ex = Math.atan2(m32, m22);
		ez = 0;
	}
	return [ex / DEG, ey / DEG, ez / DEG];
}

/**
 * Applique une rotation Euler XYZ (degrés) à un vecteur (matrice de rotation
 * Three.js `makeRotationFromEuler`, ordre XYZ). Utilisé pour la pose à plat et
 * les tests.
 */
export function rotateVectorByEulerDeg(euler: Vec3, v: Vec3): Vec3 {
	const [rx, ry, rz] = [euler[0] * DEG, euler[1] * DEG, euler[2] * DEG];
	const c1 = Math.cos(rx),
		s1 = Math.sin(rx);
	const c2 = Math.cos(ry),
		s2 = Math.sin(ry);
	const c3 = Math.cos(rz),
		s3 = Math.sin(rz);
	const m11 = c2 * c3,
		m12 = -c2 * s3,
		m13 = s2;
	const m21 = c1 * s3 + c3 * s1 * s2,
		m22 = c1 * c3 - s1 * s2 * s3,
		m23 = -c2 * s1;
	const m31 = s1 * s3 - c1 * c3 * s2,
		m32 = c3 * s1 + c1 * s2 * s3,
		m33 = c1 * c2;
	return [
		m11 * v[0] + m12 * v[1] + m13 * v[2],
		m21 * v[0] + m22 * v[1] + m23 * v[2],
		m31 * v[0] + m32 * v[1] + m33 * v[2]
	];
}

/**
 * Rotation Euler (degrés) qui pose la facette de normale `normal` à plat sur le
 * plateau : la normale est amenée vers le bas (-Z, repère Z-up d'Orca).
 */
export function layFlatRotation(normal: Vec3): Vec3 {
	const q = quaternionFromUnitVectors(normal, [0, 0, -1]);
	return eulerDegFromQuaternion(q);
}
