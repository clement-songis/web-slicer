// Encodeur STL binaire (T140) : sérialise un `SceneMesh` indexé en STL binaire
// standard, afin de téléverser une primitive paramétrique comme modèle serveur
// (elle rejoint alors le pipeline des imports : maillage `/mesh`, restauration à
// l'ouverture et tranchage `build_stl_model`). Pur → testable.
//
// Format binaire : en-tête 80 octets · u32 nombre de triangles · par triangle
// 12 f32 (normale xyz + 3 sommets xyz) + u16 attribut (0) = 50 octets.
import type { SceneMesh } from './mesh';

/** Normale unitaire d'un triangle (produit vectoriel), `[0,0,0]` si dégénéré. */
function faceNormal(
	ax: number,
	ay: number,
	az: number,
	bx: number,
	by: number,
	bz: number,
	cx: number,
	cy: number,
	cz: number
): [number, number, number] {
	const ux = bx - ax,
		uy = by - ay,
		uz = bz - az;
	const vx = cx - ax,
		vy = cy - ay,
		vz = cz - az;
	const nx = uy * vz - uz * vy;
	const ny = uz * vx - ux * vz;
	const nz = ux * vy - uy * vx;
	const len = Math.hypot(nx, ny, nz);
	if (len === 0) return [0, 0, 0];
	return [nx / len, ny / len, nz / len];
}

/** Sérialise un maillage indexé en STL binaire. */
export function sceneMeshToStlBytes(mesh: SceneMesh): Uint8Array {
	const { positions, indices } = mesh;
	const triangleCount = Math.floor(indices.length / 3);
	const buffer = new ArrayBuffer(84 + triangleCount * 50);
	const view = new DataView(buffer);
	// En-tête (80 octets) laissé à zéro, puis le compte de triangles.
	view.setUint32(80, triangleCount, true);

	let offset = 84;
	for (let t = 0; t < triangleCount; t++) {
		const i0 = indices[t * 3] * 3;
		const i1 = indices[t * 3 + 1] * 3;
		const i2 = indices[t * 3 + 2] * 3;
		const [nx, ny, nz] = faceNormal(
			positions[i0],
			positions[i0 + 1],
			positions[i0 + 2],
			positions[i1],
			positions[i1 + 1],
			positions[i1 + 2],
			positions[i2],
			positions[i2 + 1],
			positions[i2 + 2]
		);
		view.setFloat32(offset, nx, true);
		view.setFloat32(offset + 4, ny, true);
		view.setFloat32(offset + 8, nz, true);
		let o = offset + 12;
		for (const base of [i0, i1, i2]) {
			view.setFloat32(o, positions[base], true);
			view.setFloat32(o + 4, positions[base + 1], true);
			view.setFloat32(o + 8, positions[base + 2], true);
			o += 12;
		}
		view.setUint16(offset + 48, 0, true); // attribut
		offset += 50;
	}
	return new Uint8Array(buffer);
}

/** Enveloppe le maillage STL dans un `File` prêt pour `uploadModel`. */
export function sceneMeshToStlFile(mesh: SceneMesh, name: string): File {
	const bytes = sceneMeshToStlBytes(mesh);
	const filename = name.toLowerCase().endsWith('.stl') ? name : `${name}.stl`;
	return new File([bytes.buffer as ArrayBuffer], filename, { type: 'model/stl' });
}
