// Appels de l'API presets, typés sur les DTO générés (endpoints T039).
import { api } from './client';
import type {
	CreatePresetRequest,
	ImportPresetRequest,
	PresetDetail,
	PresetSummary,
	ResolvedPreset,
	UpdatePresetRequest
} from './types';

/** Presets système + utilisateur d'un type, filtrés par compatibilité (FR-021). */
export const listPresets = (kind: string, printer?: string) => {
	const q = new URLSearchParams({ kind });
	if (printer) q.set('printer', printer);
	return api.get<PresetSummary[]>(`/presets?${q.toString()}`);
};

/** Valeurs brutes (surcharges du preset). */
export const getPreset = (id: string) => api.get<PresetDetail>(`/presets/${id}`);

/** Valeurs effectives (chaîne d'héritage résolue). */
export const getResolvedPreset = (id: string) => api.get<ResolvedPreset>(`/presets/${id}/resolved`);

/** Crée / dérive un preset utilisateur. */
export const createPreset = (body: CreatePresetRequest) => api.post<PresetDetail>('/presets', body);

/** Renomme et/ou remplace les valeurs d'un preset utilisateur. */
export const updatePreset = (id: string, body: UpdatePresetRequest) =>
	api.put<PresetDetail>(`/presets/${id}`, body);

export const deletePreset = (id: string) => api.del<void>(`/presets/${id}`);

/** Importe un profil JSON Orca (clés legacy converties, FR-023). */
export const importPreset = (body: ImportPresetRequest) =>
	api.post<PresetDetail>('/presets/import', body);

/** Exporte un preset au format JSON Orca. */
export const exportPreset = (id: string) => api.get<unknown>(`/presets/${id}/export`);
