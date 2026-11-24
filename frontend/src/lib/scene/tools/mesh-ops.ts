// Placement des pièces produites par les découpes serveur (split / cut /
// boolean, `POST /api/models/{id}/{split,cut}`, `/projects/{id}/boolean`).
// Pur : le composant applique le résultat à la scène.
// Imports profonds : seules les classes de calcul, sans tirer le module `three`
// complet (renderer…) dans le chunk initial de la page projet.
import { Euler } from 'three/src/math/Euler.js';
import { Matrix4 } from 'three/src/math/Matrix4.js';
import { Quaternion } from 'three/src/math/Quaternion.js';
import { Vector3 } from 'three/src/math/Vector3.js';
import { rotateVectorByEulerDeg } from '../transform';
import { tr } from '#lib/i18n';

export type Vec3 = [number, number, number];

export interface PlacedTransform {
	position: Vec3;
	rotation: Vec3;
	scale: Vec3;
}

/**
 * Pièce d'un split / d'une coupe : `center` est exprimé dans le repère recentré
 * du maillage source. Position monde = position source + R·(S ⊙ center) ;
 * rotation et échelle sont recopiées (même géométrie apparente qu'avant).
 */
export function placeSourcePart(source: PlacedTransform, center: Vec3): PlacedTransform {
	const scaled: Vec3 = [
		center[0] * source.scale[0],
		center[1] * source.scale[1],
		center[2] * source.scale[2]
	];
	const offset = rotateVectorByEulerDeg(source.rotation, scaled);
	return {
		position: [
			source.position[0] + offset[0],
			source.position[1] + offset[1],
			source.position[2] + offset[2]
		],
		rotation: [...source.rotation],
		scale: [...source.scale]
	};
}

/**
 * Matrice monde colonne-major (`Matrix4.elements`) d'un objet de scène :
 * translation · rotation Euler XYZ (degrés) · échelle, appliquée au maillage
 * recentré — format attendu par `POST /api/projects/{id}/boolean`.
 */
export function worldMatrix(t: PlacedTransform): number[] {
	const deg = Math.PI / 180;
	const q = new Quaternion().setFromEuler(
		new Euler(t.rotation[0] * deg, t.rotation[1] * deg, t.rotation[2] * deg, 'XYZ')
	);
	return new Matrix4().compose(new Vector3(...t.position), q, new Vector3(...t.scale)).toArray();
}

/** Message utilisateur d'un échec de découpe (409 conversion, 422 refus). */
export function meshOpErrorMessage(
	feature: string,
	status: number | null,
	message: string
): string {
	if (status === 409)
		return tr('{feature}: model conversion in progress, try again in a moment.', { feature });
	if (status === 422) return tr('{feature} not possible: {message}', { feature, message });
	return tr('{feature}: failed ({message}).', { feature, message });
}
