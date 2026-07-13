// Garde de session + chargement des imprimantes et des presets machine (T077).
// Rendu client uniquement (session par cookie, WebSocket temps réel au montage).
import { getPrinterCatalog, listPrinters } from '$lib/api/printers';
import { listPresets } from '$lib/api/presets';
import { requireUser } from '$lib/guards';
import type { PageLoad } from './$types';

export const ssr = false;

export const load: PageLoad = async () => {
	// Garde de session **seule** (pas d'onboarding) : la page de gestion doit
	// rester accessible même sans imprimante, c'est là qu'on en ajoute (Phase 14).
	const user = await requireUser();
	const [printers, machines, catalog] = await Promise.all([
		listPrinters(),
		listPresets('machine'),
		getPrinterCatalog()
	]);
	return { user, printers, machines, catalog };
};
