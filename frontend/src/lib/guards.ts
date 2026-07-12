// Gardes de chargement partagées (Phase 14, T135). `redirect()` lève une
// exception SvelteKit : appelée depuis un `load`, elle interrompt le rendu.
import { redirect } from '@sveltejs/kit';
import { listPrinters } from '$lib/api/printers';
import { refreshSession } from '$lib/api/session';
import type { PrinterResponse, UserResponse } from '$lib/api/types';

/** Garde de session simple : redirige vers `/login` si non connecté. */
export async function requireUser(): Promise<UserResponse> {
	const user = await refreshSession();
	if (!user) {
		redirect(302, '/login');
	}
	return user;
}

/**
 * Garde de session **et d'onboarding** : `/login` si non connecté, `/setup` si
 * aucune imprimante n'est déclarée (décision utilisateur : wizard forcé, on ne
 * peut rien trancher sans imprimante). Renvoie le compte et ses imprimantes.
 */
export async function requireOnboardedUser(): Promise<{
	user: UserResponse;
	printers: PrinterResponse[];
}> {
	const user = await requireUser();
	const printers = await listPrinters();
	if (printers.length === 0) {
		redirect(303, '/setup');
	}
	return { user, printers };
}
