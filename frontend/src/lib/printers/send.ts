// Envoi d'un G-code vers une imprimante Moonraker (FR-061/FR-062) : logique
// pure du dialogue « Envoyer à l'imprimante » (sélection des cibles, messages
// d'erreur lisibles). Le transport vit dans `lib/api/printers`.
import { ApiError } from '$lib/api/client';
import type { PrinterResponse, PrinterUploadResponse } from '$lib/api/types';
import { tr } from '$lib/i18n';

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
	return url ? `${p.name} — ${url}` : tr('{name} — not connected', { name: p.name });
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

/**
 * Imprimante Bambu Lab (nom ou preset machine) : le réseau Bambu (MQTT/FTP,
 * module propriétaire) est exclu (exclusions.md), l'envoi direct n'existe pas.
 */
export function isBambuPrinter(
	p: Pick<PrinterResponse, 'name'> & { machine_preset_id?: string | null }
): boolean {
	return /bambu\s*lab|@BBL\b/i.test(`${p.name ?? ''} ${p.machine_preset_id ?? ''}`);
}

/**
 * Aide affichée quand l'imprimante choisie n'a pas encore d'adresse : ce
 * qu'il faut saisir, et l'alternative quand l'envoi direct est impossible.
 */
export function connectionHint(
	p: Pick<PrinterResponse, 'name'> & { machine_preset_id?: string | null }
): string | null {
	if (isBambuPrinter(p))
		return tr(
			'Bambu Lab printers use the Bambu network, which is not supported: export the G-code (SD card) or enter the address of a compatible host (Moonraker, OctoPrint…).'
		);
	return null;
}

/** Libellé de succès : fichier déposé, impression démarrée ou non. */
export function sendSuccessMessage(
	printerName: string,
	result: PrinterUploadResponse,
	queued = false
): string {
	const params = { file: result.path, printer: printerName };
	if (queued) return tr('“{file}” sent to {printer} and queued.', params);
	return result.print_started
		? tr('“{file}” sent to {printer} — print started.', params)
		: tr('“{file}” sent to {printer}.', params);
}

/**
 * Message d'erreur clair pour l'utilisateur. Le backend traduit toute panne
 * Moonraker en 502 (FR-062) avec un détail technique : on distingue
 * l'imprimante injoignable, le refus de l'imprimante et une réponse illisible.
 */
export function sendErrorMessage(e: unknown, name?: string): string {
	const printerName = name ?? tr('the printer');
	if (e instanceof ApiError) {
		const detail = e.message ?? '';
		if (e.status === 502 || e.status === 504) {
			if (/réseau|network|connect|timed? ?out|dns/i.test(detail))
				return tr(
					'{printer} is unreachable: check the Moonraker URL, the network and that the printer is on.',
					{ printer: printerName }
				);
			const status = detail.match(/statut HTTP (\d{3})/);
			if (status) {
				if (status[1] === '401' || status[1] === '403')
					return tr('{printer} rejected the upload (HTTP {code}): check the API key.', {
						printer: printerName,
						code: status[1]
					});
				return tr('{printer} rejected the upload (HTTP {code}).', {
					printer: printerName,
					code: status[1]
				});
			}
			return tr('Unexpected response from {printer}: {detail}', { printer: printerName, detail });
		}
		if (e.status === 404) return tr('The G-code or the printer cannot be found: slice again.');
		if (e.status === 401) return tr('Session expired: sign in again.');
		return detail || tr('Upload failed (HTTP {code}).', { code: e.status });
	}
	if (e instanceof TypeError) return tr('web-slicer server unreachable: check your connection.');
	return e instanceof Error ? e.message : tr('Upload failed.');
}
