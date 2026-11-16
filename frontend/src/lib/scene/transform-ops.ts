// Opérations de transformation déclenchées par raccourci ou menu contextuel
// (parité Orca : flèches de déplacement, miroir, « Scale to build volume »).
// Pures (sans Three.js ni DOM) → testables sous Vitest.
import type { SceneMesh } from './mesh';
import { rotateVectorByEulerDeg } from './transform';

type Vec3 = [number, number, number];

/** Miroir le long d'un axe (`MenuFactory::append_menu_items_mirror`) : signe de
 *  l'échelle inversé sur cet axe. Le rendu (déterminant négatif) et le tranchage
 *  (matrice d'instance) gèrent l'orientation des faces. */
export function mirrorScale(scale: Vec3 | undefined, axis: 0 | 1 | 2): Vec3 {
	const next: Vec3 = [...(scale ?? [1, 1, 1])] as Vec3;
	next[axis] = -next[axis];
	return next;
}

/** Dimensions monde (mm) d'un maillage après échelle puis rotation Euler XYZ. */
export function worldSize(
	mesh: SceneMesh,
	rotation: Vec3 = [0, 0, 0],
	scale: Vec3 = [1, 1, 1]
): Vec3 {
	const p = mesh.positions;
	if (p.length === 0) return [0, 0, 0];
	const rotated = rotation.some((r) => r !== 0);
	const min: Vec3 = [Infinity, Infinity, Infinity];
	const max: Vec3 = [-Infinity, -Infinity, -Infinity];
	for (let i = 0; i < p.length; i += 3) {
		let v: Vec3 = [p[i] * scale[0], p[i + 1] * scale[1], p[i + 2] * scale[2]];
		if (rotated) v = rotateVectorByEulerDeg(rotation, v);
		for (let a = 0; a < 3; a++) {
			if (v[a] < min[a]) min[a] = v[a];
			if (v[a] > max[a]) max[a] = v[a];
		}
	}
	return [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
}

/**
 * Facteur uniforme qui fait tenir l'objet dans le volume d'impression
 * (`Selection::scale_to_fit_print_volume`) : le plus petit rapport
 * volume/objet sur les trois axes. `null` si l'objet est vide ou le volume
 * invalide.
 */
export function fitScaleFactor(
	size: Vec3,
	volume: { width: number; depth: number; height: number }
): number | null {
	const limits = [volume.width, volume.depth, volume.height];
	let factor = Infinity;
	for (let a = 0; a < 3; a++) {
		if (!(size[a] > 0)) continue;
		if (!(limits[a] > 0)) return null;
		factor = Math.min(factor, limits[a] / size[a]);
	}
	return Number.isFinite(factor) ? factor : null;
}

/** Applique un facteur uniforme à une échelle existante (signes de miroir conservés). */
export function scaleBy(scale: Vec3 | undefined, factor: number): Vec3 {
	const s = scale ?? [1, 1, 1];
	return [s[0] * factor, s[1] * factor, s[2] * factor];
}
