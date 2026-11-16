// Interactions écran de la scène, parité GLCanvas3D / GLSelectionRectangle
// d'Orca : rectangle de sélection (Maj+glisser), pas de 1 mm du gizmo Move
// (Maj), position du plan de coupe des gizmos de peinture (Alt+molette).
// Logique pure : le viewport fournit les emprises écran projetées.

/** Rectangle écran (px CSS du canvas), bornes ordonnées. */
export interface ScreenRect {
	x0: number;
	y0: number;
	x1: number;
	y1: number;
}

/** Emprise écran d'un objet : boîte englobante de ses sommets projetés. */
export type ScreenBox = ScreenRect;

/** Rectangle normalisé entre le point de départ et le point courant du glisser. */
export function rectFromDrag(a: [number, number], b: [number, number]): ScreenRect {
	return {
		x0: Math.min(a[0], b[0]),
		y0: Math.min(a[1], b[1]),
		x1: Math.max(a[0], b[0]),
		y1: Math.max(a[1], b[1])
	};
}

/** Deux rectangles écran se recouvrent-ils (bords inclus) ? */
export function rectsIntersect(a: ScreenRect, b: ScreenRect): boolean {
	return a.x0 <= b.x1 && b.x0 <= a.x1 && a.y0 <= b.y1 && b.y0 <= a.y1;
}

/** Objets dont la projection écran intersecte le rectangle (ordre stable). */
export function rectHits(rect: ScreenRect, boxes: Readonly<Record<string, ScreenBox>>): string[] {
	return Object.keys(boxes).filter((id) => rectsIntersect(rect, boxes[id]));
}

/** Point dans une emprise écran (marge éventuelle en px). */
export function pointInBox(p: [number, number], box: ScreenBox, margin = 0): boolean {
	return (
		p[0] >= box.x0 - margin &&
		p[0] <= box.x1 + margin &&
		p[1] >= box.y0 - margin &&
		p[1] <= box.y1 + margin
	);
}

/**
 * Sélection après un rectangle : comme Orca en mode Maj (« Select »), les
 * objets touchés s'ajoutent à la sélection courante.
 */
export function selectByRect(current: ReadonlySet<string>, hits: readonly string[]): Set<string> {
	const next = new Set(current);
	for (const id of hits) next.add(id);
	return next;
}

/** Pas de translation avec Maj (gizmo Move d'Orca : « press to snap by 1mm »). */
export const MOVE_SNAP_MM = 1;

/** Arrondit une position au pas donné (mm), sans « -0 ». */
export function snapPosition(
	p: readonly [number, number, number],
	step = MOVE_SNAP_MM
): [number, number, number] {
	return p.map((v) => Math.round(v / step) * step + 0) as [number, number, number];
}

/** Pas du plan de coupe par cran de molette (GLGizmoPainterBase : 0.01). */
export const SECTION_STEP = 0.01;

/**
 * Alt+molette : déplace le plan de coupe (ratio 0..1 de la hauteur de l'objet,
 * 0 = pas de coupe). `dir` = +1 molette vers le haut, -1 vers le bas.
 */
export function stepSectionRatio(ratio: number, dir: 1 | -1): number {
	const next = Math.min(1, Math.max(0, ratio + dir * SECTION_STEP));
	return Math.round(next * 100) / 100;
}

type Vec3 = [number, number, number];

/** Plan de coupe : points gardés si `normal · p <= offset` (repère monde, mm). */
export interface SectionPlane {
	/** Normale unitaire tournée vers la caméra (`-camera.get_dir_forward()`). */
	normal: Vec3;
	offset: number;
}

/** Normale du plan de coupe pour une direction de vue (vers l'avant de la caméra). */
export function sectionNormal(forward: Vec3): Vec3 {
	const len = Math.hypot(...forward) || 1;
	return forward.map((v) => -v / len + 0) as Vec3;
}

/**
 * ObjectClipper::set_position_by_ratio d'Orca : plan de normale `normal`
 * (direction de vue, conservée tant que le gizmo est ouvert) qui balaie la
 * sphère englobante de l'instance (centre, rayon) du côté caméra vers
 * l'arrière quand le ratio croît. `null` si ratio 0 (coupe inactive).
 */
export function sectionPlane(
	normal: Vec3,
	center: Vec3,
	radius: number,
	ratio: number
): SectionPlane | null {
	if (ratio <= 0) return null;
	const dist = normal[0] * center[0] + normal[1] * center[1] + normal[2] * center[2];
	return { normal, offset: dist + radius - ratio * 2 * radius };
}
