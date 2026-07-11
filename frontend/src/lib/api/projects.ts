// Appels de la bibliothèque de projets, typés sur les DTO générés.
import { api, API_BASE, ApiError } from './client';
import type {
	CreateProjectRequest,
	ErrorBody,
	ModelResponse,
	ProjectResponse,
	SliceRequest,
	SliceResponse
} from './types';

export const listProjects = () => api.get<ProjectResponse[]>('/projects');

/** Modèles rattachés à un projet, pour repeupler la scène à l'ouverture (T092). */
export const listProjectModels = (id: string) => api.get<ModelResponse[]>(`/projects/${id}/models`);

export const createProject = (body: CreateProjectRequest) =>
	api.post<ProjectResponse>('/projects', body);

export const getProject = (id: string) => api.get<ProjectResponse>(`/projects/${id}`);

/**
 * Sauvegarde avec verrou optimiste. `expectedVersion` est un `number` (le champ
 * `version` du DTO est un `bigint` ts-rs, coercé par l'appelant).
 */
export const saveProject = (
	id: string,
	expectedVersion: number,
	scene: unknown,
	activePresets: unknown
) =>
	api.put<ProjectResponse>(`/projects/${id}`, {
		expected_version: expectedVersion,
		scene,
		active_presets: activePresets
	});

/**
 * Lance le tranchage d'un projet (`POST /api/projects/{id}/slice`, T064) : cible
 * un plateau (`plate_index`) ou tous (`all`). Renvoie les jobs créés et les
 * avertissements moteur figés (FR-032).
 */
export const sliceProject = (id: string, body: SliceRequest) =>
	api.post<SliceResponse>(`/projects/${id}/slice`, body);

/**
 * Importe un fichier (`.3mf` projet ou modèle 3D) en un nouveau projet (T090,
 * `POST /api/projects/import`, multipart). Renvoie le projet créé.
 */
export async function importProject(file: File): Promise<ProjectResponse> {
	const form = new FormData();
	form.append('file', file, file.name);
	const res = await fetch(`${API_BASE}/projects/import`, {
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
	return data as ProjectResponse;
}

export const deleteProject = (id: string) => api.del<void>(`/projects/${id}`);

export const duplicateProject = (id: string) =>
	api.post<ProjectResponse>(`/projects/${id}/duplicate`);

export const renameProject = (id: string, name: string) =>
	api.patch<ProjectResponse>(`/projects/${id}/rename`, { name });
