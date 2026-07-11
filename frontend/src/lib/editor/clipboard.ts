// Presse-papier de scène (menu Édition, T108) : copie/coupe/colle des objets
// sélectionnés. Pur → testable sous bun ; la page génère les identifiants réels
// (nœuds d'arbre) et applique le décalage de collage. Le maillage est partagé
// par référence (immuable côté rendu), seule la position est décalée.
import type { SceneObject } from '../scene/mesh';

/** Décalage (mm) appliqué à chaque collage, pour éviter la superposition exacte. */
export const PASTE_OFFSET = 10;

/** Instantané des objets sélectionnés (copie superficielle, maillage partagé). */
export function copyObjects(
	objects: readonly SceneObject[],
	selection: ReadonlySet<string>
): SceneObject[] {
	return objects.filter((o) => selection.has(o.id)).map((o) => ({ ...o }));
}

/** Position décalée d'un objet collé (X/Y décalés, Z préservée). */
export function pastedPosition(
	position: [number, number, number] | undefined
): [number, number, number] {
	const [x, y, z] = position ?? [0, 0, 0];
	return [x + PASTE_OFFSET, y + PASTE_OFFSET, z];
}
