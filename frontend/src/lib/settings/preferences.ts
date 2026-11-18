// Préférences applicatives (menu « Preferences ») : sous-ensemble de
// PreferencesDialog.cpp d'Orca qui a un sens dans un navigateur — général,
// contrôle (comportement, tranchage, caméra/souris), presets, choix mémorisés.
// Persistées dans localStorage **par utilisateur** (clé suffixée par l'id de
// session, repli sur la clé globale historique) ; le thème et la langue
// délèguent à leurs modules (`$lib/theme`, `$lib/i18n`).
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
	mixedTemperature: false
};

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
			mixedTemperature: bool(value.mixedTemperature, d.mixedTemperature)
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

/** Change d'utilisateur (session) : recharge ses préférences. */
export function setPreferencesUser(user: string | null): AppPreferences {
	if (user === userScope) return get(appPreferences);
	userScope = user;
	const prefs = parsePreferences(readRaw(user));
	appPreferences.set(prefs);
	return prefs;
}

export function loadPreferences(): AppPreferences {
	const prefs = parsePreferences(readRaw(userScope));
	appPreferences.set(prefs);
	return prefs;
}

export function savePreferences(prefs: AppPreferences): void {
	appPreferences.set(prefs);
	try {
		localStorage.setItem(preferencesKey(), JSON.stringify(prefs));
		// Copie sous la clé historique (dernier compte actif) : compatibilité avec
		// les lecteurs existants ; la clé par compte reste la référence.
		if (userScope) localStorage.setItem(PREFERENCES_KEY, JSON.stringify(prefs));
	} catch {
		// Stockage indisponible : préférence valable pour la session seulement.
	}
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
