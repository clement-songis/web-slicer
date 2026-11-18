// Résultats de tranchage **par plateau** et « Trancher tout » (phase 25).
//
// Orca garde un résultat par plateau (`PartPlate` : G-code + GCodeProcessorResult) :
// « Slice all » tranche chaque plateau (les vides sont sautés), l'aperçu montre le
// plateau courant et changer d'onglet de plateau affiche le résultat de ce plateau
// (ou « non tranché »). Ce module est pur : il réduit les événements des jobs d'un
// lot (un job par plateau) en état de session + table plateau → résultat ; la page
// `projects/[id]` applique le résultat (chargement de l'aperçu, onglet actif).
import type { ServerEvent } from '../api/types';
import { friendlySliceError, type SliceSession } from './session';

/** Résultat mémorisé d'un plateau tranché. */
export interface PlateResult {
	gcodeId: string;
	jobId: string;
	/** Statistiques du job (`job.finished.stats`) pour le « Résultat du tranchage ». */
	stats: unknown;
	/** Signature de la scène et des réglages tranchés : résultat périmé si elle change. */
	signature: string | null;
}

/** Table plateau → dernier résultat réussi. */
export type PlateResults = Record<string, PlateResult>;

/** Lot de tranchage en cours (un job par plateau). */
export interface SliceBatch {
	/** job → plateau. */
	jobPlates: Record<string, string>;
	/** Ordre des plateaux du lot (ordre des onglets). */
	plateOrder: string[];
	/** Progression 0 → 1 par job. */
	progress: Record<string, number>;
	/** Jobs terminés (réussis ou non). */
	done: Record<string, 'ok' | 'failed'>;
	/** Erreur par plateau. */
	errors: Record<string, string>;
	signature: string | null;
}

/**
 * Lot à partir des jobs créés par `POST /slice` (`plate_index` = rang de
 * l'onglet) et des identifiants des plateaux dans l'ordre des onglets.
 */
export function makeBatch(
	jobs: readonly { id: string; plate_index: bigint | number }[],
	plateIds: readonly string[],
	signature: string | null
): SliceBatch {
	const jobPlates: Record<string, string> = {};
	for (const job of jobs) {
		const id = plateIds[Number(job.plate_index)];
		if (id) jobPlates[job.id] = id;
	}
	const plateOrder = plateIds.filter((id) => Object.values(jobPlates).includes(id));
	return { jobPlates, plateOrder, progress: {}, done: {}, errors: {}, signature };
}

/** Résultat valable d'un plateau (absent ou périmé → null). */
export function resultFor(
	results: PlateResults,
	plateId: string | null,
	signature: string | null
): PlateResult | null {
	if (!plateId) return null;
	const r = results[plateId];
	if (!r) return null;
	if (signature !== null && r.signature !== null && r.signature !== signature) return null;
	return r;
}

/** Nombre de jobs terminés / total. */
export function batchCounts(batch: SliceBatch): { done: number; total: number } {
	return { done: Object.keys(batch.done).length, total: Object.keys(batch.jobPlates).length };
}

/** Le lot est-il terminé ? */
export function batchFinished(batch: SliceBatch): boolean {
	const { done, total } = batchCounts(batch);
	return total > 0 && done >= total;
}

export interface BatchState {
	session: SliceSession;
	batch: SliceBatch;
	results: PlateResults;
}

export interface BatchStep extends BatchState {
	/** Plateau à afficher (onglet à activer), le cas échéant. */
	showPlate: string | null;
}

/**
 * Applique un événement de job au lot. Plateau « cible » = l'onglet actif s'il
 * fait partie du lot, sinon le premier plateau tranché (l'onglet bascule dessus,
 * comme Orca qui montre le plateau courant). L'aperçu s'ouvre dès que la cible
 * est prête ; les autres plateaux continuent et sont mémorisés. Une erreur sur
 * un autre plateau n'interrompt pas le lot (Orca passe au plateau suivant).
 */
export function applyBatchEvent(
	state: BatchState,
	activePlateId: string | null,
	event: ServerEvent
): BatchStep {
	const none = { ...state, showPlate: null };
	if (event.event !== 'job.updated' && event.event !== 'job.finished') return none;
	const plate = state.batch.jobPlates[event.id];
	if (!plate) return none;
	if (state.batch.done[event.id]) return none;
	let { session, batch, results } = state;

	if (event.event === 'job.updated' && (event.status === 'failed' || event.status === 'cancelled')) {
		const message = errorText(event.error, event.status);
		batch = {
			...batch,
			done: { ...batch.done, [event.id]: 'failed' },
			errors: { ...batch.errors, [plate]: message }
		};
	} else if (event.event === 'job.updated') {
		batch = { ...batch, progress: { ...batch.progress, [event.id]: clamp01(event.progress) } };
		if (session.phase === 'slicing') session = { ...session, jobPhase: event.phase };
	} else if (!event.gcode_id) {
		batch = {
			...batch,
			done: { ...batch.done, [event.id]: 'failed' },
			errors: { ...batch.errors, [plate]: 'tranchage terminé sans G-code' }
		};
	} else {
		batch = {
			...batch,
			done: { ...batch.done, [event.id]: 'ok' },
			progress: { ...batch.progress, [event.id]: 1 }
		};
		results = {
			...results,
			[plate]: {
				gcodeId: event.gcode_id,
				jobId: event.id,
				stats: event.stats,
				signature: batch.signature
			}
		};
	}

	const total = Object.keys(batch.jobPlates).length;
	const progress =
		total === 0
			? 0
			: Object.keys(batch.jobPlates).reduce(
					(sum, job) => sum + (batch.done[job] ? 1 : (batch.progress[job] ?? 0)),
					0
				) / total;

	// Session déjà sur l'aperçu de la cible (ou en erreur figée) : seuls la
	// table et le lot avancent.
	if (session.phase !== 'slicing') return { session, batch, results, showPlate: null };

	const inBatch = activePlateId !== null && batch.plateOrder.includes(activePlateId);
	const fromBatch = (id: string) => batch.jobPlates[results[id]?.jobId ?? ''] === id;
	const target = inBatch ? activePlateId : (batch.plateOrder.find(fromBatch) ?? null);
	const ready = target ? results[target] : undefined;
	if (target && ready && batch.jobPlates[ready.jobId] === target) {
		return {
			session: { ...session, phase: 'preview', progress: 1, gcodeId: ready.gcodeId },
			batch,
			results,
			showPlate: target !== activePlateId ? target : null
		};
	}
	const targetError = target ? batch.errors[target] : undefined;
	if (inBatch && targetError) {
		return {
			session: { ...session, phase: 'error', error: targetError },
			batch,
			results,
			showPlate: null
		};
	}
	if (batchFinished(batch)) {
		// Rien d'affichable : tous les plateaux du lot ont échoué.
		const first = batch.plateOrder.map((id) => batch.errors[id]).find(Boolean);
		return {
			session: { ...session, phase: 'error', error: first ?? 'échec du tranchage' },
			batch,
			results,
			showPlate: null
		};
	}
	return { session: { ...session, progress }, batch, results, showPlate: null };
}

/** Texte de progression d'un lot de plusieurs plateaux (« 2 / 3 plateaux tranchés »). */
export function batchLabel(batch: SliceBatch): string | null {
	const { done, total } = batchCounts(batch);
	if (total < 2) return null;
	const failed = Object.values(batch.done).filter((d) => d === 'failed').length;
	return (
		`${done - failed} / ${total} plateaux tranchés` + (failed ? ` · ${failed} en échec` : '')
	);
}

function clamp01(v: number): number {
	if (!Number.isFinite(v)) return 0;
	return Math.min(1, Math.max(0, v));
}

function errorText(error: unknown, status: string): string {
	const raw =
		typeof error === 'string'
			? error
			: error && typeof error === 'object' && typeof (error as { message?: unknown }).message === 'string'
				? (error as { message: string }).message
				: '';
	if (raw.trim() !== '') return friendlySliceError(raw);
	return status === 'cancelled' ? 'tranchage annulé' : 'échec du tranchage';
}
