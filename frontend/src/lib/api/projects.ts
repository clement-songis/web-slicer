// Appels de la bibliothèque de projets, typés sur les DTO générés.
import { api } from './client';
import type { CreateProjectRequest, ProjectResponse } from './types';

export const listProjects = () => api.get<ProjectResponse[]>('/projects');

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

export const deleteProject = (id: string) => api.del<void>(`/projects/${id}`);

export const duplicateProject = (id: string) =>
	api.post<ProjectResponse>(`/projects/${id}/duplicate`);

export const renameProject = (id: string, name: string) =>
	api.patch<ProjectResponse>(`/projects/${id}/rename`, { name });
