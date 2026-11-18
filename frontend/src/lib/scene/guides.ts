// Repères visuels de la vue 3D pendant un outil (phase 26) : plan de coupe
// (gizmo Cut d'Orca, qui dessine le plan dans la scène) et points/segment de
// mesure (gizmo Measure). Fonctions pures, testables : la vue 3D ne fait que
// dessiner ce qui est calculé ici.

export type Vec3 = [number, number, number];
export type Axis = 'x' | 'y' | 'z';

export const AXIS_INDEX: Record<Axis, 0 | 1 | 2> = { x: 0, y: 1, z: 2 };

/** Plan dessiné : centre monde, normale (axe), dimensions dans le plan (mm). */
export interface GuidePlane {
	center: Vec3;
	normal: Vec3;
	size: [number, number];
	/** Phase 28 — plan incliné : orientation (quaternion three.js x, y, z, w). */
	quaternion?: [number, number, number, number];
	/** Rotation Orca du plan (degrés, Rz·Ry·Rx) : anneaux de rotation. */
	rotation?: Vec3;
	/** Connecteurs posés (centres monde, diamètre, profondeur, côtés du prisme). */
	connectors?: {
		id: string;
		center: Vec3;
		size: number;
		depth: number;
		segments: number;
		rotation: number;
		selected: boolean;
	}[];
	/** Clic sur le plan (pose d'un connecteur), point monde. */
	onclick?: (world: Vec3) => void;
	/** Anneaux de rotation du plan : nouvelle rotation (degrés, ordre Orca). */
	onrotate?: (rotation: Vec3) => void;
}

/** Repères de la scène : plan de coupe et/ou points de mesure (monde, mm). */
export interface SceneGuides {
	plane?: GuidePlane | null;
	points?: Vec3[];
}

/** Repère de l'objet : origine monde du maillage recentré et échelle. */
export interface ObjectFrame {
	origin: Vec3;
	scale: Vec3;
}

const IDENTITY: ObjectFrame = { origin: [0, 0, 0], scale: [1, 1, 1] };

/** Décalage local (repère recentré) → coordonnée monde sur l'axe. */
export function cutWorld(offset: number, axis: Axis, frame: ObjectFrame = IDENTITY): number {
	const a = AXIS_INDEX[axis];
	return frame.origin[a] + offset * (frame.scale[a] || 1);
}

/** Coordonnée monde sur l'axe → décalage local (repère recentré). */
export function cutOffset(world: number, axis: Axis, frame: ObjectFrame = IDENTITY): number {
	const a = AXIS_INDEX[axis];
	return (world - frame.origin[a]) / (frame.scale[a] || 1);
}

/**
 * Plan de coupe à dessiner : perpendiculaire à `axis`, au décalage local
 * `offset`, couvrant l'emprise de l'objet (+ `margin` mm de chaque côté) —
 * comme le plan semi-transparent du gizmo Cut d'Orca. Rotation de l'objet
 * ignorée (la coupe serveur est faite dans le repère du maillage).
 */
export function cutGuidePlane(
	axis: Axis,
	offset: number,
	bounds: { min: Vec3; max: Vec3 },
	frame: ObjectFrame = IDENTITY,
	margin = 4
): GuidePlane {
	const a = AXIS_INDEX[axis];
	const center = [0, 1, 2].map(
		(i) => frame.origin[i] + ((bounds.min[i] + bounds.max[i]) / 2) * (frame.scale[i] || 1)
	) as Vec3;
	center[a] = cutWorld(offset, axis, frame);
	const ext = [0, 1, 2].map(
		(i) => Math.abs((bounds.max[i] - bounds.min[i]) * (frame.scale[i] || 1)) + 2 * margin
	);
	const normal: Vec3 = [0, 0, 0];
	normal[a] = 1;
	// Dimensions dans le repère du PlaneGeometry après `guideRotation` :
	// Z → (X, Y) ; X → (Z, Y) ; Y → (X, Z).
	const size: [number, number] =
		a === 2 ? [ext[0], ext[1]] : a === 0 ? [ext[2], ext[1]] : [ext[0], ext[2]];
	return { center, normal, size };
}

/**
 * Rotation Euler (XYZ, radians) qui amène la normale +Z d'un PlaneGeometry
 * sur la normale du plan (un des trois axes).
 */
export function guideRotation(normal: Vec3): Vec3 {
	if (Math.abs(normal[0]) > 0.5) return [0, Math.PI / 2, 0];
	if (Math.abs(normal[1]) > 0.5) return [-Math.PI / 2, 0, 0];
	return [0, 0, 0];
}

/** Caméra vue par le calcul de taille écran (sous-ensemble de three.js). */
export interface MarkerCamera {
	/** Perspective : champ vertical (degrés). */
	fov?: number;
	/** Orthographique : hauteur du frustum (top − bottom) et zoom. */
	orthoHeight?: number;
	zoom?: number;
}

/** Rayon des repères de mesure à l'écran (px), comme les repères d'Orca. */
export const MARKER_PX = 5;

/**
 * Rayon monde (mm) d'un repère dessiné à `px` pixels à l'écran : les points de
 * mesure d'Orca (GLGizmoMeasure) gardent une taille écran constante, au lieu
 * de sphères de 1,2 mm invisibles de loin et énormes en gros plan.
 */
export function markerWorldRadius(
	px: number,
	distance: number,
	viewportHeight: number,
	camera: MarkerCamera
): number {
	const h = Math.max(1, viewportHeight);
	const perPixel =
		camera.orthoHeight !== undefined
			? camera.orthoHeight / Math.max(1e-6, camera.zoom ?? 1) / h
			: (2 * Math.max(0, distance) * Math.tan(((camera.fov ?? 45) * Math.PI) / 360)) / h;
	return Math.min(500, Math.max(1e-3, px * perPixel));
}

/** Points de mesure (au plus deux) et segment à tracer entre eux. */
export function measureGuides(points: readonly Vec3[]): SceneGuides {
	const pts = points.slice(-2).map((p) => [...p] as Vec3);
	return { points: pts };
}
