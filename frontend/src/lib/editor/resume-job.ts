// Reprise à l'ouverture de l'éditeur (phase 32) : après un rechargement de la
// page pendant un tranchage, le job continue côté serveur mais l'éditeur
// l'ignorait (seul le dernier résultat réussi était restauré) — ni progression,
// ni aperçu à la fin. Pur : choisit quoi reprendre dans l'historique du compte.
import type { JobResponse, ServerEvent } from '#lib/api/types';

export type ResumePlan =
	{ kind: 'slicing'; jobs: JobResponse[] } | { kind: 'result'; job: JobResponse } | null;

const ACTIVE = new Set(['queued', 'running']);

/**
 * Jobs actifs du projet plus récents que son dernier résultat réussi →
 * reprise du suivi (tous les plateaux d'un même lancement) ; sinon le dernier
 * résultat réussi ; sinon rien.
 */
export function planResume(jobs: readonly JobResponse[], projectId: string): ResumePlan {
	const mine = jobs
		.filter((j) => j.project_id === projectId)
		.sort((a, b) => b.created_at.localeCompare(a.created_at));
	const latestDone = mine.find((j) => j.status === 'succeeded' && j.gcode_id);
	const active = mine.filter(
		(j) => ACTIVE.has(j.status) && (!latestDone || j.created_at > latestDone.created_at)
	);
	if (active.length) return { kind: 'slicing', jobs: active };
	return latestDone ? { kind: 'result', job: latestDone } : null;
}

/**
 * État courant d'un job (GET /jobs/{id}) traduit en événement serveur, pour
 * resynchroniser une session après une coupure du flux (événements perdus).
 * Statistiques absentes (`null`) : l'aperçu les relit comme à l'ouverture.
 */
export function jobAsEvent(job: JobResponse): ServerEvent {
	if (job.status === 'succeeded')
		return { event: 'job.finished', id: job.id, gcode_id: job.gcode_id, stats: null };
	return {
		event: 'job.updated',
		id: job.id,
		status: job.status,
		progress: job.progress,
		phase: job.phase,
		error: job.error
	};
}

/** Progression d'ensemble des jobs repris (moyenne bornée 0–1). */
export function resumedProgress(jobs: readonly JobResponse[]): number {
	if (!jobs.length) return 0;
	const sum = jobs.reduce((s, j) => s + Math.max(0, Math.min(1, j.progress || 0)), 0);
	return sum / jobs.length;
}
