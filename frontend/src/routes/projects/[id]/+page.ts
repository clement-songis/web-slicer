// Ouverture d'un projet : garde de session + chargement typé. L'éditeur complet
// (scène 3D, onglets réglages) arrive en phases ultérieures ; cette page sert de
// destination réelle à « Ouvrir » et de point d'ancrage au brouillon local.
import { error, redirect } from '@sveltejs/kit';
import { ApiError } from '$lib/api/client';
import { getProject } from '$lib/api/projects';
import { refreshSession } from '$lib/api/session';
import type { PageLoad } from './$types';

export const ssr = false;

export const load: PageLoad = async ({ params }) => {
	const user = await refreshSession();
	if (!user) {
		redirect(302, '/login');
	}
	try {
		const project = await getProject(params.id);
		return { project };
	} catch (e) {
		// 404 = inexistant ou appartenant à autrui (SC-008) : même page d'erreur.
		if (e instanceof ApiError && e.status === 404) {
			error(404, 'Projet introuvable');
		}
		throw e;
	}
};
