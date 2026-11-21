// Logique pure du suivi d'imprimantes (T077, US8) : réduction des événements
// `printer.status` en une carte d'état par imprimante, métadonnées d'affichage
// des états d'impression Klipper, disponibilité des contrôles et formatage.
// Aucun accès réseau ni DOM (testable en isolation).

import type { PrinterStatusResponse, ServerEvent } from '$lib/api/types';
import { tr } from '$lib/i18n';

/** État d'impression affiché pour une imprimante. */
export interface PrinterStatusView {
	state: string;
	filename?: string;
	/** Progression 0.0 → 1.0. */
	progress: number;
	extruderTemp: number;
	extruderTarget: number;
	bedTemp: number;
	bedTarget: number;
}

/** Carte d'état indexée par `printer_id`. */
export type StatusMap = Record<string, PrinterStatusView>;

/** Convertit la réponse REST `GET …/status` en vue d'affichage. */
export function fromStatusResponse(s: PrinterStatusResponse): PrinterStatusView {
	return {
		state: s.state,
		filename: s.filename,
		progress: s.progress,
		extruderTemp: s.extruder_temp,
		extruderTarget: s.extruder_target,
		bedTemp: s.bed_temp,
		bedTarget: s.bed_target
	};
}

/**
 * Applique un événement serveur à la carte d'état : seul `printer.status` a un
 * effet ; les autres événements laissent la carte inchangée. Renvoie une
 * nouvelle carte (immutable).
 */
export function applyPrinterStatus(map: StatusMap, event: ServerEvent): StatusMap {
	if (event.event !== 'printer.status') return map;
	return {
		...map,
		[event.printer_id]: {
			state: event.state,
			filename: event.filename,
			progress: event.progress,
			extruderTemp: event.extruder_temp,
			extruderTarget: event.extruder_target,
			bedTemp: event.bed_temp,
			bedTarget: event.bed_target
		}
	};
}

/** Métadonnées d'affichage d'un état d'impression (libellé + classe de badge). */
export interface StateMeta {
	label: string;
	badge: string;
}

// Libellés = clés anglaises, traduites par stateMeta (langue courante).
const STATE_META: Record<string, StateMeta> = {
	standby: { label: 'Standby', badge: 'bg-overlay text-content' },
	printing: { label: 'Printing', badge: 'bg-primary text-primary-content' },
	// Teintes douces du design system : texte ≥ 4,5:1 en clair comme en sombre
	// (le blanc sur ambre / vert / gris tombait sous 3:1).
	paused: { label: 'Paused', badge: 'bg-warning-soft text-warning-content' },
	complete: { label: 'Completed', badge: 'bg-success-soft text-success-content' },
	cancelled: { label: 'Cancelled', badge: 'bg-overlay text-content-muted' },
	error: { label: 'Error', badge: 'bg-danger-soft text-danger-content' }
};

/** Métadonnées d'un état (repli neutre si inconnu ou vide). */
export function stateMeta(state: string): StateMeta {
	const m = STATE_META[state];
	if (m) return { label: tr(m.label), badge: m.badge };
	return { label: state || tr('Unknown'), badge: 'bg-overlay text-content' };
}

/** Progression en pourcentage entier borné (0–100). */
export function progressPercent(progress: number): number {
	return Math.round(Math.max(0, Math.min(1, progress)) * 100);
}

/** Température courante (et cible si non nulle), arrondie au degré. */
export function formatTemp(current: number, target: number): string {
	const now = `${Math.round(current)} °C`;
	return target > 0 ? `${now} → ${Math.round(target)} °C` : now;
}

/** Pause disponible uniquement en cours d'impression. */
export function canPause(state: string): boolean {
	return state === 'printing';
}

/** Reprise disponible uniquement en pause. */
export function canResume(state: string): boolean {
	return state === 'paused';
}

/** Annulation disponible tant que l'impression n'est pas terminée. */
export function canCancel(state: string): boolean {
	return state === 'printing' || state === 'paused';
}
