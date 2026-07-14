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

/** Un objet placé sur le plateau : maillage + transformation (repère plateau). */
export interface SceneObject {
	id: string;
	mesh: SceneMesh;
	/** Position (mm). */
	position?: [number, number, number];
	/** Rotation d'Euler (degrés), appliquée par le gizmo de transformation (T103). */
	rotation?: [number, number, number];
	/** Échelle par axe (1 = taille native). */
	scale?: [number, number, number];
}

/**
 * Recentre un maillage sur le centre de sa boîte englobante et renvoie ce centre.
 * Sert à faire coïncider l'origine locale de l'objet avec son centre visuel :
 * le groupe Three.js est alors posé à `center`, et le gizmo de transformation
 * (attaché à l'origine du groupe) apparaît au centre de l'objet, pas au coin du
 * plateau — cas des maillages importés dont la géométrie est décalée (ex. 3MF
 * dont la transformation de plateau est cuite dans les sommets). Pur.
 */
export function centerMesh(mesh: SceneMesh): { mesh: SceneMesh; center: [number, number, number] } {
	const p = mesh.positions;
	if (p.length === 0) return { mesh, center: [0, 0, 0] };
	let minX = Infinity,
		minY = Infinity,
		minZ = Infinity,
		maxX = -Infinity,
		maxY = -Infinity,
		maxZ = -Infinity;
	for (let i = 0; i < p.length; i += 3) {
		const x = p[i],
			y = p[i + 1],
			z = p[i + 2];
		if (x < minX) minX = x;
		if (x > maxX) maxX = x;
		if (y < minY) minY = y;
		if (y > maxY) maxY = y;
		if (z < minZ) minZ = z;
		if (z > maxZ) maxZ = z;
	}
	const center: [number, number, number] = [
		(minX + maxX) / 2,
		(minY + maxY) / 2,
		(minZ + maxZ) / 2
	];
	const out = new Float32Array(p.length);
	for (let i = 0; i < p.length; i += 3) {
		out[i] = p[i] - center[0];
		out[i + 1] = p[i + 1] - center[1];
		out[i + 2] = p[i + 2] - center[2];
	}
	return { mesh: { positions: out, normals: mesh.normals, indices: mesh.indices }, center };
}

// "WSMh" (octets 0x57 0x53 0x4d 0x68) lu en big-endian = 0x57534d68 — doit
// correspondre au magic écrit par le backend (`backend/src/mesh.rs`, b"WSMh").
const MAGIC = 0x5753_4d68;

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
