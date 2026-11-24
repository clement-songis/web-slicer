// Administration de l'instance (phase 32) : réservée aux comptes `admin`.
import { error } from '@sveltejs/kit';
import { getInstance } from '#lib/api/admin';
import { requireUser } from '#lib/guards';
import { tr } from '#lib/i18n';
import type { PageLoad } from './$types';

export const ssr = false;

export const load: PageLoad = async () => {
	const user = await requireUser();
	if (user.role !== 'admin') error(403, tr('Page reserved for instance administrators.'));
	const instance = await getInstance();
	return { user, instance };
};
