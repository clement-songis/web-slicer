// Phase 41c — événements de job arrivés trop tôt. Le flux WebSocket du compte
// peut livrer `job.updated` / `job.finished` AVANT que la réponse de
// POST /slice n'ait créé la session de tranchage (job déjà fini quand la
// réponse arrive : machine ou réseau chargés). Ignorés, ils laissaient
// l'aperçu sur « Tranchage en cours… » indéfiniment. Ce tampon borné garde
// les événements des jobs non suivis pour les rejouer une fois la session créée.
import type { ServerEvent } from '../api/types';

type JobEvent = Extract<ServerEvent, { event: 'job.updated' | 'job.finished' }>;

function isJobEvent(event: ServerEvent): event is JobEvent {
	return event.event === 'job.updated' || event.event === 'job.finished';
}

export class EarlyJobEvents {
	private readonly byJob = new Map<string, JobEvent[]>();

	constructor(
		private readonly maxJobs = 32,
		private readonly maxPerJob = 8
	) {}

	/** Mémorise l'événement s'il concerne un job que la page ne suit pas (encore). */
	offer(event: ServerEvent, tracked: (jobId: string) => boolean): void {
		if (!isJobEvent(event) || tracked(event.id)) return;
		const list = this.byJob.get(event.id) ?? [];
		list.push(event);
		// Réinsertion : l'ordre de la Map reste « du moins récent au plus récent ».
		this.byJob.delete(event.id);
		this.byJob.set(event.id, list.slice(-this.maxPerJob));
		while (this.byJob.size > this.maxJobs) {
			const oldest = this.byJob.keys().next().value;
			if (oldest === undefined) break;
			this.byJob.delete(oldest);
		}
	}

	/** Retire et renvoie, dans l'ordre de réception, les événements de ces jobs. */
	take(jobIds: readonly string[]): ServerEvent[] {
		const out: ServerEvent[] = [];
		for (const id of jobIds) {
			out.push(...(this.byJob.get(id) ?? []));
			this.byJob.delete(id);
		}
		return out;
	}

	get size(): number {
		return this.byJob.size;
	}
}
