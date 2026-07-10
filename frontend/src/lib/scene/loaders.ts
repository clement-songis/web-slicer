// Loaders client STL/OBJ/3MF (T051, décision R7 : aperçu immédiat côté client
// pendant que l'upload part en tâche de fond). Les parseurs sont purs et
// produisent un `SceneMesh` facetté (positions/normales/index) directement
// consommable par ModelObject.svelte — donc unit-testables sous bun.
import { unzipSync, strFromU8 } from 'fflate';
import { API_BASE, ApiError } from '../api/client';
import type { ModelResponse, ErrorBody } from '../api/types';
import { decodeMesh, type SceneMesh } from './mesh';

/** Formats dont on sait produire un aperçu client (le STEP passe par le moteur). */
export type PreviewFormat = 'stl' | 'obj' | '3mf';

/** Déduit le format d'aperçu depuis l'extension du nom de fichier. */
export function previewFormat(filename: string): PreviewFormat | null {
	const ext = filename.split('.').pop()?.toLowerCase();
	if (ext === 'stl') return 'stl';
	if (ext === 'obj') return 'obj';
	if (ext === '3mf') return '3mf';
	return null;
}

// --- Assemblage : soupe de triangles → SceneMesh facetté ---------------------

/**
 * Construit un `SceneMesh` non indexé à partir de positions de triangles
 * (9 floats par triangle). Les normales sont calculées par face (aspect
 * facetté volontaire pour l'aperçu) et les index sont séquentiels.
 */
export function meshFromTriangleSoup(positions: number[]): SceneMesh {
	const count = positions.length / 3; // nombre de sommets
	const normals = new Float32Array(positions.length);
	for (let i = 0; i < positions.length; i += 9) {
		const ax = positions[i],
			ay = positions[i + 1],
			az = positions[i + 2];
		const bx = positions[i + 3],
			by = positions[i + 4],
			bz = positions[i + 5];
		const cx = positions[i + 6],
			cy = positions[i + 7],
			cz = positions[i + 8];
		// Normale = (b-a) × (c-a), normalisée.
		const ux = bx - ax,
			uy = by - ay,
			uz = bz - az;
		const vx = cx - ax,
			vy = cy - ay,
			vz = cz - az;
		let nx = uy * vz - uz * vy;
		let ny = uz * vx - ux * vz;
		let nz = ux * vy - uy * vx;
		const len = Math.hypot(nx, ny, nz) || 1;
		nx /= len;
		ny /= len;
		nz /= len;
		for (let k = 0; k < 9; k += 3) {
			normals[i + k] = nx;
			normals[i + k + 1] = ny;
			normals[i + k + 2] = nz;
		}
	}
	const indices = new Uint32Array(count);
	for (let i = 0; i < count; i++) indices[i] = i;
	return { positions: new Float32Array(positions), normals, indices };
}

// --- STL (binaire + ASCII) ---------------------------------------------------

/** Vrai si le tampon STL est au format binaire (heuristique de la taille). */
function isBinaryStl(buffer: ArrayBuffer): boolean {
	if (buffer.byteLength < 84) return false;
	const count = new DataView(buffer).getUint32(80, true);
	return buffer.byteLength === 84 + count * 50;
}

/** Parse un STL (binaire ou ASCII) en maillage facetté. */
export function parseStl(buffer: ArrayBuffer): SceneMesh {
	return isBinaryStl(buffer) ? parseBinaryStl(buffer) : parseAsciiStl(buffer);
}

function parseBinaryStl(buffer: ArrayBuffer): SceneMesh {
	const view = new DataView(buffer);
	const count = view.getUint32(80, true);
	const positions: number[] = [];
	let off = 84;
	for (let t = 0; t < count; t++) {
		off += 12; // saute la normale de face stockée
		for (let v = 0; v < 3; v++) {
			positions.push(view.getFloat32(off, true));
			positions.push(view.getFloat32(off + 4, true));
			positions.push(view.getFloat32(off + 8, true));
			off += 12;
		}
		off += 2; // octets d'attribut
	}
	return meshFromTriangleSoup(positions);
}

function parseAsciiStl(buffer: ArrayBuffer): SceneMesh {
	const text = new TextDecoder().decode(buffer);
	const positions: number[] = [];
	const re = /vertex\s+(-?[\d.eE+]+)\s+(-?[\d.eE+]+)\s+(-?[\d.eE+]+)/g;
	let m: RegExpExecArray | null;
	while ((m = re.exec(text)) !== null) {
		positions.push(Number(m[1]), Number(m[2]), Number(m[3]));
	}
	if (positions.length % 9 !== 0) {
		throw new Error('STL ASCII invalide (nombre de sommets non multiple de 3)');
	}
	return meshFromTriangleSoup(positions);
}

// --- OBJ ---------------------------------------------------------------------

/** Parse un OBJ (sommets `v`, faces `f` triangulées en éventail). */
export function parseObj(text: string): SceneMesh {
	const verts: number[][] = [];
	const positions: number[] = [];
	for (const raw of text.split('\n')) {
		const line = raw.trim();
		if (line.startsWith('v ')) {
			const p = line.slice(2).trim().split(/\s+/).map(Number);
			verts.push([p[0], p[1], p[2]]);
		} else if (line.startsWith('f ')) {
			const tokens = line.slice(2).trim().split(/\s+/);
			// Index de sommet = premier champ du token `v/vt/vn`, 1-based (ou négatif relatif).
			const idx = tokens.map((tok) => {
				let i = Number(tok.split('/')[0]);
				if (i < 0) i = verts.length + i + 1;
				return i - 1;
			});
			// Triangulation en éventail pour les polygones.
			for (let k = 1; k + 1 < idx.length; k++) {
				for (const vi of [idx[0], idx[k], idx[k + 1]]) {
					const v = verts[vi];
					if (!v) throw new Error(`OBJ invalide (index de face hors limites : ${vi + 1})`);
					positions.push(v[0], v[1], v[2]);
				}
			}
		}
	}
	return meshFromTriangleSoup(positions);
}

// --- 3MF (unzip + XML du modèle) ---------------------------------------------

/**
 * Parse un 3MF : dézippe, lit `3D/3dmodel.model` et combine tous les maillages
 * `<mesh>` rencontrés. Extraction par regex (pas de DOMParser → testable sous
 * bun). Les transformations d'assemblage sont ignorées pour l'aperçu.
 */
export function parse3mf(buffer: ArrayBuffer): SceneMesh {
	const files = unzipSync(new Uint8Array(buffer));
	const key = Object.keys(files).find((k) => k.toLowerCase().endsWith('.model'));
	if (!key) throw new Error('3MF invalide (modèle 3D absent)');
	const xml = strFromU8(files[key]);

	const positions: number[] = [];
	const meshRe = /<mesh\b[\s\S]*?<\/mesh>/g;
	let meshMatch: RegExpExecArray | null;
	while ((meshMatch = meshRe.exec(xml)) !== null) {
		const block = meshMatch[0];
		const verts: number[][] = [];
		const vRe =
			/<vertex\b[^>]*\bx="(-?[\d.eE+]+)"[^>]*\by="(-?[\d.eE+]+)"[^>]*\bz="(-?[\d.eE+]+)"/g;
		let v: RegExpExecArray | null;
		while ((v = vRe.exec(block)) !== null) {
			verts.push([Number(v[1]), Number(v[2]), Number(v[3])]);
		}
		const tRe = /<triangle\b[^>]*\bv1="(\d+)"[^>]*\bv2="(\d+)"[^>]*\bv3="(\d+)"/g;
		let t: RegExpExecArray | null;
		while ((t = tRe.exec(block)) !== null) {
			for (const vi of [Number(t[1]), Number(t[2]), Number(t[3])]) {
				const p = verts[vi];
				if (!p) throw new Error(`3MF invalide (index de triangle hors limites : ${vi})`);
				positions.push(p[0], p[1], p[2]);
			}
		}
	}
	if (positions.length === 0) throw new Error('3MF sans géométrie exploitable');
	return meshFromTriangleSoup(positions);
}

// --- Dispatch d'aperçu --------------------------------------------------------

/** Produit un aperçu maillé depuis un fichier local, selon son extension. */
export async function loadPreview(file: File): Promise<SceneMesh> {
	const fmt = previewFormat(file.name);
	if (!fmt) throw new Error(`format non prévisualisable : ${file.name}`);
	const buffer = await file.arrayBuffer();
	if (fmt === 'stl') return parseStl(buffer);
	if (fmt === 'obj') return parseObj(new TextDecoder().decode(buffer));
	return parse3mf(buffer);
}

// --- Upload en tâche de fond + récupération du maillage backend ---------------

/**
 * Upload multipart d'un modèle (tâche de fond, T048). Le client construit
 * l'aperçu localement pendant que cet appel s'exécute.
 */
export async function uploadModel(projectId: string, file: File): Promise<ModelResponse> {
	const form = new FormData();
	form.append('file', file, file.name);
	const res = await fetch(`${API_BASE}/projects/${projectId}/models`, {
		method: 'POST',
		credentials: 'include',
		body: form
	});
	const text = await res.text();
	const data: unknown = text ? JSON.parse(text) : undefined;
	if (!res.ok) {
		const err = (data ?? {}) as Partial<ErrorBody>;
		throw new ApiError(res.status, err.code ?? 'error', err.message ?? res.statusText, err.details);
	}
	return data as ModelResponse;
}

/** Récupère le maillage WSMh servi par le backend (`GET …/mesh`) et le décode. */
export async function fetchMesh(modelId: string): Promise<SceneMesh> {
	const res = await fetch(`${API_BASE}/models/${modelId}/mesh`, { credentials: 'include' });
	if (!res.ok) {
		const text = await res.text();
		const err = (text ? JSON.parse(text) : {}) as Partial<ErrorBody>;
		throw new ApiError(res.status, err.code ?? 'error', err.message ?? res.statusText, err.details);
	}
	return decodeMesh(await res.arrayBuffer());
}
