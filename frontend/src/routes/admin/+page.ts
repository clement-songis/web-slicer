// Administration de l'instance (phase 32) : réservée aux comptes `admin`.
import { error } from '@sveltejs/kit';
import { getInstance } from '$lib/api/admin';
import { requireUser } from '$lib/guards';
import type { PageLoad } from './$types';

export const ssr = false;

export const load: PageLoad = async () => {
	const user = await requireUser();
	if (user.role !== 'admin') error(403, "Page réservée aux administrateurs de l'instance.");
	const instance = await getInstance();
	return { user, instance };
};
