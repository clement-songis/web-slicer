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
	/** Où trouver l'adresse et les identifiants (aide sous le formulaire). */
	help: string;
}

export const HOST_TYPES: readonly HostMeta[] = [
	{
		id: 'moonraker',
		label: 'Moonraker (Klipper)',
		placeholder: 'http://klipper.local:7125',
		secret: 'Clé API (facultatif)',
		authChoice: false,
		printerSlug: false,
		storages: false,
		help: "Adresse de Moonraker (port 7125 par défaut). Clé API seulement si l'autorisation est activée dans moonraker.conf."
	},
	{
		id: 'octoprint',
		label: 'OctoPrint',
		placeholder: 'http://octopi.local',
		secret: 'Clé API',
		authChoice: false,
		printerSlug: false,
		storages: false,
		help: "Clé API : OctoPrint → Paramètres → API (clé globale ou clé d'application)."
	},
	{
		id: 'prusalink',
		label: 'PrusaLink',
		placeholder: 'http://192.168.1.30',
		secret: 'Clé API',
		authChoice: true,
		printerSlug: false,
		storages: true,
		help: "Sur l'imprimante : Réglages → Réseau → PrusaLink. Utilisateur « maker » et mot de passe affiché, ou clé API (anciens firmwares)."
	},
	{
		id: 'prusaconnect',
		label: 'PrusaConnect',
		placeholder: 'https://connect.prusa3d.com',
		secret: 'Clé API',
		authChoice: true,
		printerSlug: false,
		storages: true,
		help: "Clé API de l'imprimante, dans Prusa Connect → Imprimante → Réglages."
	},
	{
		id: 'duet',
		label: 'Duet (RepRapFirmware)',
		placeholder: 'http://duet.local',
		secret: 'Mot de passe (facultatif)',
		authChoice: false,
		printerSlug: false,
		storages: false,
		help: "Mot de passe défini par M551 (laisser vide s'il n'y en a pas). RepRapFirmware autonome ou DSF."
	},
	{
		id: 'repetier',
		label: 'Repetier-Server',
		placeholder: 'http://repetier.local:3344',
		secret: 'Clé API',
		authChoice: false,
		printerSlug: true,
		storages: true,
		help: "Clé API : Repetier-Server → Paramètres globaux → Connectivité. Choisissez ensuite l'imprimante du serveur."
	},
	{
		id: 'astrobox',
		label: 'AstroBox',
		placeholder: 'http://astrobox.local',
		secret: 'Clé API',
		authChoice: false,
		printerSlug: false,
		storages: false,
		help: 'Clé API : AstroBox → Paramètres → Avancé.'
	},
	{
		id: 'mks',
		label: 'MKS (WiFi)',
		placeholder: 'http://192.168.1.40',
		secret: null,
		authChoice: false,
		printerSlug: false,
		storages: false,
		help: 'Module WiFi MKS : adresse IP seule ; le lancement passe par la console TCP (port 8080).'
	},
	{
		id: 'flashair',
		label: 'FlashAir',
		placeholder: 'http://flashair',
		secret: null,
		authChoice: false,
		printerSlug: false,
		storages: false,
		help: 'Carte FlashAir configurée avec UPLOAD=1 ; dépôt du fichier seulement, sans lancement.'
	},
	{
		id: 'esp3d',
		label: 'ESP3D',
		placeholder: 'http://esp3d.local',
		secret: null,
		authChoice: false,
		printerSlug: false,
		storages: false,
		help: 'Carte ESP3D : adresse IP seule ; les noms de fichiers sont raccourcis au format 8.3.'
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
	const name = r.klippy_state || meta.label;
	const version = r.moonraker_version?.trim() ?? '';
	// « OctoPrint 1.10.2 » / « PrusaLink 2.1.2 » : la version porte déjà le nom.
	if (version && version.toLowerCase().startsWith(name.split(' ')[0].toLowerCase()))
		return `Connecté à ${version}`;
	return `Connecté à ${name}${version ? ` ${version}` : ''}`;
}

/** Résultat de test affichable : succès ou échec, avec la cause. */
export interface TestOutcome {
	ok: boolean;
	message: string;
}

export function testOutcome(
	type: string,
	r: { connected: boolean; klippy_state: string; moonraker_version: string }
): TestOutcome {
	return { ok: r.connected, message: testMessage(type, r) };
}

/** Échec de test : cause renvoyée par le serveur (clé refusée, hôte injoignable…). */
export function testFailure(type: string, cause: string | null | undefined): TestOutcome {
	const label = hostMeta(type).label;
	const why = (cause ?? '').trim();
	if (!why) return { ok: false, message: `Échec de connexion à ${label}` };
	// 401/403 : identifiants refusés, dit en clair (la réponse brute suit).
	const denied = /HTTP (401|403)\b/.exec(why);
	if (denied)
		return {
			ok: false,
			message: `Échec — identifiants refusés par ${label} (HTTP ${denied[1]}). Vérifiez la clé ou le mot de passe.`
		};
	return { ok: false, message: `Échec — ${why}` };
}

/** Ce que permet un hôte (affiché sous le choix du type, comme une fiche). */
export interface HostCapability {
	id: 'upload' | 'start' | 'storage' | 'live';
	label: string;
	ok: boolean;
}

export function hostCapabilities(type: string): HostCapability[] {
	const meta = hostMeta(type);
	return [
		{ id: 'upload', label: 'Envoi du G-code', ok: true },
		// FlashAir : dépôt seul (Orca : pas de post-action).
		{ id: 'start', label: "Lancement de l'impression", ok: meta.id !== 'flashair' },
		{
			id: 'storage',
			label: meta.id === 'repetier' ? 'Choix du groupe' : 'Choix du stockage',
			ok: meta.storages
		},
		{ id: 'live', label: 'Suivi en direct, pause, fichiers', ok: meta.id === 'moonraker' }
	];
}

/** Types d'hôte regroupés pour le sélecteur (contrôle complet / envoi seul). */
export const HOST_GROUPS: readonly { label: string; hosts: readonly HostMeta[] }[] = [
	{ label: 'Suivi et contrôle complets', hosts: HOST_TYPES.filter((h) => h.id === 'moonraker') },
	{ label: 'Envoi et lancement', hosts: HOST_TYPES.filter((h) => h.id !== 'moonraker') }
];

/** Résumé des identifiants enregistrés (jamais le secret lui-même). */
export interface CredentialChip {
	text: string;
	/** Identifiant attendu mais absent. */
	missing?: boolean;
}

export function credentialSummary(
	p: Pick<
		PrinterResponse,
		| 'host_type'
		| 'moonraker_url'
		| 'has_api_key'
		| 'host_auth'
		| 'host_user'
		| 'has_host_password'
		| 'host_port'
	>
): CredentialChip[] {
	if (!p.moonraker_url?.trim()) return [];
	const meta = hostMeta(p.host_type);
	const chips: CredentialChip[] = [];
	if (meta.authChoice && p.host_auth === 'user') {
		chips.push({ text: `Utilisateur « ${p.host_user ?? ''} »`, missing: !p.host_user });
		chips.push(
			p.has_host_password
				? { text: 'Mot de passe enregistré' }
				: { text: 'Sans mot de passe', missing: true }
		);
	} else if (meta.secret) {
		const optional = /facultatif/i.test(meta.secret);
		const noun = meta.id === 'duet' ? 'Mot de passe' : 'Clé API';
		if (p.has_api_key) chips.push({ text: `${noun} enregistré${noun === 'Clé API' ? 'e' : ''}` });
		else if (!optional) chips.push({ text: `${noun} manquante`, missing: true });
	}
	if (meta.printerSlug)
		chips.push(
			p.host_port
				? { text: `Imprimante « ${p.host_port} »` }
				: { text: 'Imprimante du serveur à choisir', missing: true }
		);
	return chips;
}
