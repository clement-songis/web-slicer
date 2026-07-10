// Appels des outils moteur de scène (endpoints T054) : arrangement, orientation
// et réparation (rapport). Typés sur les DTO générés.
import { api } from './client';
import type { ArrangeRequest, ArrangeResponse, OrientResponse, RepairResponse } from './types';

/** Arrangement sans collision des empreintes fournies (FR-013). */
export const arrangeScene = (projectId: string, body: ArrangeRequest) =>
	api.post<ArrangeResponse>(`/projects/${projectId}/arrange`, body);

/** Suggestion d'orientation (plus grande facette vers le bas) d'un modèle. */
export const orientModel = (projectId: string, modelId: string) =>
	api.post<OrientResponse>(`/projects/${projectId}/orient`, { model_id: modelId });

/** Analyse de maillage → rapport de réparation (FR-012). */
export const repairModel = (modelId: string) =>
	api.post<RepairResponse>(`/models/${modelId}/repair`, {});
