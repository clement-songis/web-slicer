// Appels des outils moteur de scène (endpoints T054/T055) : arrangement,
// orientation, réparation et simplification (rapports). Typés sur les DTO
// générés par ts-rs.
import { api } from './client';
import type {
	ArrangeOptionsInfo,
	ArrangeRequest,
	ArrangeResponse,
	OrientResponse,
	RepairResponse
} from './types';
import type { SimplifyRequest } from '../../generated/api/SimplifyRequest';
import type { SimplifyResponse } from '../../generated/api/SimplifyResponse';

import type { CutRequest } from '../../generated/api/CutRequest';
import type { BooleanRequest } from '../../generated/api/BooleanRequest';
import type { MeshOpResponse } from '../../generated/api/MeshOpResponse';
import type { MeshOpPartResponse } from '../../generated/api/MeshOpPartResponse';

export type { SimplifyRequest, SimplifyResponse };
export type { CutRequest, BooleanRequest, MeshOpResponse, MeshOpPartResponse };

/** Arrangement sans collision des empreintes fournies (FR-013). */
export const arrangeScene = (projectId: string, body: ArrangeRequest) =>
	api.post<ArrangeResponse>(`/projects/${projectId}/arrange`, body);

/**
 * Valeurs du bouton « Réinitialiser » du menu « Options d'arrangement » d'Orca
 * et disponibilité des cases (lot 34) : GET /api/projects/{id}/arrange/options.
 */
export const getArrangeOptions = (projectId: string) =>
	api.get<ArrangeOptionsInfo>(`/projects/${projectId}/arrange/options`);

/** Suggestion d'orientation (plus grande facette vers le bas) d'un modèle. */
export const orientModel = (projectId: string, modelId: string) =>
	api.post<OrientResponse>(`/projects/${projectId}/orient`, { model_id: modelId });

/** Analyse de maillage → rapport de réparation (FR-012). */
export const repairModel = (modelId: string) =>
	api.post<RepairResponse>(`/models/${modelId}/repair`, {});

/**
 * Simplification quadrique serveur (libslic3r) : remplace le maillage stocké
 * du modèle et renvoie les comptes avant/après. Recharger ensuite `/mesh`.
 */
export const simplifyModel = (modelId: string, body: SimplifyRequest) =>
	api.post<SimplifyResponse>(`/models/${modelId}/simplify`, body);

/**
 * Séparation en composantes connexes (`TriangleMesh::split`) : chaque pièce
 * devient un nouveau modèle (maillage prêt) ; `center` est exprimé dans le
 * repère recentré du modèle source. 422 si l'objet n'a qu'une pièce.
 */
export const splitModel = (modelId: string) =>
	api.post<MeshOpResponse>(`/models/${modelId}/split`, {});

/**
 * Subdivision de Loop (`TriangleMeshDeal::smooth_triangle_mesh`, « Subdivision
 * mesh (Lost color) » d'Orca) : nouveau modèle `result` (×4 triangles), centre
 * dans le repère recentré de la source. Maillage ouvert → 422.
 */
export const smoothModel = (modelId: string) =>
	api.post<MeshOpResponse>(`/models/${modelId}/smooth`, {});

/** Coupe plane (`cut_mesh`, sections bouchées), plan dans le repère recentré. */
export const cutModel = (modelId: string, body: CutRequest) =>
	api.post<MeshOpResponse>(`/models/${modelId}/cut`, body);

/** Booléen mcut entre deux modèles placés par leurs matrices monde. */
export const booleanModels = (projectId: string, body: BooleanRequest) =>
	api.post<MeshOpResponse>(`/projects/${projectId}/boolean`, body);

/** Polices disponibles pour le texte en relief (outil Emboss). */
export const listEmbossFonts = () => api.get<{ id: string; label: string }[]>('/emboss/fonts');

/** Texte en relief (Emboss::text2shapes + polygons2model) : crée un modèle du projet. */
export const embossText = (
	projectId: string,
	body: { text: string; font: string; size_mm: number; depth_mm: number; boldness_mm?: number }
) => api.post<MeshOpResponse>(`/projects/${projectId}/emboss`, body);

/**
 * Primitive du menu « Add » générée par libslic3r (`its_make_*` avec les
 * paramètres de `ObjectList::create_mesh` d'Orca) : crée un modèle du projet.
 * Le côté (0,1 × plus grande dimension du plateau) est calculé par le serveur.
 */
export const createPrimitive = (
	projectId: string,
	body: {
		shape: 'cube' | 'cylinder' | 'sphere' | 'cone' | 'disc' | 'torus';
		bed_size_mm: [number, number];
	}
) => api.post<MeshOpResponse>(`/projects/${projectId}/primitive`, body);

/**
 * Profil de hauteur de couche variable calculé par libslic3r (Slicing.cpp :
 * `layer_height_profile_adaptive` / `smooth_height_profile`, boutons
 * « Adaptive » / « Smooth » d'Orca). Profil plat Orca `[z0, h0, z1, h1, …]`.
 */
export const layerProfileModel = (
	modelId: string,
	body: {
		op:
			| { kind: 'adaptive'; quality: number }
			| { kind: 'smooth'; profile: number[]; radius: number; keep_min: boolean };
		values: Record<string, unknown>;
		matrix?: number[];
	}
) => api.post<{ profile: number[] }>(`/models/${modelId}/layer-profile`, body);

/** Forme SVG en relief (nanosvg → polygons2model) : crée un modèle du projet. */
export const embossSvg = (
	projectId: string,
	body: { svg: string; width_mm: number; depth_mm: number }
) => api.post<MeshOpResponse>(`/projects/${projectId}/emboss-svg`, body);
