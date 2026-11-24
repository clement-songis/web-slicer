// Hôtes d'impression (Orca : PhysicalPrinterDialog, host_type) : types pris en
// charge par le serveur (lot 17), champs utiles à chacun, corps de requête.
// Pur → testable.
import type { PrinterResponse, SavePrinterRequest } from '#lib/api/types';
import { tr } from '#lib/i18n';

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
	| 'esp3d'
	| 'obico'
	| 'crealityprint'
	| 'flashforge'
	| 'elegoolink';

export interface HostMeta {
	id: HostType;
	/** Nom affiché par Orca (PrintHost::get_name). */
	label: string;
	/** Exemple d'adresse. */
	placeholder: string;
	/** Libellé du champ secret (printhost_apikey, clé anglaise à traduire au rendu), null = sans secret. */
	secret: string | null;
	/** Choix clé API / utilisateur + mot de passe (PrusaLink, PrusaConnect). */
	authChoice: boolean;
	/** Imprimante choisie sur le serveur (printhost_port) : slug Repetier, ID Obico. */
	printerSlug: boolean;
	/** Libellé du champ printhost_port (défaut : « Imprimante sur le serveur »). */
	slugLabel?: string;
	/** Numéro de série de l'imprimante (Flashforge : flashforge_serial_number). */
	serialNumber?: boolean;
	/** Destinations proposées à l'envoi (stockages PrusaLink, groupes Repetier). */
	storages: boolean;
	/** « Envoyer et mettre en file » (Orca PrintHostPostUploadAction::QueuePrint,
	 *  get_post_upload_actions) : PrusaConnect seulement. */
	queue?: boolean;
	/** Où trouver l'adresse et les identifiants (aide sous le formulaire ; clé anglaise, traduite au rendu). */
	help: string;
}

export const HOST_TYPES: readonly HostMeta[] = [
	{
		id: 'moonraker',
		label: 'Moonraker (Klipper)',
		placeholder: 'http://klipper.local:7125',
		secret: 'API key (optional)',
		authChoice: false,
		printerSlug: false,
		storages: false,
		help: 'Moonraker address (port 7125 by default). API key only if authorization is enabled in moonraker.conf.'
	},
	{
		id: 'octoprint',
		label: 'OctoPrint',
		placeholder: 'http://octopi.local',
		secret: 'API key',
		authChoice: false,
		printerSlug: false,
		storages: false,
		help: 'API key: OctoPrint → Settings → API (global key or application key).'
	},
	{
		id: 'prusalink',
		label: 'PrusaLink',
		placeholder: 'http://192.168.1.30',
		secret: 'API key',
		authChoice: true,
		printerSlug: false,
		storages: true,
		help: 'On the printer: Settings → Network → PrusaLink. User “maker” and the displayed password, or API key (older firmware).'
	},
	{
		id: 'prusaconnect',
		label: 'PrusaConnect',
		placeholder: 'https://connect.prusa3d.com',
		secret: 'API key',
		authChoice: true,
		printerSlug: false,
		// Orca PrusaConnect::get_storage → false : pas de choix du stockage.
		storages: false,
		// PrusaConnect::get_post_upload_actions : StartPrint | QueuePrint (to_queue).
		queue: true,
		help: 'Printer API key, in Prusa Connect → Printer → Settings.'
	},
	{
		id: 'duet',
		label: 'Duet (RepRapFirmware)',
		placeholder: 'http://duet.local',
		secret: 'Password (optional)',
		authChoice: false,
		printerSlug: false,
		storages: false,
		help: 'Password set by M551 (leave empty if there is none). Standalone RepRapFirmware or DSF.'
	},
	{
		id: 'repetier',
		label: 'Repetier-Server',
		placeholder: 'http://repetier.local:3344',
		secret: 'API key',
		authChoice: false,
		printerSlug: true,
		storages: true,
		help: 'API key: Repetier-Server → Global settings → Connectivity. Then choose the server printer.'
	},
	{
		id: 'astrobox',
		label: 'AstroBox',
		placeholder: 'http://astrobox.local',
		secret: 'API key',
		authChoice: false,
		printerSlug: false,
		storages: false,
		help: 'API key: AstroBox → Settings → Advanced.'
	},
	{
		id: 'mks',
		label: 'MKS (WiFi)',
		placeholder: 'http://192.168.1.40',
		secret: null,
		authChoice: false,
		printerSlug: false,
		storages: false,
		help: 'MKS WiFi module: IP address only; printing is started through the TCP console (port 8080).'
	},
	{
		id: 'flashair',
		label: 'FlashAir',
		placeholder: 'http://flashair',
		secret: null,
		authChoice: false,
		printerSlug: false,
		storages: false,
		help: 'FlashAir card configured with UPLOAD=1; file upload only, without starting the print.'
	},
	{
		id: 'esp3d',
		label: 'ESP3D',
		placeholder: 'http://esp3d.local',
		secret: null,
		authChoice: false,
		printerSlug: false,
		storages: false,
		help: 'ESP3D board: IP address only; file names are shortened to the 8.3 format.'
	},
	{
		id: 'obico',
		label: 'Obico',
		placeholder: 'https://app.obico.io',
		secret: 'API key',
		authChoice: false,
		printerSlug: true,
		// Évalué à la lecture (langue courante).
		get slugLabel() {
			return tr('Printer ID');
		},
		storages: false,
		help: 'Obico server address (cloud or self-hosted) and the account API token (Obico → Preferences → API). Then “List” to choose the printer: its ID is saved.'
	},
	{
		id: 'crealityprint',
		label: 'Creality Print (K2, Sparkx)',
		placeholder: 'http://192.168.1.60',
		secret: 'API key (optional)',
		authChoice: false,
		printerSlug: false,
		storages: false,
		help: 'Creality K2 / K2 Plus / K2 Pro / Sparkx i7 printers: IP address shown on the screen (Settings → Network). Printing is started through port 9999 of the printer.'
	},
	{
		id: 'flashforge',
		label: 'Flashforge',
		placeholder: '192.168.1.70',
		secret: 'Access code',
		authChoice: false,
		printerSlug: false,
		serialNumber: true,
		storages: false,
		help: 'Printer IP address. Serial number and access code: printer screen → Settings → Network (“LAN only” mode). Without them, sending goes through the legacy console (port 8899).'
	},
	{
		id: 'elegoolink',
		label: 'ElegooLink',
		placeholder: 'http://192.168.1.80',
		secret: 'Access code (optional)',
		authChoice: false,
		printerSlug: false,
		storages: false,
		help: 'IP address shown on the printer screen. The protocol follows the machine profile model: Centauri Carbon (upload + start), Centauri Carbon 2 (access code, 123456 by default; upload only), other models via OctoPrint.'
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
	/** Numéro de série (Flashforge). */
	serial: string;
}

export function emptyHostForm(type: HostType = 'moonraker'): HostForm {
	return { type, url: '', apiKey: '', auth: 'key', user: '', password: '', slug: '', serial: '' };
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
		slug: p.host_port ?? '',
		serial: p.host_serial_number ?? ''
	};
}

/** Adresse normalisée : schéma ajouté si absent, espaces et « / » final retirés. */
export function normalizeHostUrl(raw: string): string {
	const s = raw.trim().replace(/\/+$/, '');
	if (!s) return '';
	return /^[a-z][a-z0-9+.-]*:\/\//i.test(s) ? s : `http://${s}`;
}

// — Réglages d'hôte portés par le profil machine (Orca : printhost_port,
// printhost_authorization_type, printhost_user, printhost_password,
// flashforge_serial_number, printer_model). Le serveur les utilise quand le
// champ correspondant de l'imprimante est vide.
export interface ProfileHost {
	port?: string;
	auth?: 'key' | 'user';
	user?: string;
	hasPassword?: boolean;
	serial?: string;
	model?: string;
}

const first = (v: unknown): string => {
	const x = Array.isArray(v) ? v[0] : v;
	return typeof x === 'string' ? x.trim() : typeof x === 'number' ? String(x) : '';
};

/** Réglages d'hôte lus dans les valeurs résolues d'un preset machine. */
export function profileHost(values: Record<string, unknown> | null | undefined): ProfileHost {
	if (!values) return {};
	const out: ProfileHost = {};
	const port = first(values.printhost_port);
	const auth = first(values.printhost_authorization_type);
	const user = first(values.printhost_user);
	const serial = first(values.flashforge_serial_number);
	const model = first(values.printer_model);
	if (port) out.port = port;
	if (auth === 'user' || auth === 'key') out.auth = auth;
	if (user) out.user = user;
	if (first(values.printhost_password)) out.hasPassword = true;
	if (serial) out.serial = serial;
	if (model) out.model = model;
	return out;
}

/** Placeholder d'un champ laissé vide : valeur du profil machine, s'il en a une. */
export function profilePlaceholder(value: string | undefined | null, fallback = ''): string {
	return value ? tr('from the machine profile: {value}', { value }) : fallback;
}

/** Protocole ElegooLink selon printer_model (Orca classify_printer_model). */
export type ElegooProtocol = 'cc' | 'cc2' | 'octoprint';
export function elegooProtocol(model: string | undefined | null): ElegooProtocol {
	const m = model ?? '';
	if (!m.startsWith('Elegoo Centauri')) return 'octoprint';
	return m.trimEnd().endsWith('2') ? 'cc2' : 'cc';
}
/** Clés anglaises, traduites au rendu. */
export const ELEGOO_PROTOCOL_LABELS: Record<ElegooProtocol, string> = {
	cc: 'Centauri Carbon: chunked upload and start (SDCP, port 3030)',
	cc2: 'Centauri Carbon 2: upload with access code, without automatic start',
	octoprint: 'OctoPrint protocol (optional API key)'
};

/** Erreur de saisie bloquante (null = valide). */
export function hostFormError(f: HostForm, profile: ProfileHost = {}): string | null {
	const url = normalizeHostUrl(f.url);
	if (url) {
		try {
			new URL(url);
		} catch {
			return tr('Invalid address.');
		}
	}
	const meta = hostMeta(f.type);
	if (f.type !== 'moonraker' && !url)
		return tr('{host} host address required.', { host: meta.label });
	if (meta.authChoice && f.auth === 'user' && !f.user.trim() && !profile.user)
		return tr('User required (missing from the machine profile).');
	if (meta.serialNumber && f.apiKey.trim() && !f.serial.trim() && !profile.serial)
		return tr('Serial number required with the access code.');
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
	| 'host_serial_number'
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
		host_port: meta.printerSlug ? f.slug.trim() : '',
		// Flashforge : enregistré dans host_options.serial_number (vide = conservé).
		...(meta.serialNumber && f.serial.trim() ? { host_serial_number: f.serial.trim() } : {})
	};
}

/**
 * Valeur enregistrée pour une entrée de la liste des imprimantes du serveur :
 * Obico renvoie « nom [id] » et attend l'ID (printer_id = printhost_port).
 */
export function hostPrinterValue(type: string, entry: string): string {
	if (hostMeta(type).id === 'obico') return /\[([^\]]+)\]\s*$/.exec(entry)?.[1] ?? entry;
	return entry;
}

/** Libellé du résultat de test (Moonraker : état Klipper ; autres : nom/version de l'hôte). */
export function testMessage(
	type: string,
	r: { connected: boolean; klippy_state: string; moonraker_version: string }
): string {
	const meta = hostMeta(type);
	if (meta.id === 'moonraker')
		return r.connected
			? tr('Connected (Klipper {state}, Moonraker {version})', {
					state: r.klippy_state,
					version: r.moonraker_version
				})
			: tr('Unreachable ({state})', { state: r.klippy_state });
	if (!r.connected) return tr('{host} unreachable', { host: meta.label });
	const name = r.klippy_state || meta.label;
	const version = r.moonraker_version?.trim() ?? '';
	// « OctoPrint 1.10.2 » / « PrusaLink 2.1.2 » : la version porte déjà le nom.
	if (version && version.toLowerCase().startsWith(name.split(' ')[0].toLowerCase()))
		return tr('Connected to {host}', { host: version });
	return tr('Connected to {host}', { host: `${name}${version ? ` ${version}` : ''}` });
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
	if (!why) return { ok: false, message: tr('Failed to connect to {host}', { host: label }) };
	// 401/403 : identifiants refusés, dit en clair (la réponse brute suit).
	const denied = /HTTP (401|403)\b/.exec(why);
	if (denied)
		return {
			ok: false,
			message: tr(
				'Failed — credentials rejected by {host} (HTTP {code}). Check the key or the password.',
				{ host: label, code: denied[1] }
			)
		};
	return { ok: false, message: tr('Failed — {reason}', { reason: why }) };
}

/** Ce que permet un hôte (affiché sous le choix du type, comme une fiche). */
export interface HostCapability {
	id: 'upload' | 'start' | 'storage' | 'live';
	label: string;
	ok: boolean;
}

export function hostCapabilities(type: string, model?: string | null): HostCapability[] {
	const meta = hostMeta(type);
	// FlashAir, ElegooLink CC2 : dépôt seul (Orca : pas de post-action).
	const noStart =
		meta.id === 'flashair' || (meta.id === 'elegoolink' && elegooProtocol(model) === 'cc2');
	return [
		{ id: 'upload', label: tr('G-code upload'), ok: true },
		{ id: 'start', label: tr('Print start'), ok: !noStart },
		{
			id: 'storage',
			label: meta.id === 'repetier' ? tr('Group selection') : tr('Storage selection'),
			ok: meta.storages
		},
		{ id: 'live', label: tr('Live monitoring, pause, files'), ok: meta.id === 'moonraker' }
	];
}

/** Types d'hôte regroupés pour le sélecteur (contrôle complet / envoi seul ; libellés
 *  = clés anglaises traduites au rendu). */
export const HOST_GROUPS: readonly { label: string; hosts: readonly HostMeta[] }[] = [
	{ label: 'Full monitoring and control', hosts: HOST_TYPES.filter((h) => h.id === 'moonraker') },
	{ label: 'Upload and start', hosts: HOST_TYPES.filter((h) => h.id !== 'moonraker') }
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
		chips.push({ text: tr('User “{user}”', { user: p.host_user ?? '' }), missing: !p.host_user });
		chips.push(
			p.has_host_password
				? { text: tr('Password saved') }
				: { text: tr('No password'), missing: true }
		);
	} else if (meta.secret) {
		const optional = /optional/i.test(meta.secret);
		const noun =
			meta.id === 'duet' ? 'Password' : meta.id === 'flashforge' ? 'Access code' : 'API key';
		if (p.has_api_key) chips.push({ text: tr(`${noun} saved`) });
		else if (meta.id === 'flashforge') chips.push({ text: tr('No code: console 8899') });
		else if (!optional) chips.push({ text: tr(`${noun} missing`), missing: true });
	}
	if (meta.printerSlug)
		chips.push(
			p.host_port
				? {
						text:
							meta.id === 'obico'
								? tr('Printer ID “{id}”', { id: p.host_port })
								: tr('Printer “{id}”', { id: p.host_port })
					}
				: { text: tr('Server printer to choose'), missing: true }
		);
	return chips;
}

/** Service à rechercher pour un type d'hôte (Orca : Bonjour("octoprint") ;
 *  Creality Print : CrealityDiscoveryDialog ; Flashforge : discover_printers),
 *  ou null si l'hôte ne s'annonce pas. */
export function discoveryService(type: string): string | null {
	if (type === 'octoprint') return 'octoprint';
	if (type === 'moonraker') return 'moonraker';
	if (type === 'crealityprint') return 'creality';
	if (type === 'flashforge') return 'flashforge';
	return null;
}

/** Durée d'écoute côté serveur (ms) : Orca écoute 5 s pour Creality (cxmdns),
 *  10 s par essai pour Flashforge (3 essais si rien ne répond), 4 s sinon. */
export function discoveryTimeoutMs(service: string): number {
	if (service === 'creality') return 5000;
	if (service === 'flashforge') return 10_000;
	return 4000;
}

/** Texte d'attente et d'absence de résultat selon le service recherché. */
export function discoveryHints(service: string): { waiting: string; none: string } {
	if (service === 'creality')
		return {
			waiting: tr('Searching for Creality printers (K series) on the local network (about 5 s)…'),
			none: tr(
				'No Creality printer found. The printer must be on the same local network as the web-slicer server, without Wi-Fi client isolation.'
			)
		};
	if (service === 'flashforge')
		return {
			waiting: tr(
				'Searching for Flashforge printers on the local network (up to 30 s if none responds)…'
			),
			none: tr(
				'No Flashforge printer found. The printer must be on the same local network as the web-slicer server (UDP broadcast).'
			)
		};
	return {
		waiting: tr('Searching for servers announced on the local network (a few seconds)…'),
		none: tr(
			'No server found. The search starts from the web-slicer server: the printer must be on the same local network and announce its service (mDNS).'
		)
	};
}
