// Envoi d'un G-code vers une imprimante Moonraker (FR-061/FR-062) : logique
// pure du dialogue « Envoyer à l'imprimante » (sélection des cibles, messages
// d'erreur lisibles). Le transport vit dans `lib/api/printers`.
import { ApiError } from '$lib/api/client';
import type { PrinterResponse, PrinterUploadResponse } from '$lib/api/types';
import { tr } from '$lib/i18n';
import { uploadNameErrorReason, uploadNameReasonMessage } from './uploads';

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
	// Audit 41 M1 : Chromium accepte « http://pas une url » (espaces dans
	// l'hôte) ; une adresse d'hôte ne contient jamais d'espace.
	if (/\s/.test(value)) return null;
	const withScheme = /^[a-z][a-z0-9+.-]*:\/\//i.test(value) ? value : `http://${value}`;
	const authority = /^[a-z][a-z0-9+.-]*:\/\/([^/?#]*)/i.exec(withScheme)?.[1] ?? '';
	if (!validAuthority(authority)) return null;
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
 * Partie « [utilisateur@]hôte[:port] » d'une URL d'hôte d'impression : nom DNS
 * (lettres, chiffres, tirets, points, soulignés), IPv4 ou [IPv6], port 1–65535.
 */
function validAuthority(authority: string): boolean {
	const at = authority.lastIndexOf('@');
	const hostPort = at >= 0 ? authority.slice(at + 1) : authority;
	const m = /^(\[[0-9a-f:.]+\]|[a-z0-9_](?:[a-z0-9_.-]*[a-z0-9_])?)(?::(\d{1,5}))?$/i.exec(
		hostPort
	);
	if (!m) return false;
	if (m[2] !== undefined) {
		const port = Number(m[2]);
		if (port < 1 || port > 65535) return false;
	}
	const host = m[1];
	if (host.startsWith('[')) return true;
	if (host.includes('..')) return false;
	// Tout numérique : IPv4 complète exigée (« 192.168.1 » refusé).
	if (/^[\d.]+$/.test(host))
		return /^(\d{1,3})(\.\d{1,3}){3}$/.test(host) && host.split('.').every((n) => Number(n) <= 255);
	return true;
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

/** Raisons de refus d'une adresse d'hôte (lot 42 M1, details.reason du 422). */
export type HostUrlReason = 'whitespace' | 'scheme' | 'host' | 'port' | 'query';

/** Message lisible d'une raison de refus d'adresse d'hôte, avec exemples. */
export function hostUrlReasonMessage(reason: string, value?: string): string {
	const why = (() => {
		switch (reason) {
			case 'whitespace':
				return tr('spaces are not allowed in an address');
			case 'scheme':
				return tr('only http:// and https:// are supported');
			case 'host':
				return tr('the host name or IP address is not valid');
			case 'port':
				return tr('the port must be a number between 1 and 65535');
			case 'query':
				return tr('remove the “?” or “#” part');
			default:
				return tr('this is not a valid address');
		}
	})();
	const head = value?.trim()
		? tr('Invalid host address “{value}”: {why}.', { value: value.trim(), why })
		: tr('Invalid host address: {why}.', { why });
	return head + ' ' + tr('Examples: 192.168.1.20:7125, voron.local or http://voron.local:7125');
}

/**
 * Message traduit d'un refus d'adresse par le serveur (422 details.field =
 * moonraker_url, ou 409 printer_not_connected : adresse enregistrée
 * inutilisable), sinon null.
 */
export function hostUrlError(e: unknown): string | null {
	if (!(e instanceof ApiError)) return null;
	const d = e.details as { field?: string; reason?: string; value?: string } | null | undefined;
	if (e.status === 422 && d?.field === 'moonraker_url')
		return hostUrlReasonMessage(d.reason ?? '', d.value);
	if (isPrinterNotConnected(e)) {
		if (d?.reason) return hostUrlReasonMessage(d.reason, d.value);
		return tr('The printer address is missing or cannot be used: enter it again.');
	}
	return null;
}

/** 409 printer_not_connected : l'interface redemande l'adresse (lot 42 M1). */
export function isPrinterNotConnected(e: unknown): boolean {
	return e instanceof ApiError && e.status === 409 && e.code === 'printer_not_connected';
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
		const host = hostUrlError(e);
		if (host) return host;
		const nameReason = uploadNameErrorReason(e);
		if (nameReason !== null) return uploadNameReasonMessage(nameReason);
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
