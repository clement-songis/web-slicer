// Logique pure de l'onglet Appareil (parité Device d'Orca pour Klipper /
// Moonraker) : temps restant estimé, scripts G-code des contrôles manuels,
// URL de webcam mémorisée, détection des routes backend absentes.
import { ApiError } from '$lib/api/client';
import { tr } from '$lib/i18n';

/** Champs d'état facultatifs qu'un backend enrichi peut renvoyer
 *  (`print_stats.info`, `fan`, `toolhead` de Moonraker). */
export interface ExtendedStatus {
	print_duration?: number;
	current_layer?: number | null;
	total_layer?: number | null;
	fan_speed?: number | null;
	homed_axes?: string | null;
	webcam_url?: string | null;
}

/** Temps restant estimé (s) par extrapolation linéaire de la progression,
 *  comme l'estimation « restant » de Mainsail/Fluidd ; null sans données. */
export function remainingSeconds(progress: number, elapsed: number): number | null {
	if (!(progress > 0.01) || !(elapsed > 0) || progress >= 1) return null;
	return Math.round((elapsed * (1 - progress)) / progress);
}

/** Durée lisible : « 1 h 05 min », « 4 min 12 s ». */
export function formatClock(seconds: number | null | undefined): string {
	if (seconds == null || !Number.isFinite(seconds)) return '—';
	const s = Math.max(0, Math.round(seconds));
	const h = Math.floor(s / 3600);
	const m = Math.floor((s % 3600) / 60);
	if (h > 0) return `${h} h ${String(m).padStart(2, '0')} min`;
	return `${m} min ${String(s % 60).padStart(2, '0')} s`;
}

export type Axis = 'X' | 'Y' | 'Z';
export type Heater = 'extruder' | 'bed';

/** Déplacement relatif d'un axe (G91 … G90), vitesse en mm/min. */
export function moveGcode(axis: Axis, delta: number, feed = axis === 'Z' ? 600 : 6000): string {
	return `G91\nG1 ${axis}${delta} F${feed}\nG90`;
}

/** Mise à l'origine : tous les axes ou un seul. */
export function homeGcode(axis?: Axis): string {
	return axis ? `G28 ${axis}` : 'G28';
}

/** Consigne de température (°C, bornée à [0, 350] buse / [0, 150] plateau). */
export function temperatureGcode(heater: Heater, target: number): string {
	const max = heater === 'extruder' ? 350 : 150;
	const t = Math.round(Math.max(0, Math.min(max, target)));
	return heater === 'extruder' ? `M104 S${t}` : `M140 S${t}`;
}

/** Ventilateur de pièce en % (0 → M107). */
export function fanGcode(percent: number): string {
	const p = Math.max(0, Math.min(100, percent));
	return p === 0 ? 'M107' : `M106 S${Math.round((p / 100) * 255)}`;
}

/** Route backend absente (contrôle non encore relayé). */
export function isMissingRoute(e: unknown): boolean {
	return e instanceof ApiError && (e.status === 404 || e.status === 405 || e.status === 501);
}

const WEBCAM_KEY = 'web-slicer.webcam';

/** URL de webcam saisie localement pour une imprimante (navigateur) : surcharge
 *  le `webcam_url` publié par le statut (relais `/server/webcams/list`). */
export function loadWebcamUrl(printerId: string): string {
	try {
		return (
			(JSON.parse(localStorage.getItem(WEBCAM_KEY) ?? '{}') as Record<string, string>)[printerId] ??
			''
		);
	} catch {
		return '';
	}
}

export function saveWebcamUrl(printerId: string, url: string): void {
	try {
		const all = JSON.parse(localStorage.getItem(WEBCAM_KEY) ?? '{}') as Record<string, string>;
		if (url.trim()) all[printerId] = url.trim();
		else delete all[printerId];
		localStorage.setItem(WEBCAM_KEY, JSON.stringify(all));
	} catch {
		// Stockage indisponible : URL valable pour la session seulement.
	}
}

/** Taille de fichier lisible. */
export function formatSize(bytes: number): string {
	if (bytes >= 1048576) return tr('{n} MB', { n: (bytes / 1048576).toFixed(1) });
	return tr('{n} kB', { n: Math.max(1, Math.round(bytes / 1024)) });
}

/**
 * Axes sans origine (Orca RecenterDialog : déplacement refusé tant que la tête
 * n'a pas fait son origine). `homed` = `homed_axes` Klipper (« xyz », « xy »,
 * « »). Inconnu (statut sans ce champ) → aucun avertissement.
 */
export function unhomedAxes(
	homed: string | null | undefined,
	axes: readonly Axis[] = ['X', 'Y', 'Z']
): Axis[] {
	if (homed === null || homed === undefined) return [];
	const h = homed.toLowerCase();
	return axes.filter((a) => !h.includes(a.toLowerCase()));
}

export const RECENTER_KEY = 'web-slicer.recenter';
/** « Ne plus avertir » mémorisé (effacé par Préférences › Effacer mes choix). */
export function recenterMuted(): boolean {
	try {
		return localStorage.getItem(RECENTER_KEY) === 'muted';
	} catch {
		return false;
	}
}
export function muteRecenter(): void {
	try {
		localStorage.setItem(RECENTER_KEY, 'muted');
	} catch {
		/* stockage indisponible */
	}
}

// — Exclusion d'objet en cours d'impression (Orca PartSkipDialog ; Klipper
// exclude_object : noms EXCLUDE_OBJECT_DEFINE d'Orca « <objet>_id_<n>_copy_<i> »).
export type PrintObjectState = 'excluded' | 'current' | 'pending';

/** Nom lisible : « cube.stl_id_0_copy_1 » → « cube.stl · copie 2 ». */
export function printObjectLabel(name: string): string {
	const m = /^(.*)_id_(\d+)_copy_(\d+)$/i.exec(name);
	if (!m) return name;
	const copy = Number(m[3]);
	return copy > 0 ? tr('{name} · copy {n}', { name: m[1], n: copy + 1 }) : m[1];
}

export function printObjectState(
	name: string,
	s: { excluded_objects: readonly string[]; current_object?: string | null }
): PrintObjectState {
	const n = name.toLowerCase();
	if (s.excluded_objects.some((e) => e.toLowerCase() === n)) return 'excluded';
	if (s.current_object && s.current_object.toLowerCase() === n) return 'current';
	return 'pending';
}

/** Cadre SVG englobant les contours (marge 5 %), Y inversé géré par l'appelant. */
export function objectsBounds(
	objects: readonly {
		polygon: readonly (readonly [number, number])[];
		center?: readonly [number, number] | null;
	}[]
): { minX: number; minY: number; width: number; height: number } | null {
	const pts = objects.flatMap((o) => (o.polygon.length ? o.polygon : o.center ? [o.center] : []));
	if (!pts.length) return null;
	const xs = pts.map((p) => p[0]);
	const ys = pts.map((p) => p[1]);
	const [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
	const m = Math.max(x1 - x0, y1 - y0, 10) * 0.05;
	return { minX: x0 - m, minY: y0 - m, width: x1 - x0 + 2 * m, height: y1 - y0 + 2 * m };
}
