// Logique pure de l'onglet Appareil (parité Device d'Orca pour Klipper /
// Moonraker) : temps restant estimé, scripts G-code des contrôles manuels,
// URL de webcam mémorisée, détection des routes backend absentes.
import { ApiError } from '$lib/api/client';

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
	if (bytes >= 1048576) return `${(bytes / 1048576).toFixed(1)} Mo`;
	return `${Math.max(1, Math.round(bytes / 1024))} ko`;
}
