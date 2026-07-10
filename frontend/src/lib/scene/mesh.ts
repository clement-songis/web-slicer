// Décodage du maillage binaire « WSMh » servi par le backend (T049) en
// tableaux typés prêts pour une BufferGeometry Three.js. Pur → testable.
//
// Format little-endian : magic(4)=WSMh · version(u32)=1 · vertex_count(u32) ·
// index_count(u32) · f32 positions[vc*3] · f32 normals[vc*3] · u32 indices[ic].

/** Maillage prêt pour Three.js (attributs plats + indices). */
export interface SceneMesh {
	positions: Float32Array;
	normals: Float32Array;
	indices: Uint32Array;
}

/** Un objet placé sur le plateau : maillage + position (mm, repère plateau). */
export interface SceneObject {
	id: string;
	mesh: SceneMesh;
	position?: [number, number, number];
}

const MAGIC = 0x574d_5368; // "WSMh" en big-endian pour comparaison directe

/** Décode un tampon WSMh. Lève si l'en-tête ou la taille est incohérent. */
export function decodeMesh(buffer: ArrayBuffer): SceneMesh {
	if (buffer.byteLength < 16) throw new Error('maillage tronqué (en-tête)');
	const view = new DataView(buffer);
	if (view.getUint32(0, false) !== MAGIC) throw new Error('en-tête de maillage invalide');
	if (view.getUint32(4, true) !== 1) throw new Error('version de maillage non supportée');

	const vertexCount = view.getUint32(8, true);
	const indexCount = view.getUint32(12, true);
	const floats = vertexCount * 3;
	const expected = 16 + floats * 4 * 2 + indexCount * 4;
	if (buffer.byteLength !== expected) throw new Error('maillage tronqué');

	let off = 16;
	const positions = new Float32Array(buffer.slice(off, off + floats * 4));
	off += floats * 4;
	const normals = new Float32Array(buffer.slice(off, off + floats * 4));
	off += floats * 4;
	const indices = new Uint32Array(buffer.slice(off, off + indexCount * 4));

	return { positions, normals, indices };
}
