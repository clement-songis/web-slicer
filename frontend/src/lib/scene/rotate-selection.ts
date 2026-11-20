// Page préc. / Page suiv. (Orca GLCanvas3D::on_key → _do_rotate(±π/4)) :
// rotation de la sélection autour de l'axe Z du plateau. Un objet seul tourne
// sur lui-même ; plusieurs objets tournent ensemble autour du centre de la
// sélection (transformation « Joint » d'Orca), positions comprises. Pur.
import { rotateEulerAboutWorldZ } from './transform';

type Vec3 = [number, number, number];

export interface RotatableObject {
	id: string;
	position?: Vec3;
	rotation?: Vec3;
}

export interface RotatedPose {
	position: Vec3;
	rotation: Vec3;
}

/** Nouvelle pose de chaque objet après une rotation de `deg` degrés (axe Z). */
export function rotateSelectionZ(
	objects: RotatableObject[],
	deg: number
): Map<string, RotatedPose> {
	const out = new Map<string, RotatedPose>();
	if (objects.length === 0 || !Number.isFinite(deg)) return out;
	const xs = objects.map((o) => o.position?.[0] ?? 0);
	const ys = objects.map((o) => o.position?.[1] ?? 0);
	const cx = (Math.min(...xs) + Math.max(...xs)) / 2;
	const cy = (Math.min(...ys) + Math.max(...ys)) / 2;
	const a = (deg * Math.PI) / 180;
	const [c, s] = [Math.cos(a), Math.sin(a)];
	const round = (v: number) => Math.round(v * 1e6) / 1e6;
	for (const o of objects) {
		const [x, y, z] = o.position ?? [0, 0, 0];
		const [dx, dy] = [x - cx, y - cy];
		out.set(o.id, {
			position: [round(cx + dx * c - dy * s), round(cy + dx * s + dy * c), z],
			rotation: rotateEulerAboutWorldZ(o.rotation ?? [0, 0, 0], deg)
		});
	}
	return out;
}
