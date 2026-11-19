// Disposition des plateaux côte à côte dans la vue 3D (phase 30), comme
// PartPlateList d'Orca : grille de `compute_colum_count(n)` colonnes, pas de
// `largeur × (1 + LOGICAL_PART_PLATE_GAP)` en X et `profondeur × (1 + gap)` en
// −Y (les rangées suivantes passent « devant »). Le plateau actif reste à
// l'origine de la scène (repère plateau inchangé pour tous les outils) : les
// autres sont décalés de leur origine Orca moins celle du plateau actif.
// Pur (sans Three.js) → testable.
import type { BedShape } from './bed';
import type { SceneObject } from './mesh';

/** Plateau affiché à côté du plateau actif (vue 3D multi-plateaux). */
export interface PlateView {
	id: string;
	index: number;
	name: string;
	/** Décalage (mm) par rapport au plateau actif (qui reste à l'origine). */
	offset: [number, number];
	objects: SceneObject[];
}

/** PartPlate.cpp : `LOGICAL_PART_PLATE_GAP = 1. / 5.`. */
export const PLATE_GAP = 1 / 5;

/** PartPlate.hpp `compute_colum_count` : colonnes de la grille de plateaux. */
export function plateColumns(count: number): number {
	const value = Math.sqrt(Math.max(1, count));
	const rounded = Math.round(value);
	return value > rounded ? rounded + 1 : rounded;
}

/** PartPlateList::compute_shape_position : origine du plateau `index` (mm). */
export function plateOrigin(index: number, count: number, bed: BedShape): [number, number] {
	const cols = plateColumns(count);
	const row = Math.floor(index / cols);
	const col = index % cols;
	return [col * bed.width * (1 + PLATE_GAP), -row * bed.depth * (1 + PLATE_GAP)];
}

/** Décalage (mm) de chaque plateau par rapport au plateau actif (actif = [0, 0]). */
export function plateOffsets(
	count: number,
	activeIndex: number,
	bed: BedShape
): [number, number][] {
	const active = plateOrigin(Math.max(0, activeIndex), count, bed);
	return Array.from({ length: count }, (_, i) => {
		const o = plateOrigin(i, count, bed);
		return [o[0] - active[0], o[1] - active[1]];
	});
}

/** Emprise au sol (mm, repère scène). */
export interface Footprint {
	minX: number;
	minY: number;
	maxX: number;
	maxY: number;
}

/** Rectangle d'un plateau décalé (boîte englobante du contour, comme get_plate_box). */
export function plateRect(bed: BedShape, offset: [number, number]): Footprint {
	return {
		minX: bed.center.x - bed.width / 2 + offset[0],
		maxX: bed.center.x + bed.width / 2 + offset[0],
		minY: bed.center.y - bed.depth / 2 + offset[1],
		maxY: bed.center.y + bed.depth / 2 + offset[1]
	};
}

function intersects(a: Footprint, b: Footprint): boolean {
	return a.minX < b.maxX && a.maxX > b.minX && a.minY < b.maxY && a.maxY > b.minY;
}

/**
 * Plateau d'arrivée d'un objet déplacé (PartPlateList::notify_instance_update) :
 * il reste sur son plateau tant que son emprise le touche ; sinon il rejoint le
 * premier plateau (ordre des index) que son emprise touche. `null` = pas de
 * changement (toujours sur le plateau actif, ou hors de tout plateau : Orca le
 * range alors dans le plateau « non imprimable », ici il reste où il est et la
 * validation le signale hors plateau).
 */
export function dropPlateIndex(
	footprint: Footprint,
	bed: BedShape,
	count: number,
	activeIndex: number
): number | null {
	const offsets = plateOffsets(count, activeIndex, bed);
	if (intersects(footprint, plateRect(bed, offsets[activeIndex] ?? [0, 0]))) return null;
	for (let i = 0; i < count; i++) {
		if (i !== activeIndex && intersects(footprint, plateRect(bed, offsets[i]))) return i;
	}
	return null;
}

/**
 * Indication « Glissez un modèle ici » : seulement si le plateau actif n'a
 * réellement aucun objet — un import en échec de conversion (sans maillage,
 * donc absent de la vue) compte comme un objet présent.
 */
export function showDropHint(
	renderedCount: number,
	imports: readonly { objectId: string; status: string }[],
	plateObjectIds: readonly string[]
): boolean {
	if (renderedCount > 0) return false;
	const onPlate = new Set(plateObjectIds);
	return !imports.some((i) => i.status === 'failed' && onPlate.has(i.objectId));
}
