// Garde de session + chargement de la file (T071). Rendu client uniquement
// (session par cookie navigateur, WebSocket temps réel au montage).
import { listJobs } from '$lib/api/jobs';
import { requireOnboardedUser } from '$lib/guards';
import type { PageLoad } from './$types';

export const ssr = false;

export const load: PageLoad = async () => {
	// Garde session + onboarding (Phase 14) ; la liste d'imprimantes vient de là.
	const { user, printers } = await requireOnboardedUser();
	const jobs = await listJobs();
	return { user, jobs, printers };
};
