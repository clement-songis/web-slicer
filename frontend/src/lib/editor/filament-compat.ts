// Compatibilité haute / basse température (Orca Print::check_multi_filaments_compatibility,
// préférence « Remove mixed temperature restriction ») : contrôle à la volée (lot 21).
import { api } from '$lib/api/client';
import type { FilamentCompatibilityRequest, FilamentCompatibilityResponse } from '$lib/api/types';

export const checkFilamentCompatibility = (projectId: string, body: FilamentCompatibilityRequest) =>
	api.post<FilamentCompatibilityResponse>(`/projects/${projectId}/filament-compatibility`, body);

/** Texte affiché (verdict en clair ; le message d'Orca suit, traduit si connu). */
export function compatibilityText(r: FilamentCompatibilityResponse): string | null {
	if (r.compatibility === 'compatible') return null;
	if (r.compatibility === 'invalid_temperature_range')
		return 'Plage de température d’un filament invalide : vérifiez les températures minimale et maximale recommandées.';
	return r.blocking
		? 'Filaments haute et basse température mélangés : le tranchage sera refusé (risque de bouchage de la buse).'
		: 'Filaments haute et basse température mélangés : tranchage autorisé par vos préférences, surveillez la buse.';
}
