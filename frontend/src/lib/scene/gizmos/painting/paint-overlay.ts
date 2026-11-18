// Rendu de la peinture au triangle (supports, couture, fuzzy, MMU) dans la vue
// 3D, comme Orca (GLGizmoPainterBase::render_triangles, GLGizmoMmuSegmentation) :
// chaque chaîne hex 3MF est redécodée en sous-triangles feuilles, fidèlement à
// `TriangleSelector::deserialize` / `perform_split` (milieux d'arêtes, enfants
// lus en ordre inverse), puis colorée selon son état. Pur → testable.
import type { SceneMesh } from '../../mesh';

type Vec3 = [number, number, number];

/** Couleurs d'Orca (TriangleSelectorGUI::enforcers_color / blockers_color). */
export const PAINT_ENFORCER_COLOUR = '#80ff80';
export const PAINT_BLOCKER_COLOUR = '#ff8080';

/** Sous-triangle feuille peint (repère du maillage) et son état (> 0). */
export interface PaintedTriangle {
	vertices: [Vec3, Vec3, Vec3];
	state: number;
}

const mid = (a: Vec3, b: Vec3): Vec3 => [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2, (a[2] + b[2]) / 2];

/**
 * Feuilles peintes (état > 0) d'une facette 3MF Orca `hex` posée sur le
 * triangle source (a, b, c). Chaîne illisible → aucune feuille.
 */
export function decodeFacetTriangles(hex: string, a: Vec3, b: Vec3, c: Vec3): PaintedTriangle[] {
	if (!hex || !/^[0-9A-Fa-f]+$/.test(hex)) return [];
	// Quartets dans l'ordre du flux : la chaîne est bâtie en insérant en tête.
	const nibbles = hex
		.split('')
		.reverse()
		.map((ch) => parseInt(ch, 16));
	let pos = 0;
	const next = () => (pos < nibbles.length ? nibbles[pos++] : -1);
	const out: PaintedTriangle[] = [];

	const visit = (v: [Vec3, Vec3, Vec3], depth: number): boolean => {
		const code = next();
		if (code < 0 || depth > 32) return false;
		const split = code & 0b11;
		if (split === 0) {
			const state = (code & 0b1100) === 0b1100 ? next() + 3 : code >> 2;
			if (state < 0) return false;
			if (state > 0) out.push({ vertices: v, state });
			return true;
		}
		const s = code >> 2;
		// Sommets dans l'ordre du côté spécial (perform_split).
		const p0 = v[s % 3];
		const p1 = v[(s + 1) % 3];
		const p2 = v[(s + 2) % 3];
		let children: [Vec3, Vec3, Vec3][];
		if (split === 1) {
			const m = mid(p2, p1);
			children = [
				[p0, p1, m],
				[m, p2, p0]
			];
		} else if (split === 2) {
			const m01 = mid(p1, p0);
			const m02 = mid(p0, p2);
			children = [
				[p0, m01, m02],
				[m01, p1, m02],
				[p1, p2, m02]
			];
		} else {
			const m01 = mid(p1, p0);
			const m12 = mid(p2, p1);
			const m20 = mid(p0, p2);
			children = [
				[p0, m01, m20],
				[m01, p1, m12],
				[m12, p2, m20],
				[m01, m12, m20]
			];
		}
		// Le flux liste les enfants du dernier au premier (deserialize).
		for (let k = children.length - 1; k >= 0; k--) if (!visit(children[k], depth + 1)) return false;
		return true;
	};
	visit([a, b, c], 0);
	return out;
}

/** Géométrie à couleurs par sommet d'un canal peint (null : rien à afficher). */
export interface PaintOverlay {
	positions: Float32Array;
	colors: Float32Array;
}

/** `#rrggbb` → composantes 0..1 (null si invalide). */
export function hexToRgb(hex: string | null | undefined): [number, number, number] | null {
	const m = /^#?([0-9a-f]{6})$/i.exec(hex ?? '');
	if (!m) return null;
	const n = parseInt(m[1], 16);
	return [(n >> 16) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255];
}

/**
 * Surcouche d'un canal : un triangle par feuille peinte, couleur donnée par
 * `colourOf(state)` (null = feuille non affichée).
 */
export function paintOverlayGeometry(
	mesh: Pick<SceneMesh, 'positions' | 'indices'>,
	facets: Record<string, string> | undefined,
	colourOf: (state: number) => string | null
): PaintOverlay | null {
	if (!facets) return null;
	const pos: number[] = [];
	const col: number[] = [];
	const triCount = mesh.indices.length / 3;
	const vertex = (i: number): Vec3 => [
		mesh.positions[i * 3],
		mesh.positions[i * 3 + 1],
		mesh.positions[i * 3 + 2]
	];
	for (const [key, hex] of Object.entries(facets)) {
		const t = Number(key);
		if (!Number.isInteger(t) || t < 0 || t >= triCount) continue;
		const leaves = decodeFacetTriangles(
			hex,
			vertex(mesh.indices[t * 3]),
			vertex(mesh.indices[t * 3 + 1]),
			vertex(mesh.indices[t * 3 + 2])
		);
		for (const leaf of leaves) {
			const rgb = hexToRgb(colourOf(leaf.state));
			if (!rgb) continue;
			for (const v of leaf.vertices) {
				pos.push(v[0], v[1], v[2]);
				col.push(rgb[0], rgb[1], rgb[2]);
			}
		}
	}
	if (pos.length === 0) return null;
	return { positions: new Float32Array(pos), colors: new Float32Array(col) };
}
