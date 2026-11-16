// Surplombs du menu Vue › Show Overhang (miroir de GLCanvas3D/gouraud.fs
// d'Orca) : une facette est en surplomb si la composante Z de sa normale
// monde est sous `slope.normal_z`, seuil dérivé des réglages de supports
// (`GLVolumeCollection::get_selection_support_normal_z`). Les facettes posées
// sur le plateau (|z| < 0,1 mm) sont exclues (Orca les teinte en bleu).
import type { SceneMesh } from './mesh';
import { rotateVectorByEulerDeg } from './transform';

type Vec3 = [number, number, number];
const DEG = Math.PI / 180;
const EPSILON = 0.0001;

function num(value: unknown): number | null {
	if (Array.isArray(value)) return num(value[0]);
	const n = typeof value === 'number' ? value : Number.parseFloat(String(value ?? ''));
	return Number.isFinite(n) ? n : null;
}

/** Valeur absolue d'une option « float ou pourcentage » (`get_abs_value`). */
function absValue(value: unknown, base: number): number | null {
	if (typeof value === 'string' && value.trim().endsWith('%')) {
		const pct = num(value.trim().slice(0, -1));
		return pct === null ? null : (pct / 100) * base;
	}
	return num(value);
}

/** `slope.normal_z` d'Orca depuis les valeurs résolues (process + machine). */
export function supportNormalZ(values: Record<string, unknown>): number {
	const threshold = num(values.support_threshold_angle) ?? 0;
	const supportType = String(values.support_type ?? '');
	let angle: number;
	if (threshold > 0) {
		angle = Math.min(threshold + 1, 89) * DEG;
	} else if (supportType.startsWith('tree')) {
		angle = 30 * DEG;
	} else {
		const layerHeight = num(values.layer_height) ?? 0.2;
		const nozzle = num(values.nozzle_diameter) ?? 0.4;
		let width = absValue(values.outer_wall_line_width, nozzle) ?? 0;
		if (width <= 0) width = absValue(values.line_width, nozzle) ?? 0;
		if (width <= 0) width = nozzle;
		const overlap = absValue(values.support_threshold_overlap ?? '50%', width) ?? width / 2;
		const offset = Math.max(0, width - overlap);
		angle = offset <= EPSILON ? 89 * DEG : Math.atan(layerHeight / offset);
	}
	return -Math.cos(Math.min(Math.max(angle, 0), 89 * DEG));
}

/**
 * Index (triplets) des facettes en surplomb d'un maillage placé (échelle,
 * rotation Euler XYZ en degrés, position monde) pour le seuil `normalZ`.
 */
export function overhangIndices(
	mesh: SceneMesh,
	normalZ: number,
	rotation: Vec3 = [0, 0, 0],
	scale: Vec3 = [1, 1, 1],
	position: Vec3 = [0, 0, 0]
): Uint32Array {
	const { positions: p, indices } = mesh;
	const world = (i: number): Vec3 => {
		const v = rotateVectorByEulerDeg(rotation, [
			p[i * 3] * scale[0],
			p[i * 3 + 1] * scale[1],
			p[i * 3 + 2] * scale[2]
		]);
		return [v[0] + position[0], v[1] + position[1], v[2] + position[2]];
	};
	const mirrored = scale[0] * scale[1] * scale[2] < 0;
	const out: number[] = [];
	for (let t = 0; t < indices.length; t += 3) {
		const a = world(indices[t]);
		const b = world(indices[t + 1]);
		const c = world(indices[t + 2]);
		if (Math.abs(a[2]) < 0.1 && Math.abs(b[2]) < 0.1 && Math.abs(c[2]) < 0.1) continue;
		const u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
		const w = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
		const n = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]];
		const len = Math.hypot(n[0], n[1], n[2]);
		if (len < 1e-12) continue;
		const nz = ((mirrored ? -1 : 1) * n[2]) / len;
		if (nz < normalZ - EPSILON) out.push(indices[t], indices[t + 1], indices[t + 2]);
	}
	return Uint32Array.from(out);
}
