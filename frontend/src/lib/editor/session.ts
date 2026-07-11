// Machine à états du tranchage dans le workspace (T088) : prepare → slicing →
// preview, avec branche error. Réducteur pur (aucun effet réseau/DOM) piloté par
// les événements WebSocket du job (T065) ; le composant `projects/[id]` ne fait
// qu'appliquer ces transitions et rendre l'état. Les erreurs sont **figées** :
// une fois en `error`, les événements suivants ne l'écrasent pas.
import type { ServerEvent, SliceWarning } from '../api/types';

/** Phase du cycle de tranchage. */
export type SlicePhase = 'prepare' | 'slicing' | 'preview' | 'error';

/** État du tranchage courant (immuable). */
export interface SliceSession {
	phase: SlicePhase;
	/** Jobs suivis (un par plateau tranché) — filtre les événements étrangers. */
	jobIds: string[];
	/** Progression agrégée 0 → 1. */
	progress: number;
	/** Libellé de phase moteur du job (`slice`, `export`…). */
	jobPhase: string;
	/** G-code prêt à prévisualiser (au premier job terminé). */
	gcodeId: string | null;
	/** Avertissements moteur figés au lancement (FR-032). */
	warnings: SliceWarning[];
	/** Message d'erreur figé, le cas échéant. */
	error: string | null;
}

/** État initial : préparation, rien en cours. */
export function prepareSession(): SliceSession {
	return {
		phase: 'prepare',
		jobIds: [],
		progress: 0,
		jobPhase: '',
		gcodeId: null,
		warnings: [],
		error: null
	};
}

/** Passe en tranchage pour un lot de jobs (avertissements moteur conservés). */
export function startSlicing(jobIds: string[], warnings: SliceWarning[] = []): SliceSession {
	return {
		...prepareSession(),
		phase: 'slicing',
		jobIds: [...jobIds],
		jobPhase: 'slice',
		warnings
	};
}

/** Échec du lancement (avant tout job) : erreur figée. */
export function sliceFailed(message: string): SliceSession {
	return { ...prepareSession(), phase: 'error', error: message };
}

/** Revient à la préparation (nouveau tranchage possible). */
export function resetSession(): SliceSession {
	return prepareSession();
}

/**
 * Applique un événement serveur. Ignore ceux qui ne concernent pas nos jobs
 * (isolation) ou hors sujet (`model.converted`, `printer.status`). Une erreur
 * déjà posée est figée : plus aucun événement ne la modifie.
 */
export function applyJobEvent(session: SliceSession, event: ServerEvent): SliceSession {
	if (session.phase === 'error') return session; // erreur figée

	if (event.event === 'job.updated') {
		if (!session.jobIds.includes(event.id)) return session;
		if (event.status === 'failed' || event.status === 'cancelled') {
			return { ...session, phase: 'error', error: errorMessage(event.error, event.status) };
		}
		return { ...session, progress: clamp01(event.progress), jobPhase: event.phase };
	}

	if (event.event === 'job.finished') {
		if (!session.jobIds.includes(event.id)) return session;
		if (!event.gcode_id) {
			return { ...session, phase: 'error', error: 'tranchage terminé sans G-code' };
		}
		return { ...session, phase: 'preview', progress: 1, gcodeId: event.gcode_id };
	}

	return session;
}

// --- Interne -----------------------------------------------------------------

function clamp01(v: number): number {
	if (!Number.isFinite(v)) return 0;
	return Math.min(1, Math.max(0, v));
}

function errorMessage(error: unknown, status: string): string {
	if (typeof error === 'string' && error.trim() !== '') return error;
	return status === 'cancelled' ? 'tranchage annulé' : 'échec du tranchage';
}
