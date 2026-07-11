// Logique pure de la file de tranchage (T071/US7). Réducteur d'événements
// WebSocket sur la liste de jobs, partition file active / historique, et
// métadonnées d'affichage par état. Aucun accès réseau ni DOM : `+page.svelte`
// s'y branche.

import type { JobResponse, ServerEvent } from '$lib/api/types';

/** États d'un job (miroir du backend `JobStatus`). */
export type JobStatus = 'queued' | 'running' | 'succeeded' | 'failed' | 'cancelled';

/** Un job est « actif » tant qu'il n'a pas atteint un état terminal. */
export function isActive(job: JobResponse): boolean {
	return job.status === 'queued' || job.status === 'running';
}

/** Sépare la file active de l'historique (terminés), chacun trié récent d'abord. */
export function partitionJobs(jobs: JobResponse[]): {
	active: JobResponse[];
	history: JobResponse[];
} {
	const byRecent = (a: JobResponse, b: JobResponse) => b.created_at.localeCompare(a.created_at);
	return {
		active: jobs.filter(isActive).sort(byRecent),
		history: jobs.filter((j) => !isActive(j)).sort(byRecent)
	};
}

/**
 * Applique un événement serveur (`job.updated` / `job.finished`) à la liste. Un
 * job inconnu est ignoré (la liste est semée par le `load`, l'événement ne crée
 * pas de job fantôme). Renvoie une nouvelle liste (immutable).
 */
export function applyEvent(jobs: JobResponse[], event: ServerEvent): JobResponse[] {
	switch (event.event) {
		case 'job.updated':
			return jobs.map((j) =>
				j.id === event.id
					? {
							...j,
							status: event.status,
							progress: event.progress,
							phase: event.phase,
							error: event.error ?? j.error
						}
					: j
			);
		case 'job.finished':
			return jobs.map((j) =>
				j.id === event.id
					? { ...j, status: 'succeeded', progress: 1, gcode_id: event.gcode_id ?? j.gcode_id }
					: j
			);
		case 'model.converted':
			return jobs; // sans effet sur la file
	}
}

/** Métadonnées d'affichage d'un état (libellé + classe de badge). */
export interface StatusMeta {
	label: string;
	/** Classe Tailwind de la pastille d'état. */
	badge: string;
}

const STATUS_META: Record<JobStatus, StatusMeta> = {
	queued: { label: 'En file', badge: 'bg-slate-600 text-slate-100' },
	running: { label: 'En cours', badge: 'bg-sky-600 text-white' },
	succeeded: { label: 'Terminé', badge: 'bg-green-600 text-white' },
	failed: { label: 'Échec', badge: 'bg-red-600 text-white' },
	cancelled: { label: 'Annulé', badge: 'bg-amber-600 text-white' }
};

/** Métadonnées d'affichage d'un état (repli neutre si inconnu). */
export function statusMeta(status: string): StatusMeta {
	return (
		STATUS_META[status as JobStatus] ?? { label: status, badge: 'bg-slate-600 text-slate-100' }
	);
}

/** Progression en pourcentage entier borné (0–100). */
export function progressPercent(job: JobResponse): number {
	return Math.round(Math.max(0, Math.min(1, job.progress)) * 100);
}
