// Audit 42 I1 — « Importer » un 3MF multi-plateaux dans un projet : les objets
// des plateaux 2+ arrivent avec leurs positions absolues de la grille de
// plateaux d'Orca (PartPlateList::compute_shape_position, pas de largeur ×
// 1,2). Le serveur ne renvoie que les objets ; on retrouve leur plateau d'origine
// par la position (même règle que `reassign_plates_by_position` côté serveur),
// puis la position est ramenée dans le repère de ce plateau. Pur → testable.
import type { BedShape } from './bed';
import { PLATE_GAP, plateOrigin } from './plate-layout';

export interface ImportedPlacement {
	/** Plateau d'origine dans le 3MF (0-based). */
	plate: number;
	/** Position XY dans le repère du plateau d'origine. */
	local: [number, number];
}

/** Nombre maximal de plateaux d'un projet Orca (PartPlateList::MAX_PLATES_COUNT). */
export const MAX_IMPORTED_PLATES = 36;

/**
 * Plateau d'origine de chaque objet (centre XY `positions`) : la plus petite
 * grille de plateaux (1, 2, … plateaux) dont une cellule (plateau + moitié de
 * l'écart de chaque côté) contient chaque objet. Aucune grille → tout reste
 * sur le premier plateau, positions inchangées.
 */
export function importedPlates(
	positions: readonly [number, number][],
	bed: BedShape
): ImportedPlacement[] {
	const keep = positions.map((p) => ({ plate: 0, local: [p[0], p[1]] as [number, number] }));
	if (positions.length === 0) return keep;
	const gx = (bed.width * PLATE_GAP) / 2;
	const gy = (bed.depth * PLATE_GAP) / 2;
	const x0 = bed.origin.x;
	const y0 = bed.origin.y;
	for (let count = 1; count <= MAX_IMPORTED_PLATES; count++) {
		const out: ImportedPlacement[] = [];
		for (const [x, y] of positions) {
			let found: ImportedPlacement | null = null;
			for (let k = 0; k < count && !found; k++) {
				const [ox, oy] = plateOrigin(k, count, bed);
				const lx = x - ox;
				const ly = y - oy;
				if (lx >= x0 - gx && lx < x0 + bed.width + gx && ly >= y0 - gy && ly < y0 + bed.depth + gy)
					found = { plate: k, local: [lx, ly] };
			}
			if (!found) break;
			out.push(found);
		}
		if (out.length === positions.length) return out;
	}
	return keep;
}
