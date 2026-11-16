// Gizmo Assembly d'Orca (`GLGizmoAssembly`, modes « face à face » et « point à
// point ») : la 2e entité cliquée est amenée sur la 1re. Face à face : l'objet
// de la 2e face tourne pour que sa normale soit opposée à la 1re, puis est
// translaté pour que les points cliqués coïncident. Point à point : translation
// seule. Pur, sans three.js (chunk initial de la page).
import { eulerDegFromQuaternion, quaternionFromUnitVectors } from '../transform';
import type { SceneMesh } from '../mesh';

export type Vec3 = [number, number, number];
type Quat = [number, number, number, number];

const DEG = Math.PI / 180;

/** Quaternion [x, y, z, w] d'une rotation Euler XYZ (degrés, convention three.js). */
export function quatFromEulerDeg([ex, ey, ez]: Vec3): Quat {
	const [c1, c2, c3] = [
		Math.cos((ex * DEG) / 2),
		Math.cos((ey * DEG) / 2),
		Math.cos((ez * DEG) / 2)
	];
	const [s1, s2, s3] = [
		Math.sin((ex * DEG) / 2),
		Math.sin((ey * DEG) / 2),
		Math.sin((ez * DEG) / 2)
	];
	return [
		s1 * c2 * c3 + c1 * s2 * s3,
		c1 * s2 * c3 - s1 * c2 * s3,
		c1 * c2 * s3 + s1 * s2 * c3,
		c1 * c2 * c3 - s1 * s2 * s3
	];
}

export function quatMul(a: Quat, b: Quat): Quat {
	const [ax, ay, az, aw] = a;
	const [bx, by, bz, bw] = b;
	return [
		ax * bw + aw * bx + ay * bz - az * by,
		ay * bw + aw * by + az * bx - ax * bz,
		az * bw + aw * bz + ax * by - ay * bx,
		aw * bw - ax * bx - ay * by - az * bz
	];
}

export function rotate(q: Quat, v: Vec3): Vec3 {
	const [x, y, z, w] = q;
	const ix = w * v[0] + y * v[2] - z * v[1];
	const iy = w * v[1] + z * v[0] - x * v[2];
	const iz = w * v[2] + x * v[1] - y * v[0];
	const iw = -x * v[0] - y * v[1] - z * v[2];
	return [
		ix * w + iw * -x + iy * -z - iz * -y,
		iy * w + iw * -y + iz * -x - ix * -z,
		iz * w + iw * -z + ix * -y - iy * -x
	];
}

const normalize = (v: Vec3): Vec3 => {
	const l = Math.hypot(...v) || 1;
	return [v[0] / l, v[1] / l, v[2] / l];
};

/** Normale monde d'un triangle (repère local du maillage, rotation/échelle d'instance). */
export function worldTriangleNormal(
	mesh: SceneMesh,
	triangle: number,
	rotation: Vec3,
	scale: Vec3
): Vec3 {
	const p = (k: number): Vec3 => {
		const i = mesh.indices[triangle * 3 + k] * 3;
		return [mesh.positions[i], mesh.positions[i + 1], mesh.positions[i + 2]];
	};
	const [a, b, c] = [p(0), p(1), p(2)];
	const u: Vec3 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
	const w: Vec3 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
	const n: Vec3 = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]];
	// Normales : transformées par l'inverse-transposée de R·S = R·S⁻¹.
	return normalize(
		rotate(quatFromEulerDeg(rotation), [n[0] / scale[0], n[1] / scale[1], n[2] / scale[2]])
	);
}

/**
 * Centre (monde) de la face plane contenant `triangle` : triangles connexes
 * (sommets partagés) coplanaires, centre de gravité de leurs sommets. Orca
 * `Measure` : point d'une `SurfaceFeature` plan = cog des bords du plan, et
 * l'assemblage face à face fait coïncider ces centres (`set_to_center_coincidence`),
 * pas les points cliqués.
 */
export function worldFaceCenter(
	mesh: SceneMesh,
	triangle: number,
	position: Vec3,
	rotation: Vec3,
	scale: Vec3
): Vec3 {
	const pos = (v: number): Vec3 => [
		mesh.positions[v * 3],
		mesh.positions[v * 3 + 1],
		mesh.positions[v * 3 + 2]
	];
	const plane = (t: number): [Vec3, number] => {
		const [a, b, c] = [0, 1, 2].map((k) => pos(mesh.indices[t * 3 + k]));
		const n = normalize([
			(b[1] - a[1]) * (c[2] - a[2]) - (b[2] - a[2]) * (c[1] - a[1]),
			(b[2] - a[2]) * (c[0] - a[0]) - (b[0] - a[0]) * (c[2] - a[2]),
			(b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
		]);
		return [n, n[0] * a[0] + n[1] * a[1] + n[2] * a[2]];
	};
	const [n0, d0] = plane(triangle);
	const triangles = mesh.indices.length / 3;
	// Sommets → triangles, par position (maillages à sommets dupliqués par face
	// compris) : adjacence par sommet partagé.
	const key = (v: number) =>
		pos(v)
			.map((x) => Math.round(x * 1e4))
			.join(',');
	const byVertex = new Map<string, number[]>();
	for (let t = 0; t < triangles; t++)
		for (let k = 0; k < 3; k++) {
			const v = key(mesh.indices[t * 3 + k]);
			const list = byVertex.get(v);
			if (list) list.push(t);
			else byVertex.set(v, [t]);
		}
	const seen = new Set<number>([triangle]);
	const vertices = new Map<string, number>();
	const stack = [triangle];
	while (stack.length) {
		const t = stack.pop()!;
		for (let k = 0; k < 3; k++) {
			const index = mesh.indices[t * 3 + k];
			const v = key(index);
			vertices.set(v, index);
			for (const u of byVertex.get(v) ?? []) {
				if (seen.has(u)) continue;
				const [n, d] = plane(u);
				if (n[0] * n0[0] + n[1] * n0[1] + n[2] * n0[2] > 1 - 1e-6 && Math.abs(d - d0) < 1e-4) {
					seen.add(u);
					stack.push(u);
				}
			}
		}
	}
	const c: Vec3 = [0, 0, 0];
	for (const v of vertices.values()) {
		const p = pos(v);
		c[0] += p[0] / vertices.size;
		c[1] += p[1] / vertices.size;
		c[2] += p[2] / vertices.size;
	}
	const w = rotate(quatFromEulerDeg(rotation), [c[0] * scale[0], c[1] * scale[1], c[2] * scale[2]]);
	return [position[0] + w[0], position[1] + w[1], position[2] + w[2]];
}

export interface Placement {
	position: Vec3;
	rotation: Vec3;
}

/** Point à point : l'objet mobile est translaté de `target − picked`. */
export function alignPoints(position: Vec3, picked: Vec3, target: Vec3): Vec3 {
	return [
		position[0] + target[0] - picked[0],
		position[1] + target[1] - picked[1],
		position[2] + target[2] - picked[2]
	];
}

/**
 * Face à face : rotation de l'objet mobile (normale `movingNormal` → −`targetNormal`)
 * autour de sa position, puis translation pour amener `picked` sur `target`.
 */
export function alignFaces(
	moving: Placement,
	picked: Vec3,
	movingNormal: Vec3,
	target: Vec3,
	targetNormal: Vec3
): Placement {
	const turn = quaternionFromUnitVectors(movingNormal, [
		-targetNormal[0],
		-targetNormal[1],
		-targetNormal[2]
	]) as Quat;
	const rotation = eulerDegFromQuaternion(quatMul(turn, quatFromEulerDeg(moving.rotation)));
	const rel: Vec3 = [
		picked[0] - moving.position[0],
		picked[1] - moving.position[1],
		picked[2] - moving.position[2]
	];
	const turned = rotate(turn, rel);
	const pickedAfter: Vec3 = [
		moving.position[0] + turned[0],
		moving.position[1] + turned[1],
		moving.position[2] + turned[2]
	];
	return {
		rotation: rotation.map((r) => Math.round(r * 1e6) / 1e6) as Vec3,
		position: alignPoints(moving.position, pickedAfter, target)
	};
}
