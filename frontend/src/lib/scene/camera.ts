// Cadrage caméra de la scène de préparation (T050). Repère Z-up (comme Orca).
// Pur (sans Three.js) → testable : positionnement orbital initial qui cadre le
// plateau, distance d'ajustement selon la boîte englobante.

import type { BedShape } from './bed';

/** Pose caméra : position et cible dans le repère plateau (mm, Z-up). */
export interface CameraPose {
	position: [number, number, number];
	target: [number, number, number];
}

/** Distance d'observation pour cadrer une boîte de rayon `radius` à `fov` (deg). */
export function fitDistance(radius: number, fovDeg = 50): number {
	const fov = (fovDeg * Math.PI) / 180;
	return radius / Math.sin(fov / 2);
}

/**
 * Pose orbitale initiale : cible au centre du plateau (à mi-hauteur), caméra
 * reculée en vue trois-quarts (X−, Y−, Z+) proportionnellement à la taille.
 */
export function frameBed(bed: BedShape): CameraPose {
	const cx = bed.center.x;
	const cy = bed.center.y;
	const cz = bed.height / 2;

	// Rayon englobant (demi-diagonale du volume).
	const radius =
		0.5 * Math.hypot(Math.max(bed.width, 1), Math.max(bed.depth, 1), Math.max(bed.height, 1));
	const d = fitDistance(radius);

	// Direction trois-quarts normalisée (Z-up).
	const dir = normalize([-0.6, -0.8, 0.7]);
	return {
		position: [cx + dir[0] * d, cy + dir[1] * d, cz + dir[2] * d],
		target: [cx, cy, cz]
	};
}

function normalize([x, y, z]: [number, number, number]): [number, number, number] {
	const len = Math.hypot(x, y, z) || 1;
	return [x / len, y / len, z / len];
}
