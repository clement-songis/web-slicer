// Wizard d'onboarding (Phase 14, T135) : sélection des imprimantes possédées à
// la première connexion. Garde de session **simple** (pas d'onboarding, sinon
// boucle : c'est la page qui déclare les imprimantes).
import { getPrinterCatalog } from '$lib/api/printers';
import { requireUser } from '$lib/guards';
import type { PageLoad } from './$types';

export const ssr = false;

export const load: PageLoad = async () => {
	const user = await requireUser();
	const catalog = await getPrinterCatalog();
	return { user, catalog };
};
