// Garde de session + chargement des imprimantes et des presets machine (T077).
// Rendu client uniquement (session par cookie, WebSocket temps réel au montage).
import { redirect } from '@sveltejs/kit';
import { listPrinters } from '$lib/api/printers';
import { listPresets } from '$lib/api/presets';
import { refreshSession } from '$lib/api/session';
import type { PageLoad } from './$types';

export const ssr = false;

export const load: PageLoad = async () => {
	const user = await refreshSession();
	if (!user) {
		redirect(302, '/login');
	}
	const [printers, machines] = await Promise.all([listPrinters(), listPresets('machine')]);
	return { user, printers, machines };
};
