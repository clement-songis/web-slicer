// Outil de simplification (T055) : décimation par regroupement de sommets sur
// une grille (vertex clustering). Réduit le nombre de triangles en soudant les
// sommets proches et en supprimant les facettes devenues dégénérées. Pur →
// testable. La simplification préservant les features (quadric) est backlog FFI.
import { meshFromTriangleSoup } from '../loaders';
import type { SceneMesh } from '../mesh';

type Vec3 = [number, number, number];

/** Soude un sommet sur une grille de pas `cell` (renvoie le centre de cellule). */
function snap(p: Vec3, cell: number): Vec3 {
	return [
		Math.round(p[0] / cell) * cell,
		Math.round(p[1] / cell) * cell,
		Math.round(p[2] / cell) * cell
	];
}

/**
 * Simplifie un maillage par regroupement de sommets sur une grille de taille
 * `cell` (mm). Les triangles dont deux sommets tombent dans la même cellule
 * sont supprimés (dégénérés). `cell <= 0` renvoie le maillage inchangé.
 */
export function simplifyGrid(mesh: SceneMesh, cell: number): SceneMesh {
	if (cell <= 0) return mesh;
	const positions: number[] = [];
	const key = (p: Vec3) => `${p[0]},${p[1]},${p[2]}`;
	const triangles = mesh.indices.length / 3;
	for (let t = 0; t < triangles; t++) {
		const verts: Vec3[] = [];
		for (let k = 0; k < 3; k++) {
			const i = mesh.indices[t * 3 + k];
			verts.push(
				snap([mesh.positions[i * 3], mesh.positions[i * 3 + 1], mesh.positions[i * 3 + 2]], cell)
			);
		}
		// Rejette les facettes dégénérées (deux sommets soudés dans la même cellule).
		const [a, b, c] = verts;
		if (key(a) === key(b) || key(b) === key(c) || key(a) === key(c)) continue;
		positions.push(...a, ...b, ...c);
	}
	return meshFromTriangleSoup(positions);
}

/** Nombre de triangles d'un maillage. */
export function triangleCount(mesh: SceneMesh): number {
	return mesh.indices.length / 3;
}
