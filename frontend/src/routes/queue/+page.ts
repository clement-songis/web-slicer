// Garde de session + chargement de la file (T071). Rendu client uniquement
// (session par cookie navigateur, WebSocket temps réel au montage).
import { listJobs } from '$lib/api/jobs';
import { listProjects } from '$lib/api/projects';
import { requireOnboardedUser } from '$lib/guards';
import type { PageLoad } from './$types';

export const ssr = false;

export const load: PageLoad = async () => {
	// Garde session + onboarding (Phase 14) ; la liste d'imprimantes vient de là.
	const { user, printers } = await requireOnboardedUser();
	// Noms de projet (phase 32) : plusieurs jobs simultanés se distinguent par
	// leur projet, pas seulement par le n° de plateau. Confort : pas bloquant.
	const [jobs, projects] = await Promise.all([listJobs(), listProjects().catch(() => [])]);
	return { user, jobs, printers, projects };
};
