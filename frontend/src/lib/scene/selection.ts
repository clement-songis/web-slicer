// Gestion de la sélection d'objets de la scène (T050). Le raycasting (Three.js)
// fournit l'id touché ; la logique de sélection — pure et testable — décide de
// l'ensemble sélectionné selon le modificateur (multi-sélection).

/** Applique un clic de sélection.
 *
 * - `hitId === null` (clic dans le vide) : vide la sélection (sans modificateur)
 *   ou la conserve (avec modificateur additif).
 * - additif (`Shift`/`Ctrl`) : bascule l'objet touché dans l'ensemble.
 * - simple : remplace la sélection par l'unique objet touché.
 */
export function applyPick(
	current: ReadonlySet<string>,
	hitId: string | null,
	additive: boolean
): Set<string> {
	if (hitId === null) {
		return additive ? new Set(current) : new Set();
	}
	if (additive) {
		const next = new Set(current);
		if (next.has(hitId)) next.delete(hitId);
		else next.add(hitId);
		return next;
	}
	return new Set([hitId]);
}

/** L'objet est-il sélectionné ? */
export function isSelected(selection: ReadonlySet<string>, id: string): boolean {
	return selection.has(id);
}
