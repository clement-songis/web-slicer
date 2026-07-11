// Logique pure de l'accueil / bibliothèque (T118, US6 ; parité orca-home.png).
// Le composant `routes/library/+page.svelte` reste présentational : tri,
// formatage de date et URL de vignette sont testés ici, hors du `.svelte`.
import type { ProjectResponse } from '$lib/api/types';

/** Réordonne par date de mise à jour décroissante (comme le backend). */
export function sortByUpdated(list: readonly ProjectResponse[]): ProjectResponse[] {
	return [...list].sort((a, b) => b.updated_at.localeCompare(a.updated_at));
}

/** Date RFC 3339 → libellé localisé ; retombe sur la chaîne brute si invalide. */
export function formatDate(rfc3339: string): string {
	const d = new Date(rfc3339);
	return Number.isNaN(d.getTime()) ? rfc3339 : d.toLocaleString();
}

/** URL de la vignette d'un projet, ou `null` s'il n'en a pas. */
export function thumbnailUrl(
	project: Pick<ProjectResponse, 'id' | 'has_thumbnail'>
): string | null {
	return project.has_thumbnail ? `/api/projects/${project.id}/thumbnail` : null;
}
