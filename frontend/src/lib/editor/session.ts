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
	const raw =
		typeof error === 'string'
			? error
			: error &&
				  typeof error === 'object' &&
				  typeof (error as { message?: unknown }).message === 'string'
				? (error as { message: string }).message
				: '';
	if (raw.trim() !== '') return friendlySliceError(raw);
	return status === 'cancelled' ? 'tranchage annulé' : 'échec du tranchage';
}

/**
 * Message de tranchage lisible : retire l'enveloppe technique du worker
 * (« worker sorti en code 1 — stderr : … — stdout : E engine_crashed
 * configuration invalide : ») et traduit les refus de validation d'Orca
 * (`Print::validate`) les plus courants, notamment ceux de l'impression
 * objet par objet.
 */
export function friendlySliceError(raw: string): string {
	let msg = raw.trim();
	const invalid = /configuration invalide\s*:\s*([\s\S]*)$/i.exec(msg);
	if (invalid) msg = invalid[1].trim();
	else {
		// Journal du worker (horodatages, « P 0.25 … » de progression) : seul le
		// dernier code d'erreur « E <code> <message> » est retenu.
		const coded = [...msg.matchAll(/(?:^|\s)E\s+[a-z_]+\s+/g)].pop();
		if (coded && coded.index !== undefined) msg = msg.slice(coded.index + coded[0].length).trim();
		else {
			const out = /stdout\s*:\s*([\s\S]*)$/i.exec(msg);
			if (out && out[1].trim()) msg = out[1].trim();
		}
		// Enveloppe vide (« — stderr : » sans contenu).
		msg = msg.replace(/\s*—\s*(stderr|stdout)\s*:\s*$/i, '').trim();
	}
	// Orca concatène un message par objet (séparés par un saut de ligne, que la
	// sortie du worker peut réduire à une espace).
	const close = [
		...msg.matchAll(/(\S(?:.*?\S)?) is too close to others, and collisions may be caused\.?/gi)
	].map((m) => `« ${m[1].trim()} »`);
	if (close.length)
		return `${close.join(', ')} ${close.length > 1 ? 'sont trop près' : 'est trop près'} des autres objets : en impression objet par objet, la tête heurterait les objets déjà imprimés. Écartez-les (« Disposer » les espace automatiquement) ou repassez en impression couche par couche.`;
	const tall = [
		...msg.matchAll(/(\S(?:.*?\S)?) is too tall, and collisions will be caused\.?/gi)
	].map((m) => `« ${m[1].trim()} »`);
	if (tall.length)
		return `${tall.join(', ')} : trop haut pour l'impression objet par objet (dégagement de la tête dépassé) ; imprimez-le en dernier ou repassez en impression couche par couche.`;
	return msg;
}
