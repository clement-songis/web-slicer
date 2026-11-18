// Logique pure de l'accueil / bibliothèque (T118, US6 ; parité orca-home.png).
// Le composant `routes/library/+page.svelte` reste présentational : tri,
// formatage de date et URL de vignette sont testés ici, hors du `.svelte`.
import type { ProjectResponse } from '$lib/api/types';

/** Réordonne par date de mise à jour décroissante (comme le backend). */
export function sortByUpdated(list: readonly ProjectResponse[]): ProjectResponse[] {
	return [...list].sort((a, b) => b.updated_at.localeCompare(a.updated_at));
}

/** Page de démarrage (Orca `default_page`) : « Préparer » ouvre le dernier
 *  projet modifié, « Accueil » (ou aucun projet) la bibliothèque. */
export function startPath(page: 'home' | 'prepare', list: readonly ProjectResponse[]): string {
	if (page === 'prepare' && list.length) return `/projects/${sortByUpdated(list)[0].id}`;
	return '/library';
}

/** Vue « Récemment ouvert » bornée à `max` projets (Orca `max_recent_count`) ;
 *  `hidden` = projets masqués, affichables à la demande. */
export function limitRecent<T>(
	list: readonly T[],
	max: number,
	showAll: boolean
): { items: T[]; hidden: number } {
	if (showAll || list.length <= max) return { items: [...list], hidden: 0 };
	return { items: list.slice(0, Math.max(0, max)), hidden: list.length - Math.max(0, max) };
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

/** Tri proposé par la bibliothèque. */
export type ProjectSort = 'updated' | 'name' | 'created';

/** Trie une copie de la liste : modification ↓ (défaut), nom A→Z (locale,
 *  insensible à la casse et aux accents), création ↓. */
export function sortProjects(
	list: readonly ProjectResponse[],
	mode: ProjectSort
): ProjectResponse[] {
	const out = [...list];
	if (mode === 'name')
		return out.sort((a, b) =>
			a.name.localeCompare(b.name, undefined, { sensitivity: 'base', numeric: true })
		);
	if (mode === 'created') return out.sort((a, b) => b.created_at.localeCompare(a.created_at));
	return sortByUpdated(out);
}

const fold = (s: string) =>
	s
		.normalize('NFD')
		.replace(/\p{Diacritic}/gu, '')
		.toLowerCase();

/** Filtre par nom (sous-chaîne, insensible à la casse et aux accents). */
export function filterProjects(list: readonly ProjectResponse[], query: string): ProjectResponse[] {
	const q = fold(query.trim());
	return q ? list.filter((p) => fold(p.name).includes(q)) : [...list];
}

/** Date relative courte : « à l'instant », « il y a 5 min », « il y a 3 h »,
 *  « hier », « il y a 4 j » ; au-delà d'une semaine, la date du jour. */
export function relativeTime(rfc3339: string, now = Date.now()): string {
	const t = new Date(rfc3339).getTime();
	if (Number.isNaN(t)) return rfc3339;
	const s = Math.max(0, Math.round((now - t) / 1000));
	if (s < 60) return "à l'instant";
	const m = Math.round(s / 60);
	if (m < 60) return `il y a ${m} min`;
	const h = Math.round(m / 60);
	if (h < 24) return `il y a ${h} h`;
	const d = Math.round(h / 24);
	if (d === 1) return 'hier';
	if (d < 7) return `il y a ${d} j`;
	return new Date(t).toLocaleDateString();
}
