// Tab = « basculer Préparer/Aperçu » (raccourci Orca, actif seulement quand le
// canevas 3D a le focus — GLCanvas3D::on_key, GLCanvas3D.cpp:3710/3808). Côté
// web, la zone 3D « a le focus » quand l'élément focalisé y est, ou quand rien
// n'est focalisé (body) et que le dernier clic a eu lieu dans la zone 3D.
// Partout ailleurs — page tout juste ouverte, clic dans un panneau, focus sur un
// contrôle — Tab garde la navigation clavier native (audit 42 A1 ; WCAG 2.1.1,
// 2.4.3).

/** Conteneurs de la zone 3D (scène de préparation et aperçu). */
export const VIEW_ZONE_SELECTOR =
	'[data-testid="scene-canvas"], [data-testid="preview-canvas"], [data-testid="preview-view"]';

type FocusTarget = {
	closest?: (selector: string) => unknown;
	tagName?: string;
} | null;

/** Contrôles posés sur la vue 3D (barres d'outils flottantes, légende,
 *  curseurs de couches…) : Tab y garde la navigation native, sinon un
 *  utilisateur clavier serait bloqué dès qu'il atteint la scène (accessibilité :
 *  pas de piège au clavier, WCAG 2.1.2). */
const CONTROL_TAGS = new Set(['A', 'BUTTON', 'INPUT', 'SELECT', 'TEXTAREA', 'SUMMARY']);
const CONTROL_ROLES = '[role="button"], [role="menuitem"], [role="tab"], [role="slider"]';

/** Barre supérieure de l'éditeur, hors contrôles : comme la barre de titre /
 *  d'outils d'Orca, un clic dessus ne prend pas le focus clavier — il reste au
 *  canevas 3D (contrairement à un clic dans un panneau latéral). */
export const EDITOR_CHROME_SELECTOR = '[data-testid="editor-topbar"]';

/** `true` si un clic sur `target` donne (ou laisse) le focus à la zone 3D :
 *  canevas de la scène ou de l'aperçu, ou fond de la barre supérieure ; jamais
 *  un contrôle ni un panneau. */
export function pointerFocusesView(target: FocusTarget): boolean {
	if (!target || typeof target.closest !== 'function') return false;
	const tag = (target.tagName ?? '').toUpperCase();
	if (CONTROL_TAGS.has(tag)) return false;
	if (target.closest(CONTROL_ROLES)) return false;
	return !!target.closest(VIEW_ZONE_SELECTOR) || !!target.closest(EDITOR_CHROME_SELECTOR);
}

/** `true` si Tab doit basculer Préparer/Aperçu pour ce focus.
 *  `viewFocused` : le dernier clic a donné le focus à la zone 3D. */
export function tabTogglesView(target: FocusTarget, viewFocused = false): boolean {
	if (!target || typeof target.closest !== 'function') return viewFocused;
	const tag = (target.tagName ?? '').toUpperCase();
	if (tag === 'BODY' || tag === 'HTML') return viewFocused;
	if (CONTROL_TAGS.has(tag)) return false;
	return !!target.closest(VIEW_ZONE_SELECTOR);
}
