// Export STL binaire depuis la scène (menu Fichier › Export all objects as
// one STL / as STLs) : maillages transformés en repère plateau (échelle, puis
// rotation Euler XYZ, puis position — même ordre que Three.js), normales de
// facette réelles (pas d'en-tête « solid », lu comme ASCII par admesh).
import type { SceneObject } from '../scene/mesh';
import { rotateVectorByEulerDeg } from '../scene/transform';

type Vec3 = [number, number, number];

export interface Triangles {
	/** Sommets (x,y,z) monde, 9 valeurs par triangle. */
	points: Float32Array;
}

/** Triangles monde d'un objet de scène (ordre inversé si miroir). */
export function worldTriangles(object: SceneObject): Triangles {
	const { positions, indices } = object.mesh;
	const position = object.position ?? [0, 0, 0];
	const rotation = object.rotation ?? [0, 0, 0];
	const scale = object.scale ?? [1, 1, 1];
	const mirrored = scale[0] * scale[1] * scale[2] < 0;
	const rotated = rotation.some((r) => r !== 0);
	const world = (i: number): Vec3 => {
		const v: Vec3 = [
			positions[i * 3] * scale[0],
			positions[i * 3 + 1] * scale[1],
			positions[i * 3 + 2] * scale[2]
		];
		const r = rotated ? rotateVectorByEulerDeg(rotation as Vec3, v) : v;
		return [r[0] + position[0], r[1] + position[1], r[2] + position[2]];
	};
	const count = indices.length / 3;
	const points = new Float32Array(count * 9);
	for (let t = 0; t < count; t++) {
		const a = indices[t * 3];
		const b = mirrored ? indices[t * 3 + 2] : indices[t * 3 + 1];
		const c = mirrored ? indices[t * 3 + 1] : indices[t * 3 + 2];
		[a, b, c].forEach((index, k) => points.set(world(index), t * 9 + k * 3));
	}
	return { points };
}

/** Encode des triangles monde en STL binaire (normales de facette unitaires). */
export function encodeBinaryStl(parts: Triangles[], header = 'web-slicer STL export'): Uint8Array {
	const total = parts.reduce((n, p) => n + p.points.length / 9, 0);
	const out = new Uint8Array(84 + total * 50);
	const view = new DataView(out.buffer);
	const tag = new TextEncoder().encode(header.replace(/^solid/i, 'STL')).slice(0, 80);
	out.fill(0x20, 0, 80);
	out.set(tag, 0);
	view.setUint32(80, total, true);
	let at = 84;
	for (const { points: p } of parts) {
		for (let t = 0; t < p.length; t += 9) {
			const u = [p[t + 3] - p[t], p[t + 4] - p[t + 1], p[t + 5] - p[t + 2]];
			const w = [p[t + 6] - p[t], p[t + 7] - p[t + 1], p[t + 8] - p[t + 2]];
			let n = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]];
			const len = Math.hypot(n[0], n[1], n[2]);
			n = len > 1e-12 ? n.map((x) => x / len) : [0, 0, 0];
			for (const value of [...n, ...p.subarray(t, t + 9)]) {
				view.setFloat32(at, value, true);
				at += 4;
			}
			view.setUint16(at, 0, true);
			at += 2;
		}
	}
	return out;
}

/** Nom de fichier sûr (caractères interdits remplacés). */
export function safeFileName(name: string, extension: string): string {
	const base =
		name
			.replace(/\.[a-z0-9]+$/i, '')
			.replace(/[\\/:*?"<>|]+/g, '_')
			.trim() || 'objet';
	return `${base}.${extension}`;
}
