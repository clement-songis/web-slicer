// Compatibilité haute / basse température (Orca Print::check_multi_filaments_compatibility,
// préférence « Remove mixed temperature restriction ») : contrôle à la volée (lot 21).
import { api } from '$lib/api/client';
import { tr } from '$lib/i18n';
import type { FilamentCompatibilityRequest, FilamentCompatibilityResponse } from '$lib/api/types';

export const checkFilamentCompatibility = (projectId: string, body: FilamentCompatibilityRequest) =>
	api.post<FilamentCompatibilityResponse>(`/projects/${projectId}/filament-compatibility`, body);

/** Texte affiché (verdict en clair ; le message d'Orca suit, traduit si connu). */
export function compatibilityText(r: FilamentCompatibilityResponse): string | null {
	if (r.compatibility === 'compatible') return null;
	if (r.compatibility === 'invalid_temperature_range')
		return tr(
			'Invalid temperature range of a filament: check the recommended minimum and maximum temperatures.'
		);
	return r.blocking
		? tr(
				'High and low temperature filaments mixed: slicing will be refused (risk of nozzle clogging).'
			)
		: tr(
				'High and low temperature filaments mixed: slicing allowed by your preferences, watch the nozzle.'
			);
}
