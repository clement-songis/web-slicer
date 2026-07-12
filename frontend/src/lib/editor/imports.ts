// Orchestrateur d'import de modèles dans l'éditeur (T089) : suit le cycle de vie
// de chaque fichier importé (upload → conversion moteur → prêt) et le relie à un
// objet de la scène. Pur (aucun accès réseau/DOM) → testable ; le composant
// `projects/[id]` effectue les IO (uploadModel/fetchMesh) et applique ces
// transitions. Plus d'aperçu client (R7/T126) : le moteur décode tous les
// formats, le maillage arrive via `model.converted`.

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

/** Échec (aperçu, upload ou conversion) : erreur figée. */
export function markFailed(item: ImportItem, message: string): ImportItem {
	return { ...item, status: 'failed', error: message };
}

/** Retrouve l'import correspondant à un modèle converti (event `model.converted`). */
export function findByModel(items: ImportItem[], modelId: string): ImportItem | undefined {
	return items.find((i) => i.modelId === modelId);
}
