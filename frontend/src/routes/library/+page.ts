// Garde de session + chargement de la bibliothèque. Rendu client uniquement :
// la session est un cookie navigateur, on évite un fetch SSR sans contexte.
import { redirect } from '@sveltejs/kit';
import { listProjects } from '$lib/api/projects';
import { refreshSession } from '$lib/api/session';
import type { PageLoad } from './$types';

export const ssr = false;

export const load: PageLoad = async () => {
	const user = await refreshSession();
	if (!user) {
		redirect(302, '/login');
	}
	const projects = await listProjects();
	return { user, projects };
};
