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

/**
 * Dernières listes reçues (audit 40 E1) : le filtrage de compatibilité côté
 * serveur coûte 1–2,7 s par liste ; en revenant sur un projet (navigation dans
 * l'application), la page affiche aussitôt la liste connue puis la remplace par
 * la réponse fraîche. Mémoire seulement (jamais persisté : pas de fuite entre
 * comptes après rechargement).
 */
const listCache = new Map<string, PresetSummary[]>();
const listKey = (kind: string, printer?: string) => `${kind}\u0000${printer ?? ''}`;

/** Liste déjà reçue pour ce type / cette imprimante (ou null). */
export function cachedPresetList(kind: string, printer?: string): PresetSummary[] | null {
	return listCache.get(listKey(kind, printer)) ?? null;
}

/** Oublie les listes connues (déconnexion, changement de compte). */
export function clearPresetListCache(): void {
	listCache.clear();
}

/** Presets système + utilisateur d'un type, filtrés par compatibilité (FR-021). */
export const listPresets = async (kind: string, printer?: string) => {
	const q = new URLSearchParams({ kind });
	if (printer) q.set('printer', printer);
	const list = await api.get<PresetSummary[]>(`/presets?${q.toString()}`);
	listCache.set(listKey(kind, printer), list);
	return list;
};

/** Valeurs brutes (surcharges du preset). */
export const getPreset = (id: string) => api.get<PresetDetail>(`/presets/${id}`);

/** Valeurs effectives (chaîne d'héritage résolue). */
export const getResolvedPreset = (id: string) => api.get<ResolvedPreset>(`/presets/${id}/resolved`);

/** Détache un preset de son parent système (Orca Tab::detach_preset) : copie
 *  utilisateur autonome pour un preset système (`name` requis), sur place pour
 *  un preset utilisateur. */
export const detachPreset = (id: string, name?: string | null) =>
	api.post<PresetDetail>(`/presets/${id}/detach`, name ? { name } : {});

/** Crée / dérive un preset utilisateur. */
export const createPreset = (body: CreatePresetRequest) => api.post<PresetDetail>('/presets', body);

/** Renomme et/ou remplace les valeurs d'un preset utilisateur. */
export const updatePreset = (id: string, body: UpdatePresetRequest) =>
	api.put<PresetDetail>(`/presets/${id}`, body);

/** Projets basculés sur le preset parent (Orca : les projets retombent sur le
 *  parent du preset supprimé). `undefined` (204) : aucun projet concerné. */
export interface PresetDeleteResult {
	parent: { id: string; name: string };
	projects: { id: string; name: string }[];
}

export const deletePreset = (id: string) =>
	api.del<PresetDeleteResult | undefined>(`/presets/${id}`);

/** `details.projects` d'un refus 409 (preset sans parent encore utilisé). */
export function blockingProjects(details: unknown): { id: string; name: string }[] {
	const list = (details as { projects?: unknown } | null | undefined)?.projects;
	return Array.isArray(list)
		? list.filter((p): p is { id: string; name: string } => typeof p?.id === 'string')
		: [];
}

/** Importe un profil JSON Orca (clés legacy converties, FR-023). */
export const importPreset = (body: ImportPresetRequest) =>
	api.post<PresetDetail>('/presets/import', body);

/** Exporte un preset au format JSON Orca. */
export const exportPreset = (id: string) => api.get<unknown>(`/presets/${id}/export`);

/** `GET /api/presets/create-options` : listes des assistants « Créer un filament
 *  / une imprimante » d'Orca (CreatePresetsDialog) + gabarits système. */
export interface CreateOptions {
	filament: {
		vendors: string[];
		types: string[];
		templates: { id: string; name: string; vendor?: string | null; type?: string | null }[];
	};
	printer: {
		vendors: string[];
		models: Record<string, string[]>;
		nozzle_diameters: string[];
		templates: {
			id: string;
			name: string;
			vendor?: string | null;
			model?: string | null;
			nozzle_diameter?: string | null;
		}[];
	};
}

let createOptions: Promise<CreateOptions | null> | null = null;
/** Mémorisé par session ; `null` si la route est absente (listes locales). */
export function getCreateOptions(): Promise<CreateOptions | null> {
	createOptions ??= api.get<CreateOptions>('/presets/create-options').catch(() => {
		createOptions = null;
		return null;
	});
	return createOptions;
}
