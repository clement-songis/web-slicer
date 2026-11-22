// Raison de blocage du bouton Trancher (audit 49 PERF49-a) — pure, testable.
//
// Sous charge, l'ouverture d'un 3MF laisse le bouton désactivé de longues
// dizaines de secondes pendant que les maillages des modèles déjà importés
// sont récupérés et décodés un à un (l'objet figure dans la liste mais pas
// encore dans la scène 3D) ; l'infobulle disait à tort « Ajoutez un objet à
// trancher ». La raison de blocage est désormais dérivée de l'état des
// imports (événements `model.converted`, `resolveConversion`), pas d'un
// minuteur : dès que le dernier maillage atterrit, le bouton s'active.

export type SliceBlockerReason =
	/** Objets du projet connus (imports prêts) mais maillages pas encore chargés. */
	| 'loading-objects'
	/** Aucun objet sur le plateau, rien en attente. */
	| 'add-object'
	/** Un objet est hors du volume imprimable. */
	| 'outside'
	/** Pas de profil de processus. */
	| 'process'
	/** Aucun filament. */
	| 'filament'
	/** Import en cours (téléversement ou conversion moteur). */
	| 'importing';

export interface SliceBlockerInputs {
	/** Objets rendus sur le plateau actif (maillages chargés). */
	objectCount: number;
	/** Objets du plateau actif hors du volume imprimable. */
	outsideCount: number;
	/** Imports en cours (téléversement ou conversion — `isPending`). */
	importPending: boolean;
	/** Profil de processus actif. */
	hasProcess: boolean;
	/** Nombre de filaments actifs. */
	filamentCount: number;
	/** Imports prêts (modèle converti) dont l'objet n'est pas encore rendu :
	 *  maillages attendus à l'ouverture du projet (dériver des imports et de
	 *  la scène, jamais d'un délai). */
	meshesLoading: number;
}

/**
 * Raison du blocage du tranchage, `null` si tranchable. L'activation reste
 * exactement celle d'avant (elle exige les maillages : la requête de
 * tranchage n'embarque que les objets rendus) — seul le retour d'état
 * change : les objets en cours de chargement ne sont plus annoncés
 * « Ajoutez un objet à trancher » mais « Chargement des objets… ».
 */
export function sliceBlockerReason(i: SliceBlockerInputs): SliceBlockerReason | null {
	if (i.objectCount === 0) {
		// PERF49-a : des objets attendent leur maillage (ouverture de 3MF) ou un
		// import est en vol → l'état suit l'import, pas un timer.
		if (i.meshesLoading > 0 || i.importPending) return 'loading-objects';
		return 'add-object';
	}
	if (i.outsideCount > 0) return 'outside';
	if (!i.hasProcess) return 'process';
	if (i.filamentCount === 0) return 'filament';
	if (i.importPending) return 'importing';
	return null;
}
