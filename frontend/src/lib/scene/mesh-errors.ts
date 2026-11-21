// Erreurs de maillage par objet (lot 37, Orca ObjectList::get_mesh_errors_info
// / get_warning_icon_name) : icône d'alerte dans la liste d'objets quand le
// maillage a des arêtes ouvertes (non-manifold) ou des erreurs corrigées à la
// lecture ; infobulle d'Orca en français ; clic → « Réparer le modèle ».
// Statistiques calculées par libslic3r côté serveur
// (GET /api/projects/{id}/mesh-errors, scène enregistrée). Pur.
import type { ObjectMeshErrors } from '../../generated/api/ObjectMeshErrors';
import type { MeshErrorsRequest } from '../../generated/api/MeshErrorsRequest';
import { tr } from '$lib/i18n';

/** Alerte affichée pour un objet. */
export interface MeshWarning {
	/** Infobulle (textes d'Orca, traduction française). */
	tooltip: string;
	/** Arêtes ouvertes restantes : l'icône propose la réparation. */
	openEdges: number;
	repairedErrors: number;
}

// Clés i18n anglaises (msgids d'Orca, %1$d → {n}), traduites à l'appel.
const plural = (n: number, one: string, many: string) => tr(n > 1 ? many : one, { n });

/** Infobulle d'Orca (msgids traduits par OrcaSlicer_fr.po). */
export function meshErrorsTooltip(
	e: Pick<ObjectMeshErrors, 'open_edges' | 'repaired_errors' | 'manifold' | 'repaired'>
): string {
	if (!e.repaired && e.manifold) return '';
	let tip = '';
	if (e.repaired)
		tip += plural(e.repaired_errors, '{n} error repaired', '{n} errors repaired') + '\n';
	if (!e.manifold)
		tip +=
			tr('Remaining errors:') +
			'\n\t' +
			plural(e.open_edges, '{n} non-manifold edge', '{n} non-manifold edges') +
			'\n';
	return tip + '\n' + tr('Click the icon to repair model object');
}

/** Alertes par objet (objets sans alerte absents). */
export function meshWarnings(list: readonly ObjectMeshErrors[]): Record<string, MeshWarning> {
	const out: Record<string, MeshWarning> = {};
	for (const e of list) {
		if (!e.warning) continue;
		out[e.object_id] = {
			tooltip: meshErrorsTooltip(e),
			openEdges: e.open_edges,
			repairedErrors: e.repaired_errors
		};
	}
	return out;
}

/** Objet de scène minimal lu par `POST /api/projects/{id}/mesh-errors`. */
export interface MeshErrorsSceneObject {
	id: string;
	modelId?: string;
	volumes?: readonly { shape?: string; modelId?: string }[];
}

/**
 * Lot 39 — corps de `POST /api/projects/{id}/mesh-errors` : objets de la scène
 * COURANTE (ids vivants, y compris ceux importés et pas encore enregistrés,
 * comme Orca qui calcule l'alerte dès ObjectList::add_object_to_list). Seuls
 * `id`, `modelId` et les volumes maillés sont transmis.
 */
export function meshErrorsRequest(objects: readonly MeshErrorsSceneObject[]): MeshErrorsRequest {
	return {
		objects: objects.map((o) => {
			const volumes = (o.volumes ?? [])
				.filter((v) => v.shape === 'mesh' && typeof v.modelId === 'string')
				.map((v) => ({ shape: 'mesh', modelId: v.modelId }));
			return {
				id: o.id,
				...(o.modelId ? { modelId: o.modelId } : {}),
				...(volumes.length ? { volumes } : {})
			};
		})
	};
}

/** Clé de la requête : on ne relance la statistique que si les maillages changent. */
export function meshErrorsKey(req: MeshErrorsRequest): string {
	return JSON.stringify(req.objects);
}
