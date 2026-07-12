// Upload de modèles + récupération du maillage d'affichage décodé par le moteur
// (retournement R7/T126). Les parseurs client STL/OBJ/3MF ont été retirés : le
// moteur (libslic3r, seule source de vérité) décode tous les formats côté
// serveur, le maillage arrive via l'événement `model.converted` puis `fetchMesh`.
// `meshFromTriangleSoup` reste ici (assemblage de primitives/outils de scène).
import { API_BASE, ApiError } from '../api/client';
import type { ModelResponse, ErrorBody } from '../api/types';
import { decodeMesh, type SceneMesh } from './mesh';

// --- Assemblage : soupe de triangles → SceneMesh facetté ---------------------

/**
 * Construit un `SceneMesh` non indexé à partir de positions de triangles
 * (9 floats par triangle). Les normales sont calculées par face (aspect
 * facetté volontaire) et les index sont séquentiels. Utilisé par les primitives
 * et les outils de scène (cut/simplify), pas par l'import (décodage moteur).
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

// --- Upload + récupération du maillage backend -------------------------------

/**
 * Upload multipart d'un modèle. Le décodage moteur (tous formats) démarre côté
 * serveur ; le maillage arrive ensuite via `model.converted` → `fetchMesh`.
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
