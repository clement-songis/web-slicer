// Orchestrateur d'import de modèles dans l'éditeur (T089) : suit le cycle de vie
// de chaque fichier importé (upload → conversion moteur → prêt) et le relie à un
// objet de la scène. Pur (aucun accès réseau/DOM) → testable ; le composant
// `projects/[id]` effectue les IO (uploadModel/fetchMesh) et applique ces
// transitions. Plus d'aperçu client (R7/T126) : le moteur décode tous les
// formats, le maillage arrive via `model.converted`.

import { tr } from '#lib/i18n';

/** Étape d'un import. */
export type ImportStatus = 'uploading' | 'converting' | 'ready' | 'failed';

/** Un import en cours, relié à un objet de scène par `objectId`. */
export interface ImportItem {
	/** Id du nœud scène + liste (stable, = `SceneObject.id`). */
	objectId: string;
	filename: string;
	status: ImportStatus;
	/** Renseigné après upload — sert à retrouver l'item à `model.converted`. */
	modelId: string | null;
	error: string | null;
	/** Audit 42 C1 : liaison perdue pendant l'upload ou le chargement du
	 *  maillage — reprise automatique au retour de la connexion. */
	waiting?: boolean;
}

// Jeu de formats accepté à l'upload (aligné sur `backend::detect_format` et le
// jeu cross-plateforme d'OrcaSlicer, T091). `.oltp` = alias STL ; `.amf`/`.xml`
// = AMF. `.usd*/.abc/.ply` (Apple-only) et `.zip` (conteneur) → exclusions.md.
const UPLOADABLE = new Set([
	'stl',
	'obj',
	'3mf',
	'oltp',
	'step',
	'stp',
	'amf',
	'xml',
	'svg',
	'drc'
]);
/** Extension en minuscules d'un nom de fichier (sans le point). */
export function importExt(filename: string): string {
	return filename.split('.').pop()?.toLowerCase() ?? '';
}

/** Le fichier peut-il être importé (format accepté à l'upload) ? */
export function isAccepted(filename: string): boolean {
	return UPLOADABLE.has(importExt(filename));
}

/** Démarre un import (upload en cours ; le maillage viendra du moteur). */
export function startImport(objectId: string, filename: string): ImportItem {
	return { objectId, filename, status: 'uploading', modelId: null, error: null };
}

/**
 * Upload terminé : `ready` si un maillage est déjà disponible, sinon
 * `converting` (STEP → conversion moteur asynchrone, cf. `model.converted`).
 */
export function markUploaded(
	item: ImportItem,
	modelId: string,
	conversionPending: boolean
): ImportItem {
	return { ...item, modelId, status: conversionPending ? 'converting' : 'ready' };
}

/** Maillage serveur récupéré (fin de conversion) : import prêt. */
export function markConverted(item: ImportItem): ImportItem {
	return { ...item, status: 'ready' };
}

/** Audit 42 C1 : liaison perdue — l'import reste en cours (sauvegarde et
 *  tranchage attendent) et reprendra au retour de la connexion. */
export function markWaiting(item: ImportItem): ImportItem {
	return {
		...item,
		status: item.modelId ? 'converting' : 'uploading',
		error: null,
		waiting: true
	};
}

/** Reprise après le retour de la connexion. */
export function clearWaiting(item: ImportItem): ImportItem {
	const next = { ...item };
	delete next.waiting;
	return next;
}

/**
 * Échec dû à la liaison (requête rejetée par le navigateur, client HTTP
 * « network », proxy 502/503/504, navigateur hors ligne) plutôt qu'au fichier.
 */
export function isConnectionError(e: unknown, offline = false): boolean {
	if (offline) return true;
	if (e instanceof TypeError) return true;
	const status = (e as { status?: unknown } | null)?.status;
	const code = (e as { code?: unknown } | null)?.code;
	return code === 'network' || status === 0 || status === 502 || status === 503 || status === 504;
}

/** Échec (aperçu, upload ou conversion) : erreur figée. */
export function markFailed(item: ImportItem, message: string): ImportItem {
	return { ...item, status: 'failed', error: message };
}

/** Retrouve l'import correspondant à un modèle converti (event `model.converted`). */
export function findByModel(items: ImportItem[], modelId: string): ImportItem | undefined {
	return items.find((i) => i.modelId === modelId);
}

/** Un import est-il en cours (upload ou conversion moteur, pas encore prêt) ? */
export function isPending(item: ImportItem): boolean {
	return item.status === 'uploading' || item.status === 'converting';
}

/** Libellé court d'un import pour le placeholder / badge d'état (T127). */
export function importLabel(item: ImportItem): string {
	if (item.waiting) return tr('waiting for the connection…');
	switch (item.status) {
		case 'uploading':
			return tr('importing…');
		case 'converting':
			return tr('converting…');
		case 'failed':
			return item.error ?? tr('failed');
		case 'ready':
			return tr('ready');
	}
}
