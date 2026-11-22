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

/** Rectangle occupé au sol (centre XY + dimensions, mm). */
export interface Footprint {
	x: number;
	y: number;
	width: number;
	depth: number;
}

/** Empreinte au sol d'un objet de la scène (position + échelle XY). */
export function objectFootprint(o: SceneObject, fallback: [number, number]): Footprint {
	const { width, depth } = footprint(o.mesh);
	const [sx, sy] = [o.scale?.[0] ?? 1, o.scale?.[1] ?? 1];
	return {
		x: o.position?.[0] ?? fallback[0],
		y: o.position?.[1] ?? fallback[1],
		width: width * Math.abs(sx),
		depth: depth * Math.abs(sy)
	};
}

/** Empreintes des objets pour le corps `ArrangeRequest.items`. */
export function arrangeItems(objects: readonly SceneObject[]): ArrangeItem[] {
	// Phase 46 (ARR-a) : emprise à l'échelle courante (comme objectFootprint).
	return objects.map((o) => {
		const { width, depth } = objectFootprint(o, [0, 0]);
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

/** Rectangle du plateau (mm) : coin min, dimensions. */
export interface BedRect {
	x: number;
	y: number;
	width: number;
	depth: number;
}

/**
 * Place un nouvel objet à côté des objets du plateau (phase 28) — Orca
 * Plater::priv::load_model_objects (AUTOPLACEMENT_ON_LOAD) : plateau vide →
 * centre ; sinon GLCanvas3D::get_nearest_empty_cell, grille de pas 10 mm
 * alignée sur le centre, cellules triées par distance au centre. Orca ne teste
 * que le centre de la cellule contre les enveloppes existantes (le nouvel
 * objet chevauche souvent son voisin) : on teste l'empreinte entière, écart
 * gap (min_object_distance d'Orca, 6 mm), dans le plateau. Rien de libre →
 * repli d'Orca : centre + 5 % de la plus grande dimension du plateau.
 */
export function nearestFreeCell(
	size: { width: number; depth: number },
	existing: readonly Footprint[],
	bed: BedRect,
	step = 10,
	gap = 6
): [number, number] {
	const cx = bed.x + bed.width / 2;
	const cy = bed.y + bed.depth / 2;
	if (existing.length === 0) return [cx, cy];
	const hw = size.width / 2;
	const hd = size.depth / 2;
	const cells: [number, number][] = [];
	const minX = cx - step * Math.floor((cx - bed.x) / step);
	const minY = cy - step * Math.floor((cy - bed.y) / step);
	for (let x = minX; x < bed.x + bed.width - step / 2 + 1e-9; x += step)
		for (let y = minY; y < bed.y + bed.depth - step / 2 + 1e-9; y += step) cells.push([x, y]);
	cells.sort((a, b) => Math.hypot(a[0] - cx, a[1] - cy) - Math.hypot(b[0] - cx, b[1] - cy));
	for (const [x, y] of cells) {
		if (
			x - hw < bed.x ||
			x + hw > bed.x + bed.width ||
			y - hd < bed.y ||
			y + hd > bed.y + bed.depth
		)
			continue;
		const free = existing.every(
			(f) =>
				Math.abs(f.x - x) >= (f.width + size.width) / 2 + gap - 1e-9 ||
				Math.abs(f.y - y) >= (f.depth + size.depth) / 2 + gap - 1e-9
		);
		if (free) return [x, y];
	}
	const off = 0.05 * Math.max(bed.width, bed.depth);
	return [cx + off, cy + off];
}

/** Séquence d'impression normalisée (`print_sequence` d'Orca). */
export type PrintSequence = 'by layer' | 'by object';

function normSequence(v: unknown): PrintSequence | null {
	return v === 'by layer' || v === 'by object' ? v : null;
}

/**
 * Séquence réelle d'un plateau (PartPlate::get_real_print_seq) : surcharge du
 * plateau (« par défaut » / vide = aucune), sinon la séquence globale du
 * traitement (défaut Orca : par couche).
 */
export function realPlateSequence(plateValue: unknown, globalValue: unknown): PrintSequence {
	return normSequence(plateValue) ?? normSequence(globalValue) ?? 'by layer';
}

/**
 * Audit 45 SEQ-a — appels d'arrangement pour « A » / « Tout arranger ».
 * ArrangeJob::prepare_all d'Orca verrouille le temps de l'arrangement les
 * plateaux dont la séquence diffère de la séquence globale
 * (m_uncompatible_plates) : un plateau réglé « par objet » sous un traitement
 * « par couche » ne bougeait donc pas du tout, erreurs « trop proche »
 * comprises. On garde l'arrangement global d'Orca pour les autres plateaux,
 * puis on arrange le plateau actif avec sa propre séquence
 * (init_arrange_params, PREPARE_STATE_MENU : is_seq_print = séquence réelle du
 * plateau, rayon de dégagement + jupe, BED_SHRINK_SEQ_PRINT).
 */
export function arrangeAllScopes(plateValue: unknown, globalValue: unknown): ('all' | 'plate')[] {
	const global = normSequence(globalValue) ?? 'by layer';
	return realPlateSequence(plateValue, globalValue) === global ? ['all'] : ['all', 'plate'];
}
