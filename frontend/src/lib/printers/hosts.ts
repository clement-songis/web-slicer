// Hôtes d'impression (Orca : PhysicalPrinterDialog, host_type) : types pris en
// charge par le serveur (lot 17), champs utiles à chacun, corps de requête.
// Pur → testable.
import type { PrinterResponse, SavePrinterRequest } from '$lib/api/types';

export type HostType =
	| 'moonraker'
	| 'octoprint'
	| 'astrobox'
	| 'prusalink'
	| 'prusaconnect'
	| 'duet'
	| 'repetier'
	| 'mks'
	| 'flashair'
	| 'esp3d';

export interface HostMeta {
	id: HostType;
	/** Nom affiché par Orca (PrintHost::get_name). */
	label: string;
	/** Exemple d'adresse. */
	placeholder: string;
	/** Libellé du champ secret (printhost_apikey), null = sans secret. */
	secret: string | null;
	/** Choix clé API / utilisateur + mot de passe (PrusaLink, PrusaConnect). */
	authChoice: boolean;
	/** Slug d'imprimante sur le serveur (Repetier : printhost_port). */
	printerSlug: boolean;
	/** Destinations proposées à l'envoi (stockages PrusaLink, groupes Repetier). */
	storages: boolean;
}

export const HOST_TYPES: readonly HostMeta[] = [
	{
		id: 'moonraker',
		label: 'Moonraker (Klipper)',
		placeholder: 'http://klipper.local:7125',
		secret: 'Clé API (facultatif)',
		authChoice: false,
		printerSlug: false,
		storages: false
	},
	{
		id: 'octoprint',
		label: 'OctoPrint',
		placeholder: 'http://octopi.local',
		secret: 'Clé API',
		authChoice: false,
		printerSlug: false,
		storages: false
	},
	{
		id: 'prusalink',
		label: 'PrusaLink',
		placeholder: 'http://192.168.1.30',
		secret: 'Clé API',
		authChoice: true,
		printerSlug: false,
		storages: true
	},
	{
		id: 'prusaconnect',
		label: 'PrusaConnect',
		placeholder: 'https://connect.prusa3d.com',
		secret: 'Clé API',
		authChoice: true,
		printerSlug: false,
		storages: true
	},
	{
		id: 'duet',
		label: 'Duet (RepRapFirmware)',
		placeholder: 'http://duet.local',
		secret: 'Mot de passe (facultatif)',
		authChoice: false,
		printerSlug: false,
		storages: false
	},
	{
		id: 'repetier',
		label: 'Repetier-Server',
		placeholder: 'http://repetier.local:3344',
		secret: 'Clé API',
		authChoice: false,
		printerSlug: true,
		storages: true
	},
	{
		id: 'astrobox',
		label: 'AstroBox',
		placeholder: 'http://astrobox.local',
		secret: 'Clé API',
		authChoice: false,
		printerSlug: false,
		storages: false
	},
	{
		id: 'mks',
		label: 'MKS (WiFi)',
		placeholder: 'http://192.168.1.40',
		secret: null,
		authChoice: false,
		printerSlug: false,
		storages: false
	},
	{
		id: 'flashair',
		label: 'FlashAir',
		placeholder: 'http://flashair',
		secret: null,
		authChoice: false,
		printerSlug: false,
		storages: false
	},
	{
		id: 'esp3d',
		label: 'ESP3D',
		placeholder: 'http://esp3d.local',
		secret: null,
		authChoice: false,
		printerSlug: false,
		storages: false
	}
];

export function hostMeta(type: string | null | undefined): HostMeta {
	return HOST_TYPES.find((h) => h.id === (type ?? '').toLowerCase()) ?? HOST_TYPES[0];
}

/** Suivi d'état, pause/reprise, fichiers, G-code : Moonraker seulement (409 sinon). */
export function hasLiveControls(
	printer: Pick<PrinterResponse, 'host_type' | 'moonraker_url'>
): boolean {
	return hostMeta(printer.host_type).id === 'moonraker' && !!printer.moonraker_url?.trim();
}

/** Formulaire de connexion (champs vides = non renseignés). */
export interface HostForm {
	type: HostType;
	url: string;
	apiKey: string;
	/** « key » ou « user » (PrusaLink / PrusaConnect). */
	auth: 'key' | 'user';
	user: string;
	password: string;
	slug: string;
}

export function emptyHostForm(type: HostType = 'moonraker'): HostForm {
	return { type, url: '', apiKey: '', auth: 'key', user: '', password: '', slug: '' };
}

/** Formulaire initial depuis une imprimante (les secrets ne sont jamais renvoyés). */
export function hostFormOf(p: PrinterResponse): HostForm {
	const meta = hostMeta(p.host_type);
	return {
		type: meta.id,
		url: p.moonraker_url ?? '',
		apiKey: '',
		auth: p.host_auth === 'user' ? 'user' : 'key',
		user: p.host_user ?? '',
		password: '',
		slug: p.host_port ?? ''
	};
}

/** Adresse normalisée : schéma ajouté si absent, espaces et « / » final retirés. */
export function normalizeHostUrl(raw: string): string {
	const s = raw.trim().replace(/\/+$/, '');
	if (!s) return '';
	return /^[a-z][a-z0-9+.-]*:\/\//i.test(s) ? s : `http://${s}`;
}

/** Erreur de saisie bloquante (null = valide). */
export function hostFormError(f: HostForm): string | null {
	const url = normalizeHostUrl(f.url);
	if (url) {
		try {
			new URL(url);
		} catch {
			return 'Adresse invalide.';
		}
	}
	const meta = hostMeta(f.type);
	if (f.type !== 'moonraker' && !url) return `Adresse de l'hôte ${meta.label} requise.`;
	if (meta.authChoice && f.auth === 'user' && !f.user.trim()) return 'Utilisateur requis.';
	return null;
}

/**
 * Champs d'hôte du corps `POST/PUT /api/printers`. Secret vide = conservé côté
 * serveur (absent du corps) ; les champs non pertinents pour le type sont vidés.
 */
export function hostRequest(
	f: HostForm
): Pick<
	SavePrinterRequest,
	| 'moonraker_url'
	| 'api_key'
	| 'host_type'
	| 'host_auth'
	| 'host_user'
	| 'host_password'
	| 'host_port'
> {
	const meta = hostMeta(f.type);
	const userAuth = meta.authChoice && f.auth === 'user';
	return {
		host_type: meta.id,
		moonraker_url: normalizeHostUrl(f.url) || undefined,
		...(meta.secret && !userAuth && f.apiKey ? { api_key: f.apiKey } : {}),
		host_auth: meta.authChoice ? f.auth : '',
		host_user: userAuth ? f.user.trim() : '',
		...(userAuth && f.password ? { host_password: f.password } : {}),
		host_port: meta.printerSlug ? f.slug.trim() : ''
	};
}

/** Libellé du résultat de test (Moonraker : état Klipper ; autres : nom/version de l'hôte). */
export function testMessage(
	type: string,
	r: { connected: boolean; klippy_state: string; moonraker_version: string }
): string {
	const meta = hostMeta(type);
	if (meta.id === 'moonraker')
		return r.connected
			? `Connecté (Klipper ${r.klippy_state}, Moonraker ${r.moonraker_version})`
			: `Injoignable (${r.klippy_state})`;
	if (!r.connected) return `${meta.label} injoignable`;
	return `Connecté à ${r.klippy_state || meta.label}${r.moonraker_version ? ` ${r.moonraker_version}` : ''}`;
}
