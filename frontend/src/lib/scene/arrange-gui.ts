// Application du résultat de l'arrangement GUI (lot 34, POST /arrange avec
// `options`) : placements multi-plateaux avec matrice monde complète (rotation
// et échelle conservées), réaffectation des objets aux plateaux et objets trop
// grands (« Arrangement ignored the following objects… » d'Orca). Pur → testable.
import type { Placement } from '$lib/api/types';
import { eulerDegFromQuaternion, type Transform } from './transform';
import type { Plate } from './plates.svelte';

/**
 * Décompose une matrice monde 4×4 colonne-major (Placement.matrix) en
 * Transform de scène : translation, rotation Euler XYZ (degrés, formules
 * Three.js via quaternion de Shepperd), échelle = norme des colonnes. Les
 * matrices du serveur n'ont pas de cisaillement (rotation + échelle).
 */
export function transformFromMatrix(matrix: readonly number[]): Transform {
	if (matrix.length < 16) return { position: [0, 0, 0], rotation: [0, 0, 0], scale: [1, 1, 1] };
	const sx = Math.hypot(matrix[0], matrix[1], matrix[2]);
	const sy = Math.hypot(matrix[4], matrix[5], matrix[6]);
	const sz = Math.hypot(matrix[8], matrix[9], matrix[10]);
	// Colonnes de la matrice de rotation (normalisées par les échelles).
	const r00 = safeDiv(matrix[0], sx),
		r10 = safeDiv(matrix[1], sx),
		r20 = safeDiv(matrix[2], sx);
	const r01 = safeDiv(matrix[4], sy),
		r11 = safeDiv(matrix[5], sy),
		r21 = safeDiv(matrix[6], sy);
	const r02 = safeDiv(matrix[8], sz),
		r12 = safeDiv(matrix[9], sz),
		r22 = safeDiv(matrix[10], sz);
	// Quaternion (méthode de Shepperd, plus grand terme pilote).
	const trace = r00 + r11 + r22;
	let x: number, y: number, z: number, w: number;
	if (trace > 0) {
		const s = Math.sqrt(trace + 1) * 2;
		w = s / 4;
		x = (r21 - r12) / s;
		y = (r02 - r20) / s;
		z = (r10 - r01) / s;
	} else if (r00 > r11 && r00 > r22) {
		const s = Math.sqrt(1 + r00 - r11 - r22) * 2;
		w = (r21 - r12) / s;
		x = s / 4;
		y = (r01 + r10) / s;
		z = (r02 + r20) / s;
	} else if (r11 > r22) {
		const s = Math.sqrt(1 + r11 - r00 - r22) * 2;
		w = (r02 - r20) / s;
		x = (r01 + r10) / s;
		y = s / 4;
		z = (r12 + r21) / s;
	} else {
		const s = Math.sqrt(1 + r22 - r00 - r11) * 2;
		w = (r10 - r01) / s;
		x = (r02 + r20) / s;
		y = (r12 + r21) / s;
		z = s / 4;
	}
	return {
		position: [matrix[12], matrix[13], matrix[14]],
		rotation: eulerDegFromQuaternion([x, y, z, w]),
		scale: [sx || 1, sy || 1, sz || 1]
	};
}

function safeDiv(a: number, b: number): number {
	return b > 1e-12 || b < -1e-12 ? a / b : 0;
}

/** Nouvelle transformation d'un objet après arrangement (matrice > x,y). */
export function placedTransform(current: Transform, placement: Placement): Transform {
	if (placement.matrix && placement.matrix.length >= 16) {
		return transformFromMatrix(placement.matrix);
	}
	return { ...current, position: [placement.x, placement.y, current.position[2]] };
}

/**
 * Affectations à réappliquer après arrangement : objets dont le plateau
 * d'origine (index dans la liste) diffère du plateau calculé. `plateIndex` =
 * index 0-based du plateau dans la réponse.
 */
export function plateMoves(
	plates: readonly Plate[],
	placements: readonly Placement[]
): { objectId: string; plateIndex: number }[] {
	const moves: { objectId: string; plateIndex: number }[] = [];
	for (const placement of placements) {
		if (placement.plate_index === undefined || placement.unarranged) continue;
		const from = plates.findIndex((plate) => plate.objectIds.includes(placement.id));
		if (from !== placement.plate_index && placement.plate_index >= 0) {
			moves.push({ objectId: placement.id, plateIndex: placement.plate_index });
		}
	}
	return moves;
}

/**
 * Objets trop grands pour un plateau : posés tels quels sur un plateau ajouté
 * (notification d'Orca « Arrangement ignored the following objects which can't
 * fit into a single bed »).
 */
export function unarrangedIds(placements: readonly Placement[]): string[] {
	return placements.filter((p) => p.unarranged).map((p) => p.id);
}
