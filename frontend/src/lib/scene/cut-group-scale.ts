// Échelle groupée des pièces liées par une coupe (lot 39 §3, Orca 2.4.1) :
// outil Échelle actif → ObjectList::fix_cut_selection étend la sélection d'un
// objet coupé à toutes les pièces de même `cut_id.id` ; la sélection multiple
// n'autorise que l'échelle uniforme ; Selection::scale_and_translate applique à
// chaque instance M' = T(p) · S(k) · T(−p) · M avec p = centre de la boîte
// englobante (monde) de TOUTE la sélection ; GLCanvas3D::do_scale repose
// ensuite chaque instance sur le plateau (min z → 0). Pur.
import { restingZ, type SceneMesh } from './mesh';
import { rotateVectorByEulerDeg } from './transform';

type Vec3 = [number, number, number];

/** Pièce coupée telle que la scène la connaît. */
export interface CutScalePiece {
	id: string;
	mesh: SceneMesh;
	position: Vec3;
	rotation: Vec3;
	scale: Vec3;
}

export interface Box3 {
	min: Vec3;
	max: Vec3;
}

/** Boîte englobante monde d'une pièce (échelle, rotation Euler XYZ, translation). */
export function pieceWorldBox(p: CutScalePiece): Box3 {
	const pos = p.mesh.positions;
	const min: Vec3 = [Infinity, Infinity, Infinity];
	const max: Vec3 = [-Infinity, -Infinity, -Infinity];
	const rotated = p.rotation[0] !== 0 || p.rotation[1] !== 0 || p.rotation[2] !== 0;
	for (let i = 0; i < pos.length; i += 3) {
		const s: Vec3 = [pos[i] * p.scale[0], pos[i + 1] * p.scale[1], pos[i + 2] * p.scale[2]];
		const r = rotated ? rotateVectorByEulerDeg(p.rotation, s) : s;
		for (let a = 0; a < 3; a++) {
			const w = r[a] + p.position[a];
			if (w < min[a]) min[a] = w;
			if (w > max[a]) max[a] = w;
		}
	}
	return { min, max };
}

/** Centre de la boîte englobante de la sélection (m_cache.dragging_center). */
export function selectionCenter(pieces: readonly CutScalePiece[]): Vec3 | null {
	const min: Vec3 = [Infinity, Infinity, Infinity];
	const max: Vec3 = [-Infinity, -Infinity, -Infinity];
	for (const p of pieces) {
		const b = pieceWorldBox(p);
		for (let a = 0; a < 3; a++) {
			min[a] = Math.min(min[a], b.min[a]);
			max[a] = Math.max(max[a], b.max[a]);
		}
	}
	if (!min.every(Number.isFinite) || !max.every(Number.isFinite)) return null;
	return [(min[0] + max[0]) / 2, (min[1] + max[1]) / 2, (min[2] + max[2]) / 2];
}

/** Facteur uniforme entre deux échelles (axe le plus modifié, cf. uniformCutScale). */
export function uniformFactor(previous: Readonly<Vec3>, next: Readonly<Vec3>): number {
	let ratio = 1;
	let best = 0;
	for (let a = 0; a < 3; a++) {
		const r = next[a] / (previous[a] || 1);
		const d = Math.abs(r - 1);
		if (d > best + 1e-12) {
			best = d;
			ratio = r;
		}
	}
	return ratio;
}

/**
 * Nouvelle position / échelle de chaque pièce pour un facteur uniforme `k`
 * autour du centre de la boîte de la sélection, puis pose sur le plateau.
 * Rotation inchangée (une échelle uniforme commute avec la rotation).
 */
export function groupCutScale(
	pieces: readonly CutScalePiece[],
	k: number,
	opts: { dropToBed?: boolean } = {}
): { id: string; position: Vec3; scale: Vec3 }[] {
	if (!(k > 0) || !Number.isFinite(k)) return [];
	const c = selectionCenter(pieces);
	if (!c) return [];
	const round = (v: number) => Math.round(v * 1e6) / 1e6;
	return pieces.map((p) => {
		const scale = p.scale.map((s) => round(s * k)) as Vec3;
		const position = p.position.map((v, a) => c[a] + k * (v - c[a])) as Vec3;
		if (opts.dropToBed !== false) position[2] = restingZ(p.mesh, p.rotation, scale);
		return { id: p.id, position, scale };
	});
}
