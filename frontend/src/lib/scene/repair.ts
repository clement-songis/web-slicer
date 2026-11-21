// Réparation de maillage (FR-012) côté éditeur : choix du modèle serveur à
// réparer depuis l'objet ciblé, remplacement du maillage réparé sur toutes les
// instances du modèle en conservant leurs transformations, et résumé du
// rapport admesh. Pur (aucun accès réseau/DOM) → testable ; la page effectue
// l'appel `POST /api/models/{id}/repair` puis le rechargement `/mesh`.
//
// Même schéma pour la simplification serveur (T055) : cible = modèle serveur
// (`repairTarget`), remplacement du maillage (`replaceMesh`), puis résumé du
// rapport ; `simplifyRequest` valide le paramètre de l'outil avant l'appel.
import type { ImportItem } from '../editor/imports';
import type { RepairResponse } from '../api/types';
import type { SimplifyRequest, SimplifyResponse } from '../api/scene';
import { tr } from '$lib/i18n';

/** Cible d'une réparation : modèle serveur, ou raison du refus. */
export type RepairTarget = { ok: true; modelId: string } | { ok: false; reason: string };

/**
 * Modèle serveur à réparer pour un objet : il faut un import téléversé et
 * converti (`ready`). Un objet en cours d'upload/conversion ou en échec n'a pas
 * de maillage serveur réparable.
 */
export function repairTarget(objectId: string, imports: readonly ImportItem[]): RepairTarget {
	const item = imports.find((i) => i.objectId === objectId);
	if (!item || !item.modelId) {
		return { ok: false, reason: tr('object not attached to a server model (upload in progress?)') };
	}
	if (item.status === 'uploading' || item.status === 'converting') {
		return { ok: false, reason: tr('wait for the model conversion to finish') };
	}
	if (item.status === 'failed') {
		return { ok: false, reason: tr('the model conversion failed') };
	}
	return { ok: true, modelId: item.modelId };
}

/** Ids des objets (instances) adossés à un modèle serveur. */
export function instancesOfModel(imports: readonly ImportItem[], modelId: string): Set<string> {
	return new Set(imports.filter((i) => i.modelId === modelId).map((i) => i.objectId));
}

/**
 * Remplace le maillage des objets listés par le maillage réparé. Position,
 * rotation, échelle et autres champs sont conservés ; les autres objets sont
 * renvoyés tels quels (même référence).
 */
export function replaceMesh<T extends { id: string; mesh: M }, M>(
	objects: readonly T[],
	objectIds: ReadonlySet<string>,
	mesh: M
): T[] {
	return objects.map((o) => (objectIds.has(o.id) ? { ...o, mesh } : o));
}

/** Nombre total de corrections réalisées par admesh. */
export function repairCount(report: RepairResponse): number {
	const r = report.repairs;
	return (
		r.edges_fixed + r.degenerate_facets + r.facets_removed + r.facets_reversed + r.backwards_edges
	);
}

/** Libellés Orca des niveaux de détail (0 = détail maximal). */
export const SIMPLIFY_DETAIL_LEVELS = ['Extra high', 'High', 'Medium', 'Low', 'Extra low'] as const;

/** Paramètre de simplification validé, ou raison du refus. */
export type SimplifyParam = { ok: true; body: SimplifyRequest } | { ok: false; reason: string };

/**
 * Valide le paramètre de l'outil (mêmes bornes que le serveur) : ratio de
 * décimation en % dans ]0, 100[, ou niveau de détail entier 0..4.
 */
export function simplifyRequest(
	param: { mode: 'ratio'; decimateRatio: number } | { mode: 'detail'; level: number }
): SimplifyParam {
	if (param.mode === 'ratio') {
		const r = param.decimateRatio;
		if (!Number.isFinite(r) || r <= 0 || r >= 100) {
			return { ok: false, reason: tr('the decimation ratio must be between 0 and 100 %') };
		}
		return { ok: true, body: { mode: 'ratio', decimate_ratio: r } };
	}
	const l = param.level;
	if (!Number.isInteger(l) || l < 0 || l >= SIMPLIFY_DETAIL_LEVELS.length) {
		return { ok: false, reason: tr('unknown detail level') };
	}
	return { ok: true, body: { mode: 'detail', level: l } };
}

/** Message de fin de simplification à partir du rapport serveur. */
export function simplifySummary(report: SimplifyResponse): string {
	const before = report.triangles_before;
	const after = report.triangles_after;
	if (after >= before)
		return tr('Simplification had no effect: {n} triangles kept.', { n: before });
	const pct = Math.round((1 - after / before) * 100);
	const state = report.watertight
		? tr('watertight')
		: tr('not watertight ({n} open edge(s))', { n: report.open_edges });
	return tr('Mesh simplified: {before} → {after} triangles (−{pct} %), {state}.', {
		before,
		after,
		pct,
		state
	});
}

/** Message de fin de réparation à partir du rapport serveur. */
export function repairSummary(report: RepairResponse): string {
	const fixes = repairCount(report);
	const state = report.watertight
		? tr('watertight')
		: tr('not watertight ({n} open edge(s))', { n: report.open_edges });
	const what = fixes > 0 ? tr('{n} fix(es)', { n: fixes }) : tr('no fix needed');
	return tr('Mesh repaired: {what}, {n} triangles, {state}.', {
		what,
		n: report.triangles,
		state
	});
}
