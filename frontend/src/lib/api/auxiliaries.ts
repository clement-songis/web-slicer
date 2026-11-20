// Fichiers joints au projet (Orca : Auxiliaries/ du 3MF), lot 16 :
// GET /api/projects/{id}/auxiliaries → [{ path, size }] ;
// PUT / GET / DELETE /api/projects/{id}/auxiliaries/{dossier}/{fichier}.
import { api, API_BASE, ApiError } from './client';
import { apiErrorMessage } from './quota';
import type { ErrorBody } from './types';
import type { ObjColorsResponse } from '$lib/editor/obj-colors';

export interface AuxiliaryEntry {
	/** « <dossier>/<fichier> ». */
	path: string;
	size: number;
}

const enc = (folder: string, name: string) =>
	`${encodeURIComponent(folder)}/${encodeURIComponent(name)}`;

export const listAuxiliaries = (projectId: string) =>
	api.get<AuxiliaryEntry[]>(`/projects/${projectId}/auxiliaries`);

/** URL du fichier (aperçu d'image, téléchargement ; cookie de session). */
export const auxiliaryUrl = (projectId: string, folder: string, name: string) =>
	`${API_BASE}/projects/${projectId}/auxiliaries/${enc(folder, name)}`;

async function failure(res: Response): Promise<never> {
	const text = await res.text().catch(() => '');
	let err: Partial<ErrorBody> = {};
	try {
		err = text ? (JSON.parse(text) as Partial<ErrorBody>) : {};
	} catch {
		/* corps non JSON (413 du proxy…) */
	}
	const message = apiErrorMessage(res.status, err, res.statusText);
	throw new ApiError(res.status, err.code ?? 'error', message, err.details);
}

/** Téléverse (ou remplace) un fichier ; `true` si créé, `false` si remplacé. */
export async function putAuxiliary(
	projectId: string,
	folder: string,
	name: string,
	body: Blob
): Promise<boolean> {
	const res = await fetch(auxiliaryUrl(projectId, folder, name), {
		method: 'PUT',
		credentials: 'include',
		headers: { 'content-type': body.type || 'application/octet-stream' },
		body
	});
	if (!res.ok) await failure(res);
	return res.status === 201;
}

export const deleteAuxiliary = (projectId: string, folder: string, name: string) =>
	api.del<void>(`/projects/${projectId}/auxiliaries/${enc(folder, name)}`);

/** Couleurs d'un OBJ (corps optionnel = contenu du MTL). */
export async function objColors(modelId: string, mtl?: Blob | null): Promise<ObjColorsResponse> {
	const res = await fetch(`${API_BASE}/models/${modelId}/obj-colors`, {
		method: 'POST',
		credentials: 'include',
		headers: { 'content-type': 'application/octet-stream' },
		body: mtl ?? new Blob([])
	});
	if (!res.ok) await failure(res);
	return (await res.json()) as ObjColorsResponse;
}
