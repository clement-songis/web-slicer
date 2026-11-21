// Audit 42 A2 — clavier de la barre de menus (motif WAI-ARIA « menubar », menus
// natifs d'Orca : Alt+lettre, flèches). Fonctions pures, testées sans DOM.

/** Lettre d'accès d'un titre de menu : première lettre, sans accent, en
 *  minuscule (« Édition » → « e », « Fichier » → « f »). */
export function menuMnemonic(title: string): string {
	const first = title
		.trim()
		.normalize('NFD')
		.replace(/[\u0300-\u036f]/g, '')
		.charAt(0);
	return first.toLowerCase();
}

/** Index du menu ouvert par Alt+`letter` (titres affichés), -1 sinon. */
export function menuForMnemonic(titles: readonly string[], letter: string): number {
	const l = letter.toLowerCase();
	if (!/^[a-z]$/.test(l)) return -1;
	return titles.findIndex((t) => menuMnemonic(t) === l);
}

/** Lettre d'un événement Alt+lettre (`code` « KeyF » : indépendant de la
 *  disposition et des caractères produits par Option sur macOS), sinon null. */
export function altLetter(e: {
	altKey: boolean;
	ctrlKey: boolean;
	metaKey: boolean;
	shiftKey: boolean;
	code?: string;
}): string | null {
	if (!e.altKey || e.ctrlKey || e.metaKey || e.shiftKey) return null;
	const m = /^Key([A-Z])$/.exec(e.code ?? '');
	return m ? m[1].toLowerCase() : null;
}

/** Déplacement dans une liste circulaire (flèches, Début / Fin). */
export function moveIndex(current: number, count: number, key: string): number {
	if (count <= 0) return -1;
	switch (key) {
		case 'ArrowDown':
			return current < 0 ? 0 : (current + 1) % count;
		case 'ArrowUp':
			return current < 0 ? count - 1 : (current - 1 + count) % count;
		case 'Home':
			return 0;
		case 'End':
			return count - 1;
		default:
			return current;
	}
}
