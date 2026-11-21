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
import { numberLocale } from '$lib/i18n';
import type { UserGrouping } from '$lib/presets/user-grouping';

export type Units = 'mm' | 'in';
/** Orca `camera_navigation_style` : 0 = Default, 1 = Touchpad. */
export type CameraStyle = 'default' | 'touchpad';
/** Orca `*_mouse_drag_action` : 0 = None, 1 = Pan, 2 = Rotate. */
export type DragAction = 'none' | 'pan' | 'rotate';
/** Orca `auto_calculate_flush` : « all », « color change », « disabled ». */
export type AutoFlush = 'all' | 'color' | 'disabled';
/** Orca `default_page` : 0 = Accueil, 1 = Préparer. */
export type DefaultPage = 'home' | 'prepare';
/**
 * Orca `project_load_behaviour` (Preferences « Load behaviour ») :
 * `load_all` (défaut), `ask_when_relevant`, `always_ask`, `load_geometry_only`.
 */
export type ProjectLoadBehaviour =
	'load_all' | 'ask_when_relevant' | 'always_ask' | 'load_geometry_only';

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

/**
 * Orca Preferences › Graphismes, version navigateur (phase 34) : MSAA du
 * contexte WebGL (anticrénelage), plafond d'images/s (Orca `max_fps`, 0 =
 * illimité) et compteur d'images/s ; vue réaliste (phase 38, Orca « Realistic
 * View » : opengl_realistic_mode / _phong / phong_basic_plate_shadows /
 * phong_smooth_normals, mêmes défauts) ; SSAO (phase 39, opengl_phong_ssao).
 */
export interface GraphicsPreferences {
	antialias: boolean;
	/** 0 = illimité ; sinon 10–240. */
	fpsCap: number;
	showFps: boolean;
	/** Orca opengl_realistic_mode (défaut faux) : vue réaliste (aussi au menu Vue). */
	realisticView: boolean;
	/** Orca opengl_realistic_phong (défaut vrai) : Phong, sinon Gouraud. */
	realisticPhong: boolean;
	/** Orca opengl_phong_ssao (défaut faux) : occlusion ambiante SSAO (phase 39). */
	realisticSsao: boolean;
	/** Orca opengl_phong_basic_plate_shadows (défaut faux) : ombres sur le plateau. */
	realisticShadows: boolean;
	/** Orca opengl_phong_smooth_normals (défaut faux) : normales lissées sous 5°. */
	realisticSmoothNormals: boolean;
}

export const GRAPHICS_DEFAULTS: GraphicsPreferences = {
	antialias: true,
	fpsCap: 0,
	showFps: false,
	realisticView: false,
	realisticPhong: true,
	realisticSsao: false,
	realisticShadows: false,
	realisticSmoothNormals: false
};

/**
 * Options d'arrangement mémorisées (Orca : AppConfig section « arrange »,
 * clés min_object_distance_fff / enable_rotation_fff… ; lot 34). La zone de
 * calibration n'est proposée que sur certaines imprimantes (GET …/arrange/options).
 */
export interface ArrangePreferences {
	/** Espacement (mm), 0 = automatique (Orca ArrangeSettings::distance). */
	spacing: number;
	/** « Rotation automatique pour l'arrangement ». */
	enableRotation: boolean;
	/** « Plusieurs matériaux sur le même plateau ». */
	allowMultiMaterialsOnSamePlate: boolean;
	/** « Éviter la zone de calibration d'extrusion ». */
	avoidExtrusionCaliRegion: boolean;
	/** « Aligner sur l'axe Y » (ignoré si rotation automatique). */
	alignToYAxis: boolean;
}

export const ARRANGE_DEFAULTS: ArrangePreferences = {
	spacing: 0,
	enableRotation: false,
	allowMultiMaterialsOnSamePlate: true,
	avoidExtrusionCaliRegion: true,
	alignToYAxis: false
};

/** Plafond d'images/s borné (0 = illimité, sinon 10–240, entier). */
export function clampFpsCap(value: number): number {
	if (!Number.isFinite(value) || value <= 0) return 0;
	return Math.round(Math.min(240, Math.max(10, value)));
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
	/** Filaments cochés à l'assistant (Orca : section « filaments » d'AppConfig,
	 *  noms de presets). `null` : assistant jamais passé (phase 25). */
	wizardFilaments: string[] | null;
	/** Orca `group_filament_presets` (défaut « None ») : regroupement des presets
	 *  filament utilisateur dans le sélecteur. */
	groupFilamentPresets: UserGrouping;
	graphics: GraphicsPreferences;
	/**
	 * Orca `project_load_behaviour` : ouverture d'un 3MF (boîte « Ouvrir comme
	 * projet / Importer la géométrie seule », lot 34). Le serveur lit la même
	 * clé (camelCase accepté) quand la requête d'import ne porte pas `load`.
	 */
	projectLoadBehaviour: ProjectLoadBehaviour;
	/**
	 * Case « Ne plus afficher » de l'avertissement « G-code modifié » (Orca
	 * `no_warn_when_modified_gcodes`) : le serveur ne renvoie plus l'avertissement.
	 */
	noWarnWhenModifiedGcodes: boolean;
	/** Options d'arrangement mémorisées (menu « Options d'arrangement »). */
	arrange: ArrangePreferences;
	/** Orca `show_3d_navigator` (défaut vrai) : cube de navigation de la vue 3D. */
	show3dNavigator: boolean;
	/** Orca `show_axes` (défaut vrai) : axes du repère d'origine du plateau. */
	showAxes: boolean;
	/**
	 * Orca `drc_bits` (« Niveau de qualité pour l'exportation Draco ») : bits de
	 * quantification de l'export DRC ; 0 = sans perte, sinon 8 à 30.
	 */
	dracoBits: number;
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
	maxRecent: 18,
	wizardFilaments: null,
	groupFilamentPresets: 'none',
	graphics: { ...GRAPHICS_DEFAULTS },
	projectLoadBehaviour: 'load_all',
	noWarnWhenModifiedGcodes: false,
	arrange: { ...ARRANGE_DEFAULTS },
	show3dNavigator: true,
	showAxes: true,
	dracoBits: 0
};

/** DRC.hpp d'Orca : DRC_BITS_MIN 8, DRC_BITS_MAX 30, défaut 0 (sans perte). */
export const DRACO_BITS_MIN = 8;
export const DRACO_BITS_MAX = 30;

/**
 * Bornes de PreferencesDialog::create_item_draco : > 30 → 30 ; 1 à 7 → 8 ;
 * 0 (ou négatif / invalide) → 0, sans perte.
 */
export function clampDracoBits(value: number): number {
	if (!Number.isFinite(value) || value <= 0) return 0;
	const n = Math.round(value);
	if (n > DRACO_BITS_MAX) return DRACO_BITS_MAX;
	return n < DRACO_BITS_MIN ? DRACO_BITS_MIN : n;
}

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

function parseGraphics(raw: unknown): GraphicsPreferences {
	const g = (raw && typeof raw === 'object' ? raw : {}) as Partial<GraphicsPreferences>;
	return {
		antialias: bool(g.antialias, GRAPHICS_DEFAULTS.antialias),
		fpsCap: typeof g.fpsCap === 'number' ? clampFpsCap(g.fpsCap) : GRAPHICS_DEFAULTS.fpsCap,
		showFps: bool(g.showFps, GRAPHICS_DEFAULTS.showFps),
		realisticView: bool(g.realisticView, GRAPHICS_DEFAULTS.realisticView),
		realisticPhong: bool(g.realisticPhong, GRAPHICS_DEFAULTS.realisticPhong),
		realisticSsao: bool(g.realisticSsao, GRAPHICS_DEFAULTS.realisticSsao),
		realisticShadows: bool(g.realisticShadows, GRAPHICS_DEFAULTS.realisticShadows),
		realisticSmoothNormals: bool(g.realisticSmoothNormals, GRAPHICS_DEFAULTS.realisticSmoothNormals)
	};
}

/** Espacement d'arrangement borné (0 = automatique, sinon 0–100 mm ; slider d'Orca). */
export function clampArrangeSpacing(value: number): number {
	if (!Number.isFinite(value) || value <= 0) return 0;
	return Math.round(Math.min(100, Math.max(0, value)) * 100) / 100;
}

function parseArrange(raw: unknown): ArrangePreferences {
	const a = (raw && typeof raw === 'object' ? raw : {}) as Partial<ArrangePreferences>;
	return {
		spacing:
			typeof a.spacing === 'number' ? clampArrangeSpacing(a.spacing) : ARRANGE_DEFAULTS.spacing,
		enableRotation: bool(a.enableRotation, ARRANGE_DEFAULTS.enableRotation),
		allowMultiMaterialsOnSamePlate: bool(
			a.allowMultiMaterialsOnSamePlate,
			ARRANGE_DEFAULTS.allowMultiMaterialsOnSamePlate
		),
		avoidExtrusionCaliRegion: bool(
			a.avoidExtrusionCaliRegion,
			ARRANGE_DEFAULTS.avoidExtrusionCaliRegion
		),
		alignToYAxis: bool(a.alignToYAxis, ARRANGE_DEFAULTS.alignToYAxis)
	};
}

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
				typeof value.maxRecent === 'number' ? clampRecentCount(value.maxRecent) : d.maxRecent,
			wizardFilaments: Array.isArray(value.wizardFilaments)
				? value.wizardFilaments.filter((n): n is string => typeof n === 'string').slice(0, 500)
				: d.wizardFilaments,
			groupFilamentPresets: oneOf(
				value.groupFilamentPresets,
				['all', 'none', 'type', 'vendor'] as const,
				d.groupFilamentPresets
			),
			graphics: parseGraphics(value.graphics),
			projectLoadBehaviour: oneOf(
				value.projectLoadBehaviour,
				['load_all', 'ask_when_relevant', 'always_ask', 'load_geometry_only'] as const,
				d.projectLoadBehaviour
			),
			noWarnWhenModifiedGcodes: bool(value.noWarnWhenModifiedGcodes, d.noWarnWhenModifiedGcodes),
			arrange: parseArrange(value.arrange),
			show3dNavigator: bool(value.show3dNavigator, d.show3dNavigator),
			showAxes: bool(value.showAxes, d.showAxes),
			dracoBits: typeof value.dracoBits === 'number' ? clampDracoBits(value.dracoBits) : d.dracoBits
		};
	} catch {
		return {
			...DEFAULT_PREFERENCES,
			camera: { ...CAMERA_DEFAULTS },
			graphics: { ...GRAPHICS_DEFAULTS }
		};
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
	{ key: 'web-slicer.recenter', label: 'Move without origin (Device)' }
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

/** Montant formaté avec la devise (« 0,08 € » en français, « 0.08 € » en anglais — audit 40 A4). */
export function formatMoney(value: number, currency: string, loc = numberLocale()): string {
	const n = value.toLocaleString(loc, { minimumFractionDigits: 2, maximumFractionDigits: 2 });
	return currency ? `${n} ${currency}` : n;
}

/** Longueur (mm) formatée selon l'unité d'affichage. */
export function formatLength(mm: number, units: Units, digits = 2): string {
	return units === 'in' ? `${(mm / 25.4).toFixed(digits)} in` : `${mm.toFixed(digits)} mm`;
}
