// Audit 41 V2 : après une mise à l'échelle ou une rotation (panneau, gizmo,
// miroir, orientation), l'objet est reposé sur le plateau comme dans Orca 2.4.1
// (GLCanvas3D::do_scale / do_rotate / do_mirror, « Fixes sinking/flying
// instances (snaps object to buildplate) ») : un objet posé (ou flottant) avant
// l'opération voit son point le plus bas ramené à z = 0 ; seul un objet déjà
// volontairement enfoncé AVANT et encore enfoncé APRÈS le reste. Pur.
import { restingZ, type SceneMesh } from './mesh';
import type { Transform } from './transform';

/** Model.hpp : SINKING_Z_THRESHOLD. */
export const SINKING_Z_THRESHOLD = -0.001;

const same = (a: readonly number[], b: readonly number[]) =>
	a.length === b.length && a.every((v, i) => Math.abs(v - b[i]) <= 1e-12);

/** Z du point le plus bas d'un objet placé (repère du plateau). */
export function objectMinZ(mesh: SceneMesh, t: Transform): number {
	return t.position[2] - restingZ(mesh, t.rotation, t.scale);
}

/**
 * Transformation `next` corrigée : si l'échelle ou la rotation change, l'objet
 * est reposé sur le plateau selon la règle d'Orca. Sinon (déplacement seul,
 * Z saisi à la main compris) elle est rendue telle quelle.
 */
export function autoDrop(
	mesh: SceneMesh | null | undefined,
	prev: Transform,
	next: Transform
): Transform {
	if (!mesh || mesh.positions.length === 0) return next;
	if (same(prev.scale, next.scale) && same(prev.rotation, next.rotation)) return next;
	const before = objectMinZ(mesh, prev);
	const after = objectMinZ(mesh, next);
	if (after === 0) return next;
	if (before < SINKING_Z_THRESHOLD && after <= SINKING_Z_THRESHOLD) return next;
	return {
		...next,
		position: [next.position[0], next.position[1], restingZ(mesh, next.rotation, next.scale)]
	};
}
