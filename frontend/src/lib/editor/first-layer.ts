// Phase 46 (audit 44 PERF-a) — « One object has an empty first layer… » :
// Orca nomme l'objet fautif (StringObjectException + lien « Sélectionner ») ;
// le worker ne renvoie que le texte. On retrouve ici, côté client, les objets
// du plateau dont la section à mi-hauteur de la première couche est plus
// étroite qu'une ligne d'extrusion (pointe, arête posée sur le plateau).
import { rotateVectorByEulerDeg } from '#lib/scene/transform';
import type { SceneMesh } from '#lib/scene/mesh';

type Vec3 = [number, number, number];

/** Message d'échec « couche initiale vide » (anglais d'Orca ou traduction). */
export function isEmptyFirstLayerError(message: string | null | undefined): boolean {
	return !!message && /empty (first|initial) layer|couche initiale vide/i.test(message);
}

/**
 * Emprise XY (largeur, profondeur) de la section de l'objet posé à la hauteur
 * `z` au-dessus de son point le plus bas ; `null` si le plan ne coupe rien.
 */
export function sectionExtent(
	mesh: SceneMesh,
	z: number,
	rotation: Vec3 = [0, 0, 0],
	scale: Vec3 = [1, 1, 1]
): { width: number; depth: number } | null {
	const p = mesh.positions;
	const n = p.length / 3;
	if (n === 0) return null;
	const rotated = rotation.some((r) => r !== 0);
	const w = new Float64Array(p.length);
	let minZ = Infinity;
	for (let i = 0; i < n; i++) {
		const v: Vec3 = [p[3 * i] * scale[0], p[3 * i + 1] * scale[1], p[3 * i + 2] * scale[2]];
		const r = rotated ? rotateVectorByEulerDeg(rotation, v) : v;
		w[3 * i] = r[0];
		w[3 * i + 1] = r[1];
		w[3 * i + 2] = r[2];
		if (r[2] < minZ) minZ = r[2];
	}
	const plane = minZ + z;
	let x0 = Infinity,
		x1 = -Infinity,
		y0 = Infinity,
		y1 = -Infinity;
	const idx = mesh.indices;
	const edge = (a: number, b: number) => {
		const za = w[3 * a + 2] - plane;
		const zb = w[3 * b + 2] - plane;
		if ((za > 0 && zb > 0) || (za < 0 && zb < 0) || za === zb) return;
		const t = za / (za - zb);
		const x = w[3 * a] + t * (w[3 * b] - w[3 * a]);
		const y = w[3 * a + 1] + t * (w[3 * b + 1] - w[3 * a + 1]);
		x0 = Math.min(x0, x);
		x1 = Math.max(x1, x);
		y0 = Math.min(y0, y);
		y1 = Math.max(y1, y);
	};
	for (let i = 0; i + 2 < idx.length; i += 3) {
		edge(idx[i], idx[i + 1]);
		edge(idx[i + 1], idx[i + 2]);
		edge(idx[i + 2], idx[i]);
	}
	if (!Number.isFinite(x0)) return null;
	return { width: x1 - x0, depth: y1 - y0 };
}

/**
 * Noms des objets dont la première couche serait vide : section à mi-hauteur
 * de `firstLayer` absente ou plus étroite que `lineWidth` dans une direction.
 */
export function emptyFirstLayerSuspects(
	objects: readonly { name: string; mesh: SceneMesh; rotation?: Vec3; scale?: Vec3 }[],
	firstLayer: number,
	lineWidth: number
): string[] {
	const h = firstLayer > 0 ? firstLayer / 2 : 0.1;
	return objects
		.filter((o) => {
			const s = sectionExtent(o.mesh, h, o.rotation, o.scale);
			return !s || Math.min(s.width, s.depth) < lineWidth;
		})
		.map((o) => o.name);
}
