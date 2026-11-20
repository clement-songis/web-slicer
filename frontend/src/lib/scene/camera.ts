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

/**
 * Pose cadrant une boîte englobante (aperçu G-code : objets imprimés, comme le
 * zoom d'Orca sur la boîte du G-code au premier affichage). Même direction
 * trois-quarts que `frameBed` ; rayon minimal 5 mm (petite pièce).
 */
export function frameBox(
	min: [number, number, number],
	max: [number, number, number],
	margin = 1.15
): CameraPose {
	const c: [number, number, number] = [
		(min[0] + max[0]) / 2,
		(min[1] + max[1]) / 2,
		(min[2] + max[2]) / 2
	];
	const radius = Math.max(5, 0.5 * Math.hypot(max[0] - min[0], max[1] - min[1], max[2] - min[2]));
	const d = fitDistance(radius * margin);
	const dir = normalize([-0.6, -0.8, 0.7]);
	return { position: [c[0] + dir[0] * d, c[1] + dir[1] * d, c[2] + dir[2] * d], target: c };
}

/** Boîte englobante de sommets (x,y,z à plat), ou null si vide. */
export function boundsOf(
	positions: Float32Array
): { min: [number, number, number]; max: [number, number, number] } | null {
	if (positions.length < 3) return null;
	const min: [number, number, number] = [Infinity, Infinity, Infinity];
	const max: [number, number, number] = [-Infinity, -Infinity, -Infinity];
	for (let i = 0; i < positions.length; i += 3)
		for (let k = 0; k < 3; k++) {
			const v = positions[i + k];
			if (v < min[k]) min[k] = v;
			if (v > max[k]) max[k] = v;
		}
	return { min, max };
}

function normalize([x, y, z]: [number, number, number]): [number, number, number] {
	const len = Math.hypot(x, y, z) || 1;
	return [x / len, y / len, z / len];
}

/** Vues nommées du menu Vue (parité OrcaSlicer, repère Z-up). */
export type NamedView =
	| 'default'
	| 'top'
	| 'bottom'
	| 'front'
	| 'rear'
	| 'left'
	| 'right'
	/** Ctrl+7 : vue du plateau courant (Orca PartPlateList::select_plate_view →
	 *  Camera::select_view("topfront") : dessus-avant à 45°, centrée sur le plateau). */
	| 'plate';

/** Distance d'observation cadrant le volume du plateau (mutualisée avec `frameBed`). */
function bedDistance(bed: BedShape): number {
	const radius =
		0.5 * Math.hypot(Math.max(bed.width, 1), Math.max(bed.depth, 1), Math.max(bed.height, 1));
	return fitDistance(radius);
}

/** Direction caméra→plateau (normalisée) de chaque vue nommée. */
const VIEW_DIR: Record<NamedView, [number, number, number]> = {
	default: normalize([-0.6, -0.8, 0.7]),
	top: [0, 0, 1],
	bottom: [0, 0, -1],
	front: [0, -1, 0],
	rear: [0, 1, 0],
	left: [-1, 0, 0],
	right: [1, 0, 0],
	// Orca « topfront » : m_target − 0,707·d·Y + 0,707·d·Z.
	plate: normalize([0, -0.707, 0.707])
};

/**
 * Pose caméra d'une vue nommée : cible au centre du plateau (mi-hauteur), caméra
 * reculée dans la direction de la vue à la distance de cadrage.
 */
export function viewPose(bed: BedShape, view: NamedView): CameraPose {
	const target: [number, number, number] = [bed.center.x, bed.center.y, bed.height / 2];
	const dir = VIEW_DIR[view];
	const d = bedDistance(bed);
	return {
		position: [target[0] + dir[0] * d, target[1] + dir[1] * d, target[2] + dir[2] * d],
		target
	};
}

/** Vecteur « haut » caméra d'une vue : Y+ pour dessus/dessous (sinon Z+ dégénère). */
export function viewUp(view: NamedView): [number, number, number] {
	if (view === 'top') return [0, 1, 0];
	if (view === 'bottom') return [0, -1, 0];
	return [0, 0, 1];
}

/**
 * Ctrl+flèches d'Orca : déplacement de la sélection dans le repère caméra,
 * projeté sur le plateau (GLCanvas3D::TranslationProcessor). Les vues de
 * profil peuvent donc avoir un déplacement vertical nul après projection.
 */
export function cameraSpaceNudge(
	view: NamedView,
	key: string,
	fine = false
): [number, number] | null {
	const forward = VIEW_DIR[view].map((v) => -v) as [number, number, number];
	const up = viewUp(view);
	const cross = (
		a: [number, number, number],
		b: [number, number, number]
	): [number, number, number] => [
		a[1] * b[2] - a[2] * b[1],
		a[2] * b[0] - a[0] * b[2],
		a[0] * b[1] - a[1] * b[0]
	];
	const right = normalize(cross(forward, up));
	const cameraUp = cross(right, forward);
	const direction =
		key === 'ArrowLeft'
			? right.map((v) => -v)
			: key === 'ArrowRight'
				? right
				: key === 'ArrowUp'
					? cameraUp
					: key === 'ArrowDown'
						? cameraUp.map((v) => -v)
						: null;
	if (!direction) return null;
	const step = fine ? 1 : 10;
	return [direction[0] * step, direction[1] * step];
}

/** Facteur de zoom borné des raccourcis I/O (1 = cadrage nominal). */
export function clampZoom(factor: number): number {
	return Number.isFinite(factor) ? Math.max(0.2, Math.min(5, factor)) : 1;
}

/** Rapproche/éloigne la caméra de sa cible sans changer son orientation. */
export function zoomPose(pose: CameraPose, factor: number): CameraPose {
	const z = clampZoom(factor);
	return {
		target: pose.target,
		position: pose.position.map(
			(v, i) => pose.target[i] + (v - pose.target[i]) / z
		) as CameraPose['position']
	};
}

/** Projection caméra (menu Vue : « Use Perspective View » / « Use Orthogonal View »). */
export type Projection = 'perspective' | 'orthographic';

/**
 * Zoom d'une caméra orthogonale Three.js (pixels par mm) cadrant le plateau
 * dans une vue d'environ 900 px, comme la vue perspective par défaut. Pur.
 */
export function orthoZoom(bed: BedShape, viewportPx = 900): number {
	const extent = Math.max(bed.width, bed.depth, 1);
	return Math.max(0.1, viewportPx / (extent * 1.6));
}
