// Appels de la file de tranchage (T071/US7), typés sur les DTO générés.
import { api } from './client';
import type { JobResponse } from './types';

/** File + historique du compte (plus récent d'abord). */
export const listJobs = () => api.get<JobResponse[]>('/jobs');

/** Détail d'un job. */
export const getJob = (id: string) => api.get<JobResponse>(`/jobs/${id}`);

/** Annule un job actif (idempotent si déjà annulé, 409 si terminé). */
export const cancelJob = (id: string) => api.post<JobResponse>(`/jobs/${id}/cancel`);
