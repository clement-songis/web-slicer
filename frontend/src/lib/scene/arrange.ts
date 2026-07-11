// Aide pure à l'arrangement de plateau (barre d'outils `arrange`, T106) : calcule
// l'empreinte au sol (XY) d'un maillage pour le corps `POST …/arrange`, puis
// réapplique les positions calculées (`Placement`) aux objets de la scène. Pur →
// testable sous bun ; la page orchestre l'appel API.
import type { SceneMesh, SceneObject } from './mesh';
import type { ArrangeItem, Placement } from '../api/types';

/** Empreinte au sol (mm) d'un maillage : étendues X (largeur) et Y (profondeur). */
export function footprint(mesh: SceneMesh): { width: number; depth: number } {
	const p = mesh.positions;
	let minX = Infinity;
	let maxX = -Infinity;
	let minY = Infinity;
	let maxY = -Infinity;
	for (let i = 0; i < p.length; i += 3) {
		const x = p[i];
		const y = p[i + 1];
		if (x < minX) minX = x;
		if (x > maxX) maxX = x;
		if (y < minY) minY = y;
		if (y > maxY) maxY = y;
	}
	if (!Number.isFinite(minX)) return { width: 0, depth: 0 };
	return { width: maxX - minX, depth: maxY - minY };
}

/** Empreintes des objets pour le corps `ArrangeRequest.items`. */
export function arrangeItems(objects: readonly SceneObject[]): ArrangeItem[] {
	return objects.map((o) => {
		const { width, depth } = footprint(o.mesh);
		return { id: o.id, width, depth };
	});
}

/**
 * Réapplique les positions calculées (centre XY, mm) aux objets. La hauteur Z
 * existante est préservée ; les objets sans placement restent inchangés.
 */
export function applyPlacements(
	objects: readonly SceneObject[],
	placements: readonly Placement[]
): SceneObject[] {
	const byId = new Map(placements.map((pl) => [pl.id, pl]));
	return objects.map((o) => {
		const pl = byId.get(o.id);
		if (!pl) return o;
		const z = o.position?.[2] ?? 0;
		return { ...o, position: [pl.x, pl.y, z] as [number, number, number] };
	});
}
