// Préférences applicatives (menu « Preferences ») : sous-ensemble de
// PreferencesDialog.cpp d'Orca qui a un sens dans un navigateur — général,
// contrôle (comportement, tranchage, caméra/souris), presets, choix mémorisés.
// Persistées **sur le serveur, par compte** (GET/PUT /api/me/preferences :
// lecture au changement de session, écriture différée) ; localStorage sert de
// cache (clé suffixée par l'id du compte + copie sous la clé historique). Si
// le serveur n'a encore rien ({}), les préférences locales y sont migrées une
// fois. Le thème et la langue délèguent à leurs modules (`$lib/theme`, `$lib/i18n`).
//
// Les réglages lus hors de la page (scène 3D, aperçu, sélecteurs de presets)
// passent par le store `appPreferences`.

import { get, writable } from 'svelte/store';

export type Units = 'mm' | 'in';
/** Orca `camera_navigation_style` : 0 = Default, 1 = Touchpad. */
export type CameraStyle = 'default' | 'touchpad';
/** Orca `*_mouse_drag_action` : 0 = None, 1 = Pan, 2 = Rotate. */
export type DragAction = 'none' | 'pan' | 'rotate';
/** Orca `auto_calculate_flush` : « all », « color change », « disabled ». */
export type AutoFlush = 'all' | 'color' | 'disabled';
/** Orca `default_page` : 0 = Accueil, 1 = Préparer. */
export type DefaultPage = 'home' | 'prepare';

export interface CameraPreferences {
	style: CameraStyle;
	/** Orca `camera_orbit_mult` (0,05–2). */
	orbitSpeed: number;
	/** Orca `zoom_to_mouse`. */
	zoomToMouse: boolean;
	/** Orca `use_free_camera` : rotation libre (boule de commande) au lieu d'une orbite à Z vertical. */
	freeCamera: boolean;
	/** Orca `reverse_mouse_wheel_zoom`. */
	reverseWheel: boolean;
	leftDrag: DragAction;
	middleDrag: DragAction;
	rightDrag: DragAction;
}

export interface AppPreferences {
	units: Units;
	/** Avertir avant de quitter la page (Orca : « Show warning before close »). */
	confirmClose: boolean;
	/** Panneau latéral de l'éditeur replié (Shift+Tab, groupe Plater). */
	sidebarCollapsed: boolean;
	/** Devise des coûts (Orca : `filament_cost` en « money/kg », sans devise :
	 *  l'unité monétaire est celle saisie dans le filament ; on l'affiche). */
	currency: string;
	camera: CameraPreferences;
	/** Orca `auto_arrange` : arranger le plateau après un clonage / un ajout. */
	autoArrange: boolean;
	/** Orca `auto_calculate_flush`. */
	autoFlush: AutoFlush;
	/** Orca `show_unsupported_presets` : presets incompatibles listés, non sélectionnables. */
	showUnsupportedPresets: boolean;
	/** Proposer la conversion pouces/mètres à l'import (Orca : toujours proposé). */
	unitConversionPrompt: boolean;
	/** Orca `keep_painting` : peinture conservée après Simplifier / Réparer. */
	keepPainting: boolean;
	/** Orca `enable_high_low_temp_mixed_printing` : lève le refus des filaments haute/basse température. */
	mixedTemperature: boolean;
	/** Orca `pop_up_filament_map_dialog` : proposer la répartition des filaments par buse
	 *  avant chaque tranchage (imprimantes à deux buses). */
	filamentMapPopup: boolean;
	/** Orca `default_page` : page ouverte au démarrage (après connexion ou à la
	 *  racine du site) — bibliothèque, ou dernier projet modifié. */
	defaultPage: DefaultPage;
	/** Orca `max_recent_count` (0–999, défaut 18) : projets de la vue « Récemment
	 *  ouvert » de la bibliothèque ; les autres restent accessibles. */
	maxRecent: number;
}

export const PREFERENCES_KEY = 'web-slicer.preferences';

export const CAMERA_DEFAULTS: CameraPreferences = {
	style: 'default',
	orbitSpeed: 1,
	zoomToMouse: false,
	freeCamera: false,
	reverseWheel: false,
	leftDrag: 'rotate',
	middleDrag: 'pan',
	rightDrag: 'pan'
};

export const DEFAULT_PREFERENCES: AppPreferences = {
	units: 'mm',
	confirmClose: true,
	sidebarCollapsed: false,
	currency: '€',
	camera: { ...CAMERA_DEFAULTS },
	autoArrange: true,
	autoFlush: 'all',
	showUnsupportedPresets: false,
	unitConversionPrompt: true,
	keepPainting: false,
	mixedTemperature: false,
	filamentMapPopup: false,
	defaultPage: 'home',
	maxRecent: 18
};

/** Bornes d'Orca (MainFrame::set_max_recent_count : 0–999). */
export function clampRecentCount(value: number): number {
	if (!Number.isFinite(value)) return DEFAULT_PREFERENCES.maxRecent;
	return Math.round(Math.min(999, Math.max(0, value)));
}

const ORBIT_MIN = 0.05;
const ORBIT_MAX = 2;

/** Bornes d'Orca (create_camera_orbit_mult_input : 0,05–2, 2 décimales). */
export function clampOrbitSpeed(value: number): number {
	if (!Number.isFinite(value)) return CAMERA_DEFAULTS.orbitSpeed;
	return Math.round(Math.min(ORBIT_MAX, Math.max(ORBIT_MIN, value)) * 100) / 100;
}

const bool = (v: unknown, d: boolean) => (typeof v === 'boolean' ? v : d);
const oneOf = <T extends string>(v: unknown, allowed: readonly T[], d: T): T =>
	allowed.includes(v as T) ? (v as T) : d;
const DRAGS = ['none', 'pan', 'rotate'] as const;

function parseCamera(raw: unknown): CameraPreferences {
	const c = (raw && typeof raw === 'object' ? raw : {}) as Partial<CameraPreferences>;
	return {
		style: oneOf(c.style, ['default', 'touchpad'] as const, CAMERA_DEFAULTS.style),
		orbitSpeed:
			typeof c.orbitSpeed === 'number' ? clampOrbitSpeed(c.orbitSpeed) : CAMERA_DEFAULTS.orbitSpeed,
		zoomToMouse: bool(c.zoomToMouse, CAMERA_DEFAULTS.zoomToMouse),
		freeCamera: bool(c.freeCamera, CAMERA_DEFAULTS.freeCamera),
		reverseWheel: bool(c.reverseWheel, CAMERA_DEFAULTS.reverseWheel),
		leftDrag: oneOf(c.leftDrag, DRAGS, CAMERA_DEFAULTS.leftDrag),
		middleDrag: oneOf(c.middleDrag, DRAGS, CAMERA_DEFAULTS.middleDrag),
		rightDrag: oneOf(c.rightDrag, DRAGS, CAMERA_DEFAULTS.rightDrag)
	};
}

/** Lecture tolérante (valeurs inconnues → défauts). */
export function parsePreferences(raw: string | null): AppPreferences {
	try {
		const value = raw ? (JSON.parse(raw) as Partial<AppPreferences>) : {};
		const d = DEFAULT_PREFERENCES;
		return {
			units: value.units === 'in' ? 'in' : 'mm',
			confirmClose: bool(value.confirmClose, d.confirmClose),
			sidebarCollapsed: bool(value.sidebarCollapsed, d.sidebarCollapsed),
			currency:
				typeof value.currency === 'string' && value.currency.trim()
					? value.currency.trim().slice(0, 4)
					: d.currency,
			camera: parseCamera(value.camera),
			autoArrange: bool(value.autoArrange, d.autoArrange),
			autoFlush: oneOf(value.autoFlush, ['all', 'color', 'disabled'] as const, d.autoFlush),
			showUnsupportedPresets: bool(value.showUnsupportedPresets, d.showUnsupportedPresets),
			unitConversionPrompt: bool(value.unitConversionPrompt, d.unitConversionPrompt),
			keepPainting: bool(value.keepPainting, d.keepPainting),
			mixedTemperature: bool(value.mixedTemperature, d.mixedTemperature),
			filamentMapPopup: bool(value.filamentMapPopup, d.filamentMapPopup),
			defaultPage: oneOf(value.defaultPage, ['home', 'prepare'] as const, d.defaultPage),
			maxRecent:
				typeof value.maxRecent === 'number' ? clampRecentCount(value.maxRecent) : d.maxRecent
		};
	} catch {
		return { ...DEFAULT_PREFERENCES, camera: { ...CAMERA_DEFAULTS } };
	}
}

// — Persistance par utilisateur —————————————————————————————————————————
let userScope: string | null = null;

/** Clé de stockage : `web-slicer.preferences:<utilisateur>` si connecté. */
export function preferencesKey(user: string | null = userScope): string {
	return user ? `${PREFERENCES_KEY}:${user}` : PREFERENCES_KEY;
}

function readRaw(user: string | null): string | null {
	try {
		const own = localStorage.getItem(preferencesKey(user));
		if (own !== null || !user) return own ?? localStorage.getItem(PREFERENCES_KEY);
		// Migration : préférences d'avant la persistance par utilisateur (aucune
		// clé par compte encore écrite dans ce navigateur), reprises une fois.
		const anyUser = Object.keys(localStorage).some((k) => k.startsWith(`${PREFERENCES_KEY}:`));
		return anyUser ? null : localStorage.getItem(PREFERENCES_KEY);
	} catch {
		return null;
	}
}

/** Préférences courantes, partagées (scène 3D, aperçu, sélecteurs). */
export const appPreferences = writable<AppPreferences>(parsePreferences(null));

// — Synchronisation serveur ————————————————————————————————————————————
/** Transport des préférences du compte (GET/PUT /api/me/preferences). */
export interface PreferencesRemote {
	load(): Promise<Record<string, unknown>>;
	save(doc: Record<string, unknown>, opts?: { keepalive?: boolean }): Promise<void>;
}

/** Délai d'écriture différée (les réglages en rafale ne font qu'un PUT). */
export const SAVE_DELAY_MS = 800;
const MIGRATED_KEY = `${PREFERENCES_KEY}.migrated`;

const httpRemote: PreferencesRemote = {
	async load() {
		const res = await fetch('/api/me/preferences', { credentials: 'include' });
		if (!res.ok) throw new Error(`préférences : HTTP ${res.status}`);
		const doc = (await res.json()) as unknown;
		return doc && typeof doc === 'object' && !Array.isArray(doc)
			? (doc as Record<string, unknown>)
			: {};
	},
	async save(doc, opts) {
		const res = await fetch('/api/me/preferences', {
			method: 'PUT',
			credentials: 'include',
			keepalive: opts?.keepalive,
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify(doc)
		});
		if (!res.ok) throw new Error(`préférences : HTTP ${res.status}`);
	}
};

// Navigateur seulement (les tests unitaires tournent sous Node sans backend).
let remote: PreferencesRemote | null = typeof window !== 'undefined' ? httpRemote : null;
/** Dernier document serveur : ses clés inconnues sont conservées à l'écriture. */
let serverDoc: Record<string, unknown> = {};
/** Génération de session : une lecture serveur périmée est ignorée. */
let generation = 0;
/** Modifié localement depuis le début de la lecture serveur : le local gagne. */
let dirty = false;
let saveTimer: ReturnType<typeof setTimeout> | null = null;
let pending: AppPreferences | null = null;
let inflight: Promise<void> = Promise.resolve();

/** Remplace le transport (tests) ; `null` = local seulement. */
export function setPreferencesRemote(next: PreferencesRemote | null): void {
	remote = next;
}

function writeCache(prefs: AppPreferences): void {
	try {
		localStorage.setItem(preferencesKey(), JSON.stringify(prefs));
		// Copie sous la clé historique (dernier compte actif) : compatibilité avec
		// les lecteurs existants ; la clé par compte reste la référence.
		if (userScope) localStorage.setItem(PREFERENCES_KEY, JSON.stringify(prefs));
	} catch {
		// Stockage indisponible : préférence valable pour la session seulement.
	}
}

function pushNow(prefs: AppPreferences, keepalive = false): Promise<void> {
	if (!remote || !userScope) return Promise.resolve();
	const doc = { ...serverDoc, ...prefs } as Record<string, unknown>;
	const user = userScope;
	const target = remote;
	inflight = target
		.save(doc, { keepalive })
		.then(() => {
			if (userScope === user) serverDoc = doc;
		})
		.catch(() => {
			// Hors ligne / session expirée : le cache local garde la valeur.
		});
	return inflight;
}

/** Écrit tout de suite l'éventuelle écriture différée (fermeture de page, tests). */
export function flushPreferences(keepalive = false): Promise<void> {
	if (saveTimer) clearTimeout(saveTimer);
	saveTimer = null;
	const prefs = pending;
	pending = null;
	return prefs ? pushNow(prefs, keepalive) : inflight;
}

function schedulePush(prefs: AppPreferences): void {
	if (!remote || !userScope) return;
	pending = prefs;
	if (saveTimer) clearTimeout(saveTimer);
	saveTimer = setTimeout(() => void flushPreferences(), SAVE_DELAY_MS);
}

if (typeof window !== 'undefined')
	window.addEventListener('pagehide', () => void flushPreferences(true));

/** Lecture serveur du compte `user` ; migration unique si le serveur est vide. */
async function pullRemote(user: string, gen: number, localRaw: string | null): Promise<void> {
	if (!remote) return;
	let doc: Record<string, unknown>;
	try {
		doc = await remote.load();
	} catch {
		return; // Hors ligne : on reste sur le cache local.
	}
	if (gen !== generation || userScope !== user) return;
	serverDoc = doc;
	if (Object.keys(doc).length === 0) {
		// Serveur vierge : reprise unique des préférences locales de ce navigateur.
		let migrated = false;
		try {
			migrated = localStorage.getItem(`${MIGRATED_KEY}:${user}`) !== null;
		} catch {
			// Stockage indisponible.
		}
		if (migrated || localRaw === null) return;
		await pushNow(get(appPreferences));
		try {
			if (userScope === user) localStorage.setItem(`${MIGRATED_KEY}:${user}`, '1');
		} catch {
			// Stockage indisponible.
		}
		return;
	}
	if (dirty) return; // Modifié pendant la lecture : l'écriture différée fera foi.
	const prefs = parsePreferences(JSON.stringify(doc));
	appPreferences.set(prefs);
	writeCache(prefs);
}

/** Change d'utilisateur (session) : préférences du cache, puis celles du serveur. */
export function setPreferencesUser(user: string | null): AppPreferences {
	if (user === userScope) return get(appPreferences);
	// L'écriture différée du compte précédent ne part pas sous la nouvelle session.
	if (saveTimer) clearTimeout(saveTimer);
	saveTimer = null;
	pending = null;
	serverDoc = {};
	dirty = false;
	userScope = user;
	const gen = ++generation;
	const raw = readRaw(user);
	const prefs = parsePreferences(raw);
	appPreferences.set(prefs);
	if (user && remote) inflight = pullRemote(user, gen, raw);
	return prefs;
}

/** Attend la lecture / écriture serveur en cours (tests, chargement). */
export function preferencesSettled(): Promise<void> {
	return inflight;
}

export function loadPreferences(): AppPreferences {
	const prefs = parsePreferences(readRaw(userScope));
	appPreferences.set(prefs);
	return prefs;
}

export function savePreferences(prefs: AppPreferences): void {
	appPreferences.set(prefs);
	dirty = true;
	writeCache(prefs);
	schedulePush(prefs);
}

/** Lecture ponctuelle hors composant (handlers). */
export function currentPreferences(): AppPreferences {
	return get(appPreferences);
}

// — Choix mémorisés (Orca : « Clear my choice on… ») ————————————————————
/** Choix « Ne plus afficher » remis à zéro par « Effacer mes choix ». */
export const REMEMBERED_CHOICES: { key: string; label: string }[] = [
	{ key: 'web-slicer.recenter', label: 'Déplacement sans origine (Appareil)' }
];

/** Efface les choix mémorisés ; renvoie le nombre de choix effacés. */
export function clearRememberedChoices(): number {
	let n = 0;
	try {
		for (const { key } of REMEMBERED_CHOICES) {
			if (localStorage.getItem(key) !== null) n++;
			localStorage.removeItem(key);
		}
	} catch {
		// Stockage indisponible : rien à effacer.
	}
	return n;
}

/** Montant formaté avec la devise (« 0,08 € »). */
export function formatMoney(value: number, currency: string): string {
	const n = value.toLocaleString('fr-FR', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
	return currency ? `${n} ${currency}` : n;
}

/** Longueur (mm) formatée selon l'unité d'affichage. */
export function formatLength(mm: number, units: Units, digits = 2): string {
	return units === 'in' ? `${(mm / 25.4).toFixed(digits)} in` : `${mm.toFixed(digits)} mm`;
}
