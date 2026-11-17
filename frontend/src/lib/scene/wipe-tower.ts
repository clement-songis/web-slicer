// Tour d'essuyage sur le plateau (Préparer), parité Orca :
// - taille estimée comme PartPlate::estimate_wipe_tower_size (avant tranchage) ;
// - profondeur minimale WipeTower::get_limit_depth_by_height, bordure auto
//   WipeTower::get_auto_brim_by_height ;
// - position (coin min, repère plateau) = wipe_tower_x/y[plateau] de la config
//   projet, bornée comme PartPlate::estimate_wipe_tower_polygon
//   (WIPE_TOWER_MARGIN = 1 mm + bordure).
// Logique pure, testée.

export const WIPE_TOWER_MARGIN = 1;
/** WipeTower::min_depth_per_height (hauteur → profondeur minimale, mm). */
const MIN_DEPTH_PER_HEIGHT: [number, number][] = [
	[5, 5],
	[100, 20],
	[250, 40],
	[350, 60]
];

export function limitDepthByHeight(maxHeight: number): number {
	const t = MIN_DEPTH_PER_HEIGHT;
	if (t[0][0] >= maxHeight) return t[0][1];
	for (let i = 0; i < t.length - 1; i++) {
		const [h0, d0] = t[i];
		const [h1, d1] = t[i + 1];
		if (h1 > maxHeight) return d0 + ((maxHeight - h0) / (h1 - h0)) * (d1 - d0);
	}
	return t[t.length - 1][1];
}

export function autoBrimByHeight(maxHeight: number): number {
	return maxHeight < 100 ? (maxHeight / 100) * 8 : 8;
}

export interface WipeTowerInputs {
	/** prime_tower_width (mm). */
	width: number;
	/** prime_volume (mm³). */
	primeVolume: number;
	layerHeight: number;
	/** prime_tower_infill_gap (%). */
	infillGap: number;
	/** wipe_tower_wall_type ('rib' | 'rectangle' | 'cone'). */
	wallType: string;
	ribWidth: number;
	extraRibLength: number;
	/** Hauteur du plus haut objet du plateau (mm). */
	maxHeight: number;
	/** Nombre de filaments utilisés sur le plateau. */
	plateExtruders: number;
	/** Nombre de buses de l'imprimante (nozzle_diameter). */
	nozzles: number;
	/** max(filament_change_length) et max(filament_diameter) (mm). */
	changeLength?: number;
	filamentDiameter?: number;
}

/** Taille estimée [largeur X, profondeur Y, hauteur Z] (mm) ; [0,0,0] si rien. */
export function estimateWipeTowerSize(p: WipeTowerInputs): [number, number, number] {
	if (p.plateExtruders <= 0) return [0, 0, 0];
	const layer = p.layerHeight > 0 ? p.layerHeight : 0.08;
	const extra = p.infillGap / 100;
	const d = p.filamentDiameter ?? 1.75;
	const changeVolume = ((p.changeLength ?? 0) * Math.PI * d * d) / 4;
	let volume = p.primeVolume * (p.nozzles === 2 ? p.plateExtruders : p.plateExtruders - 1);
	if (p.nozzles === 2) volume += changeVolume * Math.floor(p.plateExtruders / 2);
	if (p.wallType === 'rib') {
		let depth = Math.sqrt(Math.max(0, (volume / layer) * extra));
		if (p.plateExtruders > 1) {
			const volumeDepth = depth;
			depth = Math.max(limitDepthByHeight(p.maxHeight), depth);
			const rib = Math.min(p.ribWidth, depth / 2);
			depth = rib / Math.SQRT2 + Math.max(depth + p.extraRibLength, volumeDepth);
			return [depth, depth, p.maxHeight];
		}
		return [0, 0, p.maxHeight];
	}
	let depth = (volume / (layer * p.width)) * extra;
	if (depth > 1e-6) depth = Math.max(limitDepthByHeight(p.maxHeight), depth);
	return [p.width, depth, p.maxHeight];
}

/** Bordure effective (négative = automatique selon la hauteur). */
export function effectiveBrim(brim: number, height: number): number {
	return brim < 0 ? autoBrimByHeight(height) : brim;
}

/** Coin min borné au plateau (repère plateau, bed.width × bed.depth). */
export function clampTowerPosition(
	x: number,
	y: number,
	size: readonly [number, number, number],
	bed: { width: number; depth: number; origin?: { x: number; y: number } },
	brim: number
): [number, number] {
	const ox = bed.origin?.x ?? 0;
	const oy = bed.origin?.y ?? 0;
	const margin = WIPE_TOWER_MARGIN + brim;
	const clamp = (v: number, lo: number, hi: number) =>
		hi < lo ? lo : Math.min(hi, Math.max(lo, v));
	return [
		clamp(x, ox + margin, ox + bed.width - size[0] - margin),
		clamp(y, oy + margin, oy + bed.depth - size[1] - margin)
	];
}

/** Tour affichée (Orca GLCanvas3D::reload_scene) : tour activée, ≥ 2 filaments
 *  utilisés sur le plateau, au moins un objet ; en objet par objet seulement
 *  s'il n'y a qu'un objet imprimable. */
export function showWipeTower(o: {
	enabled: boolean;
	plateExtruders: number;
	objects: number;
	byObject: boolean;
}): boolean {
	if (!o.enabled || o.objects === 0 || o.plateExtruders < 2) return false;
	return !o.byObject || o.objects === 1;
}
