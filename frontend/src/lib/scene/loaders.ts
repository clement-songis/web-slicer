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
	// `.oltp` est un alias STL chez OrcaSlicer (Model.cpp).
	if (ext === 'stl' || ext === 'oltp') return 'stl';
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

/** Accumule dans `positions` les triangles de tous les `<mesh>` d'un modèle XML. */
function collect3mfMeshes(xml: string, positions: number[]): void {
	const meshRe = /<mesh\b[\s\S]*?<\/mesh>/g;
	let meshMatch: RegExpExecArray | null;
	while ((meshMatch = meshRe.exec(xml)) !== null) {
		const block = meshMatch[0];
		const verts: number[][] = [];
		// Valeurs capturées en bloc (`[^"]+`) puis converties par `Number` : robuste
		// à la notation scientifique à exposant négatif (`-5.57e-14`) qu'OrcaSlicer
		// émet couramment, que `[\d.eE+]` tronquait au `-` de l'exposant.
		const vRe = /<vertex\b[^>]*\bx="([^"]+)"[^>]*\by="([^"]+)"[^>]*\bz="([^"]+)"/g;
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
}

// Transformation affine 3mf : 12 nombres (matrice 4×3 ligne-majeur, dernière
// colonne implicite [0,0,0,1]) ; un point est un vecteur ligne `p·M`.
type Mat = number[];
const IDENTITY: Mat = [1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0];

/** Lit un attribut `transform` 3mf (12 flottants), ou l'identité si absent/invalide. */
function parseTransform(s: string | undefined): Mat {
	if (!s) return IDENTITY;
	const n = s.trim().split(/\s+/).map(Number);
	return n.length === 12 && n.every((x) => Number.isFinite(x)) ? n : IDENTITY;
}

/** Composition `a` puis `b` : `p·a·b = p·(a·b)`. */
function multiply(a: Mat, b: Mat): Mat {
	return [
		a[0] * b[0] + a[1] * b[3] + a[2] * b[6],
		a[0] * b[1] + a[1] * b[4] + a[2] * b[7],
		a[0] * b[2] + a[1] * b[5] + a[2] * b[8],
		a[3] * b[0] + a[4] * b[3] + a[5] * b[6],
		a[3] * b[1] + a[4] * b[4] + a[5] * b[7],
		a[3] * b[2] + a[4] * b[5] + a[5] * b[8],
		a[6] * b[0] + a[7] * b[3] + a[8] * b[6],
		a[6] * b[1] + a[7] * b[4] + a[8] * b[7],
		a[6] * b[2] + a[7] * b[5] + a[8] * b[8],
		a[9] * b[0] + a[10] * b[3] + a[11] * b[6] + b[9],
		a[9] * b[1] + a[10] * b[4] + a[11] * b[7] + b[10],
		a[9] * b[2] + a[10] * b[5] + a[11] * b[8] + b[11]
	];
}

/** Applique une transformation à un point, empile le résultat dans `out`. */
function pushPoint(m: Mat, x: number, y: number, z: number, out: number[]): void {
	out.push(
		x * m[0] + y * m[3] + z * m[6] + m[9],
		x * m[1] + y * m[4] + z * m[7] + m[10],
		x * m[2] + y * m[5] + z * m[8] + m[11]
	);
}

/** Un objet 3mf : maillage propre (soupe de triangles) et/ou composants référencés. */
interface Object3mf {
	soup?: number[];
	components?: { path?: string; objectid: string; transform: Mat }[];
}

/** Indexe les `<object id=…>` d'un fichier modèle (maillage inline et composants). */
function parseObjects(xml: string): Map<string, Object3mf> {
	const objs = new Map<string, Object3mf>();
	const objRe = /<object\b([^>]*)>([\s\S]*?)<\/object>/g;
	let m: RegExpExecArray | null;
	while ((m = objRe.exec(xml)) !== null) {
		const id = /\bid="([^"]+)"/.exec(m[1])?.[1];
		if (!id) continue;
		const body = m[2];
		const obj: Object3mf = {};
		const soup: number[] = [];
		collect3mfMeshes(body, soup);
		if (soup.length) obj.soup = soup;
		const comps: { path?: string; objectid: string; transform: Mat }[] = [];
		const compRe = /<component\b([^>]*?)\/?>/g;
		let c: RegExpExecArray | null;
		while ((c = compRe.exec(body)) !== null) {
			const cid = /\bobjectid="([^"]+)"/.exec(c[1])?.[1];
			if (!cid) continue;
			const path = /\b(?:p:)?path="([^"]+)"/.exec(c[1])?.[1];
			comps.push({
				objectid: cid,
				path,
				transform: parseTransform(/\btransform="([^"]+)"/.exec(c[1])?.[1])
			});
		}
		if (comps.length) obj.components = comps;
		objs.set(id, obj);
	}
	return objs;
}

/**
 * Parse un 3MF en un maillage d'aperçu, **transformations d'assemblage
 * appliquées** (positionnement sur le plateau, parité OrcaSlicer). Le format
 * scinde la géométrie : `3D/3dmodel.model` porte le `<build>` (items placés
 * sur le plateau) et des `<object>` en `<components>` référençant des
 * `3D/Objects/*.model` où vivent les `<mesh>`. On indexe donc tous les objets
 * de tous les fichiers `.model`, puis on parcourt chaque `<item>` du build en
 * composant la chaîne de transformations `item ∘ composant` jusqu'au maillage.
 * Repli sans build : on agrège les maillages bruts (aperçu quand même utile).
 * Extraction par regex (pas de DOMParser → testable sous bun).
 */
export function parse3mf(buffer: ArrayBuffer): SceneMesh {
	const files = unzipSync(new Uint8Array(buffer));
	// `.rels` (ex. `3dmodel.model.rels`) est exclu naturellement : il finit par
	// `.rels`, pas `.model`.
	const modelKeys = Object.keys(files).filter((k) => k.toLowerCase().endsWith('.model'));
	if (modelKeys.length === 0) throw new Error('3MF invalide (modèle 3D absent)');

	const xmls = new Map<string, string>();
	const registry = new Map<string, Map<string, Object3mf>>();
	for (const key of modelKeys) {
		const xml = strFromU8(files[key]);
		xmls.set(key, xml);
		registry.set(key, parseObjects(xml));
	}

	// Localise le `<build>` (items placés sur le plateau) et son fichier hôte.
	let buildFile: string | undefined;
	const buildItems: { objectid: string; transform: Mat }[] = [];
	for (const [key, xml] of xmls) {
		const build = /<build\b[^>]*>([\s\S]*?)<\/build>/.exec(xml);
		if (!build) continue;
		buildFile = key;
		const itemRe = /<item\b([^>]*?)\/?>/g;
		let it: RegExpExecArray | null;
		while ((it = itemRe.exec(build[1])) !== null) {
			const oid = /\bobjectid="([^"]+)"/.exec(it[1])?.[1];
			if (!oid) continue;
			buildItems.push({
				objectid: oid,
				transform: parseTransform(/\btransform="([^"]+)"/.exec(it[1])?.[1])
			});
		}
		break;
	}

	const positions: number[] = [];
	// Parcours récursif objet → composants, transformations composées.
	function emit(filePath: string, id: string, accum: Mat, depth: number): void {
		if (depth > 50) return; // garde anti-cycle de composants
		const obj = registry.get(filePath)?.get(id);
		if (!obj) return;
		if (obj.soup) {
			for (let i = 0; i < obj.soup.length; i += 3) {
				pushPoint(accum, obj.soup[i], obj.soup[i + 1], obj.soup[i + 2], positions);
			}
		}
		if (obj.components) {
			for (const comp of obj.components) {
				const childFile = comp.path ? comp.path.replace(/^\//, '') : filePath;
				emit(childFile, comp.objectid, multiply(comp.transform, accum), depth + 1);
			}
		}
	}

	if (buildFile) {
		for (const item of buildItems) emit(buildFile, item.objectid, item.transform, 0);
	}
	// Repli : aucun build exploitable (3mf minimal) → maillages bruts non transformés.
	if (positions.length === 0) {
		for (const xml of xmls.values()) collect3mfMeshes(xml, positions);
	}
	if (positions.length === 0) throw new Error('3MF sans géométrie exploitable');
	return meshFromTriangleSoup(positions);
}

// --- Dispatch d'aperçu --------------------------------------------------------

/** Parse un tampon selon l'extension du nom de fichier (aperçu / repeuplement). */
export function previewFromBuffer(filename: string, buffer: ArrayBuffer): SceneMesh {
	const fmt = previewFormat(filename);
	if (!fmt) throw new Error(`format non prévisualisable : ${filename}`);
	if (fmt === 'stl') return parseStl(buffer);
	if (fmt === 'obj') return parseObj(new TextDecoder().decode(buffer));
	return parse3mf(buffer);
}

/** Produit un aperçu maillé depuis un fichier local, selon son extension. */
export async function loadPreview(file: File): Promise<SceneMesh> {
	return previewFromBuffer(file.name, await file.arrayBuffer());
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

/**
 * Récupère le fichier source brut d'un modèle (`GET …/file`, T092) : sert à
 * reconstruire l'aperçu client des formats non décodés serveur (OBJ/3MF).
 */
export async function fetchModelFile(modelId: string): Promise<ArrayBuffer> {
	const res = await fetch(`${API_BASE}/models/${modelId}/file`, { credentials: 'include' });
	if (!res.ok) {
		const text = await res.text();
		const err = (text ? JSON.parse(text) : {}) as Partial<ErrorBody>;
		throw new ApiError(res.status, err.code ?? 'error', err.message ?? res.statusText, err.details);
	}
	return res.arrayBuffer();
}
