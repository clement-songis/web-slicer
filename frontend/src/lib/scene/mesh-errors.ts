// Erreurs de maillage par objet (lot 37, Orca ObjectList::get_mesh_errors_info
// / get_warning_icon_name) : icône d'alerte dans la liste d'objets quand le
// maillage a des arêtes ouvertes (non-manifold) ou des erreurs corrigées à la
// lecture ; infobulle d'Orca en français ; clic → « Réparer le modèle ».
// Statistiques calculées par libslic3r côté serveur
// (GET /api/projects/{id}/mesh-errors, scène enregistrée). Pur.
import type { ObjectMeshErrors } from '../../generated/api/ObjectMeshErrors';

/** Alerte affichée pour un objet. */
export interface MeshWarning {
	/** Infobulle (textes d'Orca, traduction française). */
	tooltip: string;
	/** Arêtes ouvertes restantes : l'icône propose la réparation. */
	openEdges: number;
	repairedErrors: number;
}

const plural = (n: number, one: string, many: string) =>
	(n > 1 ? many : one).replace('%1$d', String(n));

/** Infobulle d'Orca (msgids traduits par OrcaSlicer_fr.po). */
export function meshErrorsTooltip(
	e: Pick<ObjectMeshErrors, 'open_edges' | 'repaired_errors' | 'manifold' | 'repaired'>
): string {
	if (!e.repaired && e.manifold) return '';
	let tip = '';
	if (e.repaired)
		tip += plural(e.repaired_errors, '%1$d erreur réparée', '%1$d erreurs réparées') + '\n';
	if (!e.manifold)
		tip +=
			'Erreurs restantes :\n\t' +
			plural(e.open_edges, '%1$d arête non-manifold', '%1$d arêtes non-manifold') +
			'\n';
	return tip + '\nCliquer sur l’icône pour réparer l’objet modèle';
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
