// Page de démarrage (Orca `default_page`, préférence « Page au démarrage ») :
// bibliothèque par défaut, ou dernier projet modifié (« Préparer »). Les gardes
// renvoient vers `/login` sans session et vers `/setup` sans imprimante.
import { redirect } from '@sveltejs/kit';
import { listProjects } from '#lib/api/projects';
import { requireOnboardedUser } from '#lib/guards';
import { startPath } from '#lib/library/library';
import { currentPreferences, preferencesSettled } from '#lib/settings/preferences';
import type { PageLoad } from './$types';

export const ssr = false;

export const load: PageLoad = async () => {
	await requireOnboardedUser();
	// Préférences du compte (lecture serveur lancée par la session).
	await preferencesSettled();
	const page = currentPreferences().defaultPage;
	const projects = page === 'prepare' ? await listProjects().catch(() => []) : [];
	redirect(307, startPath(page, projects));
};
