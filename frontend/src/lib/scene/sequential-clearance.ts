// Contrôle « objet par objet » dans la vue Préparer (phase 27), comme la
// validation d'arrière-plan d'Orca (Print::sequential_print_clearance_valid) :
// objets trop proches (enveloppes décalées de la moitié du rayon de dégagement
// de la tête) et objets trop hauts pour passer sous le portique / le capot.
// Le moteur reste l'autorité au tranchage ; ce contrôle prévient AVANT.
import { Euler, Matrix4, Vector3 } from 'three';
import type { SceneMesh, SceneObject } from './mesh';

/** Paramètres machine (Orca : extruder_clearance_*, printable_height, nozzle_height). */
export interface ClearanceParams {
	radius: number;
	heightToRod: number;
	heightToLid: number;
	printableHeight: number;
	nozzleHeight: number;
}

/** Emprise monde d'un objet : boîte XY (centre, dimensions) et hauteur. */
export interface ClearanceItem {
	id: string;
	x: number;
	y: number;
	width: number;
	depth: number;
	height: number;
}

/** Orca Print.hpp : MAX_OUTER_NOZZLE_DIAMETER (objets tous plus bas que la buse). */
const MAX_OUTER_NOZZLE_DIAMETER = 4;

type Extents = { min: [number, number, number]; max: [number, number, number] };
const cache = new WeakMap<SceneMesh, Map<string, Extents>>();

/** Boîte du maillage après rotation (Euler XYZ, degrés) et échelle, mémorisée
 *  par maillage et transformation (un déplacement ne recalcule rien). */
function extents(mesh: SceneMesh, rotation: readonly number[], scale: readonly number[]): Extents {
	const key = [...rotation, ...scale].join(',');
	let byKey = cache.get(mesh);
	if (!byKey) cache.set(mesh, (byKey = new Map()));
	const hit = byKey.get(key);
	if (hit) return hit;
	const deg = Math.PI / 180;
	const m = new Matrix4()
		.makeRotationFromEuler(
			new Euler(rotation[0] * deg, rotation[1] * deg, rotation[2] * deg, 'XYZ')
		)
		.multiply(new Matrix4().makeScale(scale[0], scale[1], scale[2]));
	const v = new Vector3();
	const min: [number, number, number] = [Infinity, Infinity, Infinity];
	const max: [number, number, number] = [-Infinity, -Infinity, -Infinity];
	const p = mesh.positions;
	for (let i = 0; i < p.length; i += 3) {
		v.set(p[i], p[i + 1], p[i + 2]).applyMatrix4(m);
		const c = [v.x, v.y, v.z];
		for (let a = 0; a < 3; a++) {
			if (c[a] < min[a]) min[a] = c[a];
			if (c[a] > max[a]) max[a] = c[a];
		}
	}
	const out = Number.isFinite(min[0]) ? { min, max } : { min: [0, 0, 0], max: [0, 0, 0] };
	byKey.set(key, out as Extents);
	return out as Extents;
}

/** Emprise monde d'un objet de scène (position = centre du maillage recentré). */
export function clearanceItem(o: SceneObject, fallback: [number, number]): ClearanceItem {
	const e = extents(o.mesh, o.rotation ?? [0, 0, 0], o.scale ?? [1, 1, 1]);
	const [px, py] = [o.position?.[0] ?? fallback[0], o.position?.[1] ?? fallback[1]];
	return {
		id: o.id,
		x: px + (e.min[0] + e.max[0]) / 2,
		y: py + (e.min[1] + e.max[1]) / 2,
		width: e.max[0] - e.min[0],
		depth: e.max[1] - e.min[1],
		// Objet posé sur le plateau : sa hauteur est l'étendue Z.
		height: e.max[2] - e.min[2]
	};
}

/** Distance entre deux boîtes XY (0 si elles se touchent ou se chevauchent). */
function gap(a: ClearanceItem, b: ClearanceItem): number {
	const dx = Math.max(0, Math.abs(a.x - b.x) - (a.width + b.width) / 2);
	const dy = Math.max(0, Math.abs(a.y - b.y) - (a.depth + b.depth) / 2);
	return Math.hypot(dx, dy);
}

/**
 * Conflits d'une impression objet par objet, `items` dans l'ordre d'impression.
 * - `close` : objets dont l'enveloppe décalée de r/2 (coins arrondis, comme
 *   l'offset jtRound d'Orca) touche celle d'un objet imprimé avant ;
 * - `tall` : objets plus hauts que le dégagement disponible — capot
 *   (height_to_lid), ou portique (height_to_rod) si un objet imprimé ensuite
 *   partage sa bande Y ; le dernier n'est limité que par la hauteur imprimable.
 */
export function sequentialClearance(
	items: readonly ClearanceItem[],
	p: ClearanceParams
): { close: string[]; tall: string[] } {
	const allShort = items.every((it) => it.height < p.nozzleHeight);
	const half = allShort ? 0.5 * MAX_OUTER_NOZZLE_DIAMETER : 0.5 * p.radius;
	const limit = Math.max(0, 2 * half - 0.2);
	const close: string[] = [];
	for (let j = 1; j < items.length; j++)
		if (items.slice(0, j).some((a) => gap(a, items[j]) < limit)) close.push(items[j].id);
	const tall: string[] = [];
	items.forEach((it, k) => {
		let max = k === items.length - 1 ? p.printableHeight : p.heightToLid;
		const y1 = it.y - it.depth / 2;
		const y2 = it.y + it.depth / 2;
		for (const later of items.slice(k + 1)) {
			const inter =
				Math.min(y2, later.y + later.depth / 2) - Math.max(y1, later.y - later.depth / 2);
			if (inter > 0) {
				max = p.heightToRod;
				break;
			}
		}
		if (it.height > max + 1e-6) tall.push(it.id);
	});
	return { close, tall };
}
