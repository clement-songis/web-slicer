// Garde de session + chargement de la bibliothèque. Rendu client uniquement :
// la session est un cookie navigateur, on évite un fetch SSR sans contexte.
import { listProjects } from '$lib/api/projects';
import { requireOnboardedUser } from '$lib/guards';
import type { PageLoad } from './$types';

export const ssr = false;

export const load: PageLoad = async () => {
	// Garde session + onboarding : sans imprimante déclarée → /setup (Phase 14).
	const { user } = await requireOnboardedUser();
	const projects = await listProjects();
	return { user, projects };
};
