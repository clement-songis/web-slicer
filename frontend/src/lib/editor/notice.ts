// Barre de messages de l'éditeur (phase 31) : chaque affectation de
// `saveMessage` est « consommée » (remise à null) et devient une notification
// numérotée. Un message identique au précédent produit donc une nouvelle
// notification (nouveau numéro) : il réapparaît même après l'estompage, comme
// les notifications d'Orca qui se reposent à chaque événement.

/** Notification affichée : texte et numéro (change à chaque message, même identique). */
export interface Notice {
	text: string;
	seq: number;
}

/** Notification suivante pour `text` ; `null` (rien à afficher) laisse la précédente. */
export function nextNotice(prev: Notice | null, text: string | null | undefined): Notice | null {
	if (text === null || text === undefined || text === '') return prev;
	return { text, seq: (prev?.seq ?? 0) + 1 };
}
