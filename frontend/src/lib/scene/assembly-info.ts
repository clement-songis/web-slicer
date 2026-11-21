// Vue d'assemblage (lot 39, Orca 2.4.1 GLCanvas3D::_render_assemble_control /
// _render_assemble_info, ModelObjectsClipper::set_position). Pur.
//  - « Informations sur l'assemblage » : boîte englobante de la sélection ;
//    « Volume : » = produit de ses dimensions (Orca affiche size0·size1·size2,
//    pas le volume du maillage), « Taille: » = « x × y × z », 2 décimales.
//  - « Vue en coupe » : plan de normale −direction de vue, qui balaie la sphère
//    englobante de tous les objets affichés (centre et rayon de leur boîte).
import { pieceWorldBox, type Box3, type CutScalePiece } from './cut-group-scale';
import { rotateVectorByEulerDeg } from './transform';
import { sectionPlane, type SectionPlane } from './screen-select';

type Vec3 = [number, number, number];

/** Union de boîtes (null si aucune boîte finie). */
export function unionBox(boxes: readonly Box3[]): Box3 | null {
	const min: Vec3 = [Infinity, Infinity, Infinity];
	const max: Vec3 = [-Infinity, -Infinity, -Infinity];
	for (const b of boxes)
		for (let a = 0; a < 3; a++) {
			min[a] = Math.min(min[a], b.min[a]);
			max[a] = Math.max(max[a], b.max[a]);
		}
	return min.every(Number.isFinite) && max.every(Number.isFinite) ? { min, max } : null;
}

/** Boîte monde des objets (maillage placé). */
export function objectsBox(objects: readonly Omit<CutScalePiece, 'id'>[]): Box3 | null {
	return unionBox(objects.map((o) => pieceWorldBox({ id: '', ...o })));
}

/**
 * Boîte monde d'une pièce (volume) : pavé `size` centré en `offset`, tourné
 * de sa rotation, puis transformation de l'objet (échelle, rotation, position).
 */
export function volumeWorldBox(
	object: { position: Vec3; rotation: Vec3; scale: Vec3 },
	volume: { size: Vec3; offset: Vec3; rotation?: Vec3 }
): Box3 {
	const min: Vec3 = [Infinity, Infinity, Infinity];
	const max: Vec3 = [-Infinity, -Infinity, -Infinity];
	const h = volume.size.map((s) => s / 2);
	for (const sx of [-1, 1])
		for (const sy of [-1, 1])
			for (const sz of [-1, 1]) {
				let p: Vec3 = [sx * h[0], sy * h[1], sz * h[2]];
				if (volume.rotation) p = rotateVectorByEulerDeg(volume.rotation, p);
				p = [p[0] + volume.offset[0], p[1] + volume.offset[1], p[2] + volume.offset[2]];
				p = [p[0] * object.scale[0], p[1] * object.scale[1], p[2] * object.scale[2]];
				p = rotateVectorByEulerDeg(object.rotation, p);
				for (let a = 0; a < 3; a++) {
					const w = p[a] + object.position[a];
					min[a] = Math.min(min[a], w);
					max[a] = Math.max(max[a], w);
				}
			}
	return { min, max };
}

export interface AssemblyInfo {
	size: Vec3;
	/** Volume de la boîte englobante (mm³), comme Orca. */
	volume: number;
}

export function assemblyInfo(box: Box3 | null): AssemblyInfo | null {
	if (!box) return null;
	const size = [0, 1, 2].map((a) => box.max[a] - box.min[a]) as Vec3;
	return { size, volume: size[0] * size[1] * size[2] };
}

/** Textes du panneau (format « %.2f » et « %.2f x %.2f x %.2f » d'Orca). */
export function formatAssemblyInfo(info: AssemblyInfo): { volume: string; size: string } {
	return {
		volume: info.volume.toFixed(2),
		size: info.size.map((s) => s.toFixed(2)).join(' x ')
	};
}

/**
 * Plan de la vue en coupe (ModelObjectsClipper::set_position) : normale de vue
 * figée, centre et rayon (demi-diagonale) de la boîte des objets affichés ;
 * ratio 0 → pas de coupe, 1 → tout est coupé.
 */
export function assemblySectionPlane(
	normal: Vec3 | null,
	box: Box3 | null,
	ratio: number
): SectionPlane | null {
	if (!normal || !box || !(ratio > 0)) return null;
	const center = [0, 1, 2].map((a) => (box.min[a] + box.max[a]) / 2) as Vec3;
	const radius = Math.hypot(...[0, 1, 2].map((a) => box.max[a] - box.min[a])) / 2;
	return sectionPlane(normal, center, radius, Math.min(1, ratio));
}
