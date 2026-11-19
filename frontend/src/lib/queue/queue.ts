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
		case 'model.conversion_failed':
		case 'printer.status':
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
	queued: { label: 'En file', badge: 'bg-overlay text-content' },
	running: { label: 'En cours', badge: 'bg-primary text-primary-content' },
	succeeded: { label: 'Terminé', badge: 'bg-success text-white' },
	failed: { label: 'Échec', badge: 'bg-danger text-white' },
	cancelled: { label: 'Annulé', badge: 'bg-warning text-white' }
};

/** Métadonnées d'affichage d'un état (repli neutre si inconnu). */
export function statusMeta(status: string): StatusMeta {
	return STATUS_META[status as JobStatus] ?? { label: status, badge: 'bg-overlay text-content' };
}

/**
 * Rang de chaque job **en file** (1 = prochain à démarrer), dans l'ordre
 * d'arrivée (phase 32 : plusieurs jobs simultanés, « En file » seul ne disait
 * pas lequel passerait d'abord).
 */
export function queuePositions(jobs: readonly JobResponse[]): Record<string, number> {
	const queued = jobs
		.filter((j) => j.status === 'queued')
		.sort((a, b) => a.created_at.localeCompare(b.created_at));
	return Object.fromEntries(queued.map((j, i) => [j.id, i + 1]));
}

/** Ajoute ou remplace un job (par id) — job apparu après le chargement de la
 *  page (lancé depuis un autre onglet), connu par un événement serveur. */
export function upsertJob(jobs: readonly JobResponse[], job: JobResponse): JobResponse[] {
	return jobs.some((j) => j.id === job.id)
		? jobs.map((j) => (j.id === job.id ? job : j))
		: [job, ...jobs];
}

/** Identifiant de job inconnu de la liste, porté par un événement de job. */
export function unknownJobId(jobs: readonly JobResponse[], event: ServerEvent): string | null {
	if (event.event !== 'job.updated' && event.event !== 'job.finished') return null;
	return jobs.some((j) => j.id === event.id) ? null : event.id;
}

/** Libellé français de la phase moteur d'un job (`slice`, `export`…). */
export function phaseLabel(phase: string | null | undefined): string {
	const p = (phase ?? '').trim();
	if (!p) return '';
	const known: Record<string, string> = {
		queued: 'en attente du moteur',
		slice: 'tranchage',
		slicing: 'tranchage',
		gcode: 'génération du G-code',
		export: 'écriture du G-code',
		post: 'post-traitement',
		postprocess: 'post-traitement',
		thumbnail: 'vignette',
		done: 'terminé'
	};
	return known[p.toLowerCase()] ?? p;
}

/** Texte d'échec d'un job (`error` : chaîne ou `{ message }`). */
export function jobErrorText(error: unknown): string {
	if (!error) return '';
	if (typeof error === 'string') return error;
	if (typeof error === 'object' && error && 'message' in error)
		return String((error as { message: unknown }).message ?? '');
	return '';
}

/** Progression en pourcentage entier borné (0–100). */
export function progressPercent(job: JobResponse): number {
	return Math.round(Math.max(0, Math.min(1, job.progress)) * 100);
}
