// Repère monde ↔ repère objet pour le gizmo des volumes (pièce, négatif,
// modificateur, bloqueur, forceur). Un volume est rendu enfant du maillage de
// son objet (`ModelObject`) : centre `offset` et rotation propre exprimés dans
// le repère recentré de l'objet, donc soumis à la translation, la rotation et
// l'échelle de l'objet. Le gizmo, lui, manipule un objet témoin en repère monde
// (axes non déformés par l'échelle de l'objet) ; ces fonctions convertissent
// l'un vers l'autre. Le résultat est appliqué par `applyVolumeTransform`
// (échelle du geste = facteur sur la taille du pavé, axes propres du volume).
// Formules quaternion/Euler XYZ de Three.js. Pur → testable.

import { eulerDegFromQuaternion, type Transform } from './transform';
import { applyVolumeTransform, type SceneVolume } from './volumes';

type Vec3 = [number, number, number];
type Quat = [number, number, number, number]; // x, y, z, w

const DEG = Math.PI / 180;

/** Quaternion d'une rotation Euler XYZ en degrés (Three.js `setFromEuler`). */
export function quaternionFromEulerDeg(euler: readonly number[]): Quat {
	const [c1, c2, c3] = euler.map((a) => Math.cos((a * DEG) / 2));
	const [s1, s2, s3] = euler.map((a) => Math.sin((a * DEG) / 2));
	return [
		s1 * c2 * c3 + c1 * s2 * s3,
		c1 * s2 * c3 - s1 * c2 * s3,
		c1 * c2 * s3 + s1 * s2 * c3,
		c1 * c2 * c3 - s1 * s2 * s3
	];
}

function multiply(a: Quat, b: Quat): Quat {
	const [ax, ay, az, aw] = a;
	const [bx, by, bz, bw] = b;
	return [
		ax * bw + aw * bx + ay * bz - az * by,
		ay * bw + aw * by + az * bx - ax * bz,
		az * bw + aw * bz + ax * by - ay * bx,
		aw * bw - ax * bx - ay * by - az * bz
	];
}

function conjugate(q: Quat): Quat {
	return [-q[0], -q[1], -q[2], q[3]];
}

function rotate(q: Quat, v: Vec3): Vec3 {
	const p = multiply(multiply(q, [v[0], v[1], v[2], 0]), conjugate(q));
	return [p[0], p[1], p[2]];
}

/** Arrondi d'affichage : supprime le bruit flottant (et le zéro négatif). */
function tidy(n: number, step: number): number {
	const r = Math.round(n / step) * step;
	return Math.abs(r) < step / 2 ? 0 : Number(r.toFixed(6));
}

const scaleOf = (t: Transform): Vec3 => t.scale.map((s) => (s === 0 ? 1e-9 : s)) as Vec3;

/**
 * Pose monde d'un volume : centre = T + R·(S ⊙ offset), rotation = R·Rv,
 * échelle 1 (le geste d'échelle repart de 1).
 */
export function volumeWorldTransform(object: Transform, volume: SceneVolume): Transform {
	const q = quaternionFromEulerDeg(object.rotation);
	const s = scaleOf(object);
	const local: Vec3 = [volume.offset[0] * s[0], volume.offset[1] * s[1], volume.offset[2] * s[2]];
	const r = rotate(q, local);
	const qv = quaternionFromEulerDeg(volume.rotation ?? [0, 0, 0]);
	return {
		position: [object.position[0] + r[0], object.position[1] + r[1], object.position[2] + r[2]],
		rotation: eulerDegFromQuaternion(multiply(q, qv)),
		scale: [1, 1, 1]
	};
}

/**
 * Inverse : pose monde du témoin → transformation dans le repère de l'objet
 * (offset = S⁻¹ ⊙ R⁻¹·(p − T), rotation = R⁻¹·Rw, échelle du geste conservée).
 * Avec une échelle d'objet non uniforme et un volume tourné, la rotation est
 * celle des axes non cisaillés (même approximation que le panneau Orca).
 */
export function worldToObjectTransform(object: Transform, world: Transform): Transform {
	const q = quaternionFromEulerDeg(object.rotation);
	const qi = conjugate(q);
	const s = scaleOf(object);
	const d: Vec3 = [
		world.position[0] - object.position[0],
		world.position[1] - object.position[1],
		world.position[2] - object.position[2]
	];
	const r = rotate(qi, d);
	const rotation = eulerDegFromQuaternion(multiply(qi, quaternionFromEulerDeg(world.rotation)));
	return {
		position: [tidy(r[0] / s[0], 1e-4), tidy(r[1] / s[1], 1e-4), tidy(r[2] / s[2], 1e-4)],
		rotation: rotation.map((a) => tidy(a, 1e-4)) as Vec3,
		scale: [...world.scale] as Vec3
	};
}

/**
 * Volume résultant d'un geste : `base` = volume au début du geste, `world` =
 * pose courante du témoin (échelle relative au début du geste).
 */
export function volumeFromWorld(
	base: SceneVolume,
	object: Transform,
	world: Transform
): SceneVolume {
	return applyVolumeTransform(base, worldToObjectTransform(object, world));
}
