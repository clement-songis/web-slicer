// Tab = « basculer Préparer/Aperçu » (raccourci Orca, actif quand le canevas
// 3D a le focus — GLCanvas3D::on_key). Côté web, la touche ne bascule la vue
// que si le focus est dans la zone 3D ou nulle part (body) ; dès qu'un élément
// de l'interface a le focus (liste d'objets, barre latérale, onglets…), Tab
// garde la navigation clavier native.

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

/** `true` si Tab doit basculer Préparer/Aperçu pour ce focus. */
export function tabTogglesView(target: FocusTarget): boolean {
	if (!target || typeof target.closest !== 'function') return true;
	const tag = (target.tagName ?? '').toUpperCase();
	if (tag === 'BODY' || tag === 'HTML') return true;
	if (CONTROL_TAGS.has(tag)) return false;
	return !!target.closest(VIEW_ZONE_SELECTOR);
}
