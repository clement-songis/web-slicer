// Envoi d'un G-code vers une imprimante Moonraker (FR-061/FR-062) : logique
// pure du dialogue « Envoyer à l'imprimante » (sélection des cibles, messages
// d'erreur lisibles). Le transport vit dans `lib/api/printers`.
import { ApiError } from '$lib/api/client';
import type { PrinterResponse, PrinterUploadResponse } from '$lib/api/types';

/** Imprimantes pouvant recevoir un fichier : URL Moonraker renseignée. */
export function sendTargets(printers: readonly PrinterResponse[]): PrinterResponse[] {
	return printers.filter((p) => !!p.moonraker_url && p.moonraker_url.trim() !== '');
}

/**
 * Imprimantes proposées dans le dialogue d'envoi : **toutes** celles du compte
 * (y compris celle créée à l'inscription, sans URL), celles qui peuvent
 * recevoir un fichier en premier (sélection par défaut), ordre stable sinon.
 */
export function sendChoices(printers: readonly PrinterResponse[]): PrinterResponse[] {
	const ready = sendTargets(printers);
	return [...ready, ...printers.filter((p) => !ready.includes(p))];
}

/** Libellé d'option : « nom — URL » ou « nom — non connectée ». */
export function printerOptionLabel(p: PrinterResponse): string {
	const url = p.moonraker_url?.trim();
	return url ? `${p.name} — ${url}` : `${p.name} — non connectée`;
}

/**
 * URL Moonraker saisie dans le dialogue : espaces retirés, `http://` ajouté
 * si le schéma manque (« 192.168.1.20:7125 »), barre finale retirée.
 * `null` si la valeur n'est pas une URL http(s) valable.
 */
export function normalizeMoonrakerUrl(raw: string): string | null {
	const value = raw.trim();
	if (!value) return null;
	const withScheme = /^[a-z][a-z0-9+.-]*:\/\//i.test(value) ? value : `http://${value}`;
	try {
		const url = new URL(withScheme);
		if (url.protocol !== 'http:' && url.protocol !== 'https:') return null;
		if (!url.hostname) return null;
		return withScheme.replace(/\/+$/, '');
	} catch {
		return null;
	}
}

/** Libellé de succès : fichier déposé, impression démarrée ou non. */
export function sendSuccessMessage(printerName: string, result: PrinterUploadResponse): string {
	return result.print_started
		? `« ${result.path} » envoyé à ${printerName} — impression démarrée.`
		: `« ${result.path} » envoyé à ${printerName}.`;
}

/**
 * Message d'erreur clair pour l'utilisateur. Le backend traduit toute panne
 * Moonraker en 502 (FR-062) avec un détail technique : on distingue
 * l'imprimante injoignable, le refus de l'imprimante et une réponse illisible.
 */
export function sendErrorMessage(e: unknown, printerName = "l'imprimante"): string {
	if (e instanceof ApiError) {
		const detail = e.message ?? '';
		if (e.status === 502 || e.status === 504) {
			if (/réseau|network|connect|timed? ?out|dns/i.test(detail))
				return `${printerName} est injoignable : vérifiez l'URL Moonraker, le réseau et que l'imprimante est allumée.`;
			const status = detail.match(/statut HTTP (\d{3})/);
			if (status) {
				if (status[1] === '401' || status[1] === '403')
					return `${printerName} a refusé l'envoi (HTTP ${status[1]}) : vérifiez la clé API.`;
				return `${printerName} a refusé l'envoi (HTTP ${status[1]}).`;
			}
			return `Réponse inattendue de ${printerName} : ${detail}`;
		}
		if (e.status === 404) return "Le G-code ou l'imprimante est introuvable : tranchez à nouveau.";
		if (e.status === 401) return 'Session expirée : reconnectez-vous.';
		return detail || `Échec de l'envoi (HTTP ${e.status}).`;
	}
	if (e instanceof TypeError) return 'Serveur web-slicer injoignable : vérifiez votre connexion.';
	return e instanceof Error ? e.message : "Échec de l'envoi.";
}
