// Navigateur 3D (Orca « Navigateur 3D », GLCanvas3D::_render_3d_navigator :
// cube ImGuizmo::ViewManipulate en bas à gauche de la vue, faces Avant /
// Dessus / Gauche… cliquables) : cube CSS orienté comme la caméra (repère
// Z vertical), chaque face renvoie la vue nommée correspondante. Pur.
import type { NamedView } from './camera';

type Vec3 = [number, number, number];

/** Face du cube : vue nommée, libellé (Orca fr), normale et repère du texte. */
export interface CubeFace {
	view: Exclude<NamedView, 'default' | 'plate'>;
	label: string;
	normal: Vec3;
	/** Droite et bas du texte (repère de la vue nommée vue de face). */
	right: Vec3;
	down: Vec3;
}

/** Faces dans le repère monde (Z vertical, avant = −Y), comme select_view d'Orca. */
export const CUBE_FACES: CubeFace[] = [
	// Libellés : clés i18n anglaises, traduites au rendu.
	{ view: 'front', label: 'Front||cube', normal: [0, -1, 0], right: [1, 0, 0], down: [0, 0, -1] },
	{ view: 'rear', label: 'Rear', normal: [0, 1, 0], right: [-1, 0, 0], down: [0, 0, -1] },
	{ view: 'left', label: 'Left', normal: [-1, 0, 0], right: [0, -1, 0], down: [0, 0, -1] },
	{ view: 'right', label: 'Right', normal: [1, 0, 0], right: [0, 1, 0], down: [0, 0, -1] },
	{ view: 'top', label: 'Top', normal: [0, 0, 1], right: [1, 0, 0], down: [0, -1, 0] },
	{ view: 'bottom', label: 'Bottom', normal: [0, 0, -1], right: [1, 0, 0], down: [0, 1, 0] }
];

const cross = (a: Vec3, b: Vec3): Vec3 => [
	a[1] * b[2] - a[2] * b[1],
	a[2] * b[0] - a[0] * b[2],
	a[0] * b[1] - a[1] * b[0]
];
const norm = (a: Vec3): Vec3 => {
	const l = Math.hypot(...a) || 1;
	return [a[0] / l, a[1] / l, a[2] / l];
};
const fmt = (n: number) => (Math.abs(n) < 1e-9 ? 0 : Math.round(n * 1e6) / 1e6);

/**
 * Repère écran de la caméra pour une direction de vue `forward` (Z vertical) :
 * droite = forward × Z (repli sur +X en vue de dessus / dessous), haut =
 * −forward × droite.
 */
export function cameraBasis(forward: Vec3): { right: Vec3; up: Vec3; forward: Vec3 } {
	const f = norm(forward);
	let r = cross(f, [0, 0, 1]);
	r = Math.hypot(...r) < 1e-6 ? [1, 0, 0] : norm(r);
	const u = norm(cross([-f[0], -f[1], -f[2]], r));
	return { right: r, up: u, forward: f };
}

/** `matrix3d` CSS du cube : monde → écran (x droite, y bas, z vers l'observateur). */
export function cubeTransform(forward: Vec3): string {
	const { right: r, up: u, forward: f } = cameraBasis(forward);
	const cols = [0, 1, 2].map((j) => [r[j], -u[j], -f[j], 0]);
	return `matrix3d(${[...cols.flat(), 0, 0, 0, 1].map(fmt).join(', ')})`;
}

/** `matrix3d` CSS d'une face (demi-côté `half` px) dans le repère du cube. */
export function faceTransform(face: CubeFace, half: number): string {
	const c = face.normal.map((n) => n * half);
	return `matrix3d(${[...face.right, 0, ...face.down, 0, ...face.normal, 0, ...c, 1].map(fmt).join(', ')})`;
}

/** Face la plus en vue (normale la plus opposée à la direction de vue). */
export function facingFace(forward: Vec3): CubeFace {
	const f = norm(forward);
	return CUBE_FACES.reduce((best, face) => {
		const d = -(face.normal[0] * f[0] + face.normal[1] * f[1] + face.normal[2] * f[2]);
		const b = -(best.normal[0] * f[0] + best.normal[1] * f[1] + best.normal[2] * f[2]);
		return d > b ? face : best;
	});
}
