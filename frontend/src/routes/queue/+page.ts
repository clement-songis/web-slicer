// Garde de session + chargement de la file (T071). Rendu client uniquement
// (session par cookie navigateur, WebSocket temps réel au montage).
import { redirect } from '@sveltejs/kit';
import { listJobs } from '$lib/api/jobs';
import { refreshSession } from '$lib/api/session';
import type { PageLoad } from './$types';

export const ssr = false;

export const load: PageLoad = async () => {
	const user = await refreshSession();
	if (!user) {
		redirect(302, '/login');
	}
	const jobs = await listJobs();
	return { user, jobs };
};
