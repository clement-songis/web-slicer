// Audit 42 I1 — objets hors du volume imprimable, comme Orca 2.4.1
// (GLVolumeCollection::check_outside_state, 3DScene.cpp:1229-1330 ;
// BuildVolume::object_state) : l'enveloppe convexe de l'objet placé est
// comparée au contour `printable_area` et à `printable_height`.
//   - « inside »  : entièrement dans le volume (une partie enfoncée sous le
//     plateau est permise : Orca coupe les objets « sinking ») ;
//   - « partly »  : à cheval sur le bord ou plus haut que le volume
//     (ModelInstancePVS_Partly_Outside) → Orca grise « Trancher » et affiche
//     « Following objects are laid over the boundary of plate… » ;
//   - « outside » : entièrement hors du plateau. Orca ne le rattache à aucun
//     plateau (il n'est pas tranché) ; ici l'appartenance au plateau est
//     explicite, l'objet est donc signalé et le tranchage bloqué tant qu'il
//     n'est pas replacé (drapeau `off_bed` lu par le serveur).
// Pur (sans rendu) → testable.
import { Euler } from 'three/src/math/Euler.js';
import { Matrix4 } from 'three/src/math/Matrix4.js';
import { Quaternion } from 'three/src/math/Quaternion.js';
import { Vector3 } from 'three/src/math/Vector3.js';
import type { BedShape, Point2 } from './bed';
import { hullPoints } from './footprint-overlap';
import type { SceneObject } from './mesh';

export type VolumeState = 'inside' | 'partly' | 'outside';

/** Tolérance (mm) : BuildVolume::BedEpsilon d'Orca est de 3e-4 mm ; les
 *  maillages sont ici en float32 (erreur d'arrondi ~1e-4 sur 350 mm). */
export const BED_EPSILON = 1e-3;

const DEG = Math.PI / 180;

/** Point dans le contour du plateau (bord compris, tolérance `eps`). */
export function insideBed(x: number, y: number, bed: BedShape, eps = BED_EPSILON): boolean {
	const x0 = bed.origin.x;
	const y0 = bed.origin.y;
	if (x < x0 - eps || y < y0 - eps || x > x0 + bed.width + eps || y > y0 + bed.depth + eps)
		return false;
	if (bed.rectangular || bed.polygon.length < 3) return true;
	return pointInPolygon(x, y, bed.polygon) || nearEdge(x, y, bed.polygon, eps);
}

function pointInPolygon(x: number, y: number, poly: readonly Point2[]): boolean {
	let inside = false;
	for (let i = 0, j = poly.length - 1; i < poly.length; j = i++) {
		const a = poly[i];
		const b = poly[j];
		if (a.y > y !== b.y > y && x < ((b.x - a.x) * (y - a.y)) / (b.y - a.y) + a.x) inside = !inside;
	}
	return inside;
}

function nearEdge(x: number, y: number, poly: readonly Point2[], eps: number): boolean {
	for (let i = 0, j = poly.length - 1; i < poly.length; j = i++) {
		const a = poly[j];
		const b = poly[i];
		const dx = b.x - a.x;
		const dy = b.y - a.y;
		const len2 = dx * dx + dy * dy;
		const t = len2 > 0 ? Math.max(0, Math.min(1, ((x - a.x) * dx + (y - a.y) * dy) / len2)) : 0;
		const px = a.x + t * dx - x;
		const py = a.y + t * dy - y;
		if (px * px + py * py <= eps * eps) return true;
	}
	return false;
}

/** État d'un objet placé vis-à-vis du volume imprimable de son plateau. */
export function objectVolumeState(
	o: Pick<SceneObject, 'mesh' | 'position' | 'rotation' | 'scale'>,
	bed: BedShape,
	eps = BED_EPSILON
): VolumeState {
	const pts = hullPoints(o.mesh);
	if (pts.length < 3) return 'inside';
	const r = o.rotation ?? [0, 0, 0];
	const s = o.scale ?? [1, 1, 1];
	const p = o.position ?? [0, 0, 0];
	const m = new Matrix4().compose(
		new Vector3(p[0], p[1], p[2]),
		new Quaternion().setFromEuler(new Euler(r[0] * DEG, r[1] * DEG, r[2] * DEG)),
		new Vector3(s[0], s[1], s[2])
	);
	const v = new Vector3();
	let inCount = 0;
	let outCount = 0;
	let tooHigh = false;
	let above = 0;
	for (let i = 0; i < pts.length; i += 3) {
		v.set(pts[i], pts[i + 1], pts[i + 2]).applyMatrix4(m);
		// Partie enfoncée sous le plateau : ignorée (coupée par Orca).
		if (v.z < -eps) continue;
		above++;
		if (v.z > bed.height + eps) tooHigh = true;
		if (insideBed(v.x, v.y, bed, eps)) inCount++;
		else outCount++;
	}
	if (above === 0) return 'inside';
	if (inCount === 0) return footprintCrossesBed(o, m, bed, eps) ? 'partly' : 'outside';
	return outCount > 0 || tooHigh ? 'partly' : 'inside';
}

/** Objet dont aucun sommet n'est sur le plateau mais qui le recouvre (plus
 *  grand que le plateau, ou en croix) : à cheval, pas dehors. */
function footprintCrossesBed(
	o: Pick<SceneObject, 'mesh'>,
	m: Matrix4,
	bed: BedShape,
	eps: number
): boolean {
	const pts = hullPoints(o.mesh);
	const v = new Vector3();
	let minX = Infinity;
	let minY = Infinity;
	let maxX = -Infinity;
	let maxY = -Infinity;
	for (let i = 0; i < pts.length; i += 3) {
		v.set(pts[i], pts[i + 1], pts[i + 2]).applyMatrix4(m);
		minX = Math.min(minX, v.x);
		maxX = Math.max(maxX, v.x);
		minY = Math.min(minY, v.y);
		maxY = Math.max(maxY, v.y);
	}
	const x0 = bed.origin.x;
	const y0 = bed.origin.y;
	return !(
		maxX < x0 - eps ||
		maxY < y0 - eps ||
		minX > x0 + bed.width + eps ||
		minY > y0 + bed.depth + eps
	);
}

/** Objets hors volume (état ≠ « inside ») par id. */
export function outsideStates(
	objects: readonly Pick<SceneObject, 'id' | 'mesh' | 'position' | 'rotation' | 'scale'>[],
	bed: BedShape
): Record<string, Exclude<VolumeState, 'inside'>> {
	const out: Record<string, Exclude<VolumeState, 'inside'>> = {};
	for (const o of objects) {
		const state = objectVolumeState(o, bed);
		if (state !== 'inside') out[o.id] = state;
	}
	return out;
}

/** Boîte du volume imprimable (repère du plateau) pour l'assombrissement des
 *  fragments hors volume (gouraud.fs d'Orca : mix(couleur, noir, 1/3)). Pas de
 *  borne basse : la partie enfoncée n'est pas « dehors ». */
export function printVolumeBox(bed: BedShape): {
	min: [number, number, number];
	max: [number, number, number];
} {
	return {
		min: [bed.origin.x, bed.origin.y, -1e9],
		max: [bed.origin.x + bed.width, bed.origin.y + bed.depth, bed.height]
	};
}
