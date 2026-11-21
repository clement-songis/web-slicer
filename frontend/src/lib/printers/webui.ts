// Interface web de l'hôte d'impression dans l'onglet Appareil (phase 39, Orca
// 2.4.1 MainFrame::load_printer_url → PrintHost::get_print_host_webui) : pour
// les hôtes sans suivi en direct ici (OctoPrint, PrusaLink, Duet, FlashAir,
// AstroBox, Repetier, MKS, ESP3D, ElegooLink…), Orca affiche dans l'onglet
// Device la page web de l'hôte : `print_host_webui`, sinon `print_host`,
// préfixée de « http:// » sans schéma http/file. Le navigateur peut refuser
// l'intégration (X-Frame-Options / CSP frame-ancestors, contenu mixte http
// dans une page https) : repli sur un lien externe. Pur, sauf le stockage local.

/** URL de l'interface web (PrintHost::get_print_host_webui). '' = aucune. */
export function webuiUrl(src: {
	/** Adresse saisie localement (surcharge, comme la webcam). */
	override?: string | null;
	/** `print_host_webui` du profil machine. */
	presetWebui?: string | null;
	/** `print_host` : adresse de connexion de l'imprimante. */
	host?: string | null;
}): string {
	let url = (src.override ?? '').trim();
	if (!url) url = (src.presetWebui ?? '').trim();
	if (!url) url = (src.host ?? '').trim();
	if (!url) return '';
	if (!/^http/i.test(url) && !/^file:/i.test(url)) url = `http://${url}`;
	return url;
}

/**
 * Raison connue d'avance pour laquelle le navigateur bloquera l'iframe :
 * `mixed` (page https, hôte http), `file` (chemin local, refusé par le
 * navigateur), `invalid` (adresse illisible) ; null = tentative possible.
 */
export function embedBlockedReason(
	url: string,
	pageProtocol: string
): 'mixed' | 'file' | 'invalid' | null {
	let u: URL;
	try {
		u = new URL(url);
	} catch {
		return 'invalid';
	}
	if (u.protocol === 'file:') return 'file';
	if (u.protocol !== 'http:' && u.protocol !== 'https:') return 'invalid';
	if (pageProtocol === 'https:' && u.protocol === 'http:') return 'mixed';
	return null;
}

/** Explication affichée avec le lien de repli. */
export function blockedMessage(
	reason: 'mixed' | 'file' | 'invalid' | 'refused' | 'manual'
): string {
	switch (reason) {
		case 'mixed':
			return 'Cette page est en HTTPS et l’hôte en HTTP : le navigateur interdit de l’afficher ici.';
		case 'file':
			return 'Interface locale (fichier) : elle ne peut pas être affichée dans le navigateur.';
		case 'invalid':
			return 'Adresse de l’interface illisible : vérifiez-la ci-dessous.';
		case 'refused':
			return 'L’hôte n’a pas répondu ou refuse d’être affiché dans une autre page.';
		case 'manual':
			return 'Intégration désactivée pour cette imprimante.';
	}
}

/** Délai au-delà duquel une iframe sans événement « load » est jugée refusée. */
export const EMBED_TIMEOUT_MS = 8000;

/** Réglages locaux par imprimante : adresse saisie, intégration désactivée. */
export interface WebuiPrefs {
	url?: string;
	noEmbed?: boolean;
}

const KEY = 'web-slicer.device-webui';

export function loadWebuiPrefs(printerId: string): WebuiPrefs {
	try {
		const all = JSON.parse(localStorage.getItem(KEY) ?? '{}') as Record<string, WebuiPrefs>;
		const p = all[printerId];
		return p && typeof p === 'object' ? p : {};
	} catch {
		return {};
	}
}

export function saveWebuiPrefs(printerId: string, prefs: WebuiPrefs): void {
	try {
		const all = JSON.parse(localStorage.getItem(KEY) ?? '{}') as Record<string, WebuiPrefs>;
		const clean: WebuiPrefs = {};
		if (prefs.url?.trim()) clean.url = prefs.url.trim();
		if (prefs.noEmbed) clean.noEmbed = true;
		if (Object.keys(clean).length) all[printerId] = clean;
		else delete all[printerId];
		localStorage.setItem(KEY, JSON.stringify(all));
	} catch {
		// Stockage indisponible : réglage valable pour la session seulement.
	}
}

/** `print_host_webui` lu dans les valeurs résolues d'un preset machine. */
export function presetWebui(values: Record<string, unknown> | null | undefined): string {
	const v = values?.print_host_webui;
	const x = Array.isArray(v) ? v[0] : v;
	return typeof x === 'string' ? x.trim() : '';
}
