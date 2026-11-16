// Règles pures de présentation et d'interaction de la scène. Le composant
// d'édition fournit l'état réactif ; ces fonctions gardent le filtrage par
// plateau, le masquage et le verrouillage cohérents entre toutes les actions.

export interface PlateMembership {
	id: string;
	objectIds: readonly string[];
}

/** Objets appartenant au plateau actif, dans leur ordre de scène. */
export function objectsOnPlate<T extends { id: string }>(
	objects: readonly T[],
	plate: PlateMembership | null | undefined
): T[] {
	if (!plate) return [];
	const ids = new Set(plate.objectIds);
	return objects.filter((object) => ids.has(object.id));
}

/** Objets réellement rendus : plateau actif, puis masquage effectif. */
export function viewportObjects<T extends { id: string }>(
	objects: readonly T[],
	plate: PlateMembership | null | undefined,
	isHidden: (id: string) => boolean
): T[] {
	return objectsOnPlate(objects, plate).filter((object) => !isHidden(object.id));
}

/** Un objet ne peut être transformé que sur son plateau actif et déverrouillé. */
export function canTransformObject(
	id: string,
	plate: PlateMembership | null | undefined,
	isLocked: (id: string) => boolean
): boolean {
	return Boolean(plate?.objectIds.includes(id)) && !isLocked(id);
}

/** Cibles d'une action globale (arrangement) : actives, visibles et mobiles. */
export function movableObjectsOnPlate<T extends { id: string }>(
	objects: readonly T[],
	plate: PlateMembership | null | undefined,
	isLocked: (id: string) => boolean
): T[] {
	return objectsOnPlate(objects, plate).filter((object) => !isLocked(object.id));
}
