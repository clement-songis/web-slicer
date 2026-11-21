// Vue réaliste (Orca 2.4.1 : préférences « Graphics › Realistic View »,
// GLCanvas3D.cpp:7505-7900, GLModel.cpp:460) en rendu three.js, sans calcul
// serveur :
// - Phong (opengl_realistic_phong, défaut vrai) → MeshPhongMaterial ; sinon
//   shader « gouraud » d'Orca → MeshLambertMaterial (mat, sans reflets) ;
// - ombres du plateau (opengl_phong_basic_plate_shadows, défaut faux) :
//   lumière fixe dans le repère caméra, direction œil (-0,4575, 0,4575, 0,7625) ;
// - normales lissées (opengl_phong_smooth_normals, défaut faux) :
//   igl::per_corner_normals(V, F, 5°) ≡ BufferGeometryUtils.toCreasedNormals(g, 5°).
// Orca coupe la vue réaliste pendant les gizmos Couture / Supports / Fuzzy skin
// (GLGizmosManager.cpp:1405) et la rétablit à leur fermeture ; pas pendant le MMU.
import * as THREE from 'three';
import { toCreasedNormals } from 'three/examples/jsm/utils/BufferGeometryUtils.js';
import type { PaintChannel } from './gizmos/painting/painting';

export interface RealisticOptions {
	phong: boolean;
	shadows: boolean;
	smoothNormals: boolean;
}

/** Angle de pli d'Orca (per_corner_normals, 5°). */
export const CREASE_ANGLE_DEG = 5;

/** Direction de la lumière d'Orca, repère œil (normalisée). */
export const EYE_LIGHT_DIR: [number, number, number] = (() => {
	const v = [-0.4575, 0.4575, 0.7625];
	const n = Math.hypot(v[0], v[1], v[2]);
	return [v[0] / n, v[1] / n, v[2] / n];
})();

/** Préférences graphiques utiles ici (sous-ensemble de GraphicsPreferences). */
export interface RealisticPrefs {
	realisticView: boolean;
	realisticPhong: boolean;
	realisticShadows: boolean;
	realisticSmoothNormals: boolean;
}

/**
 * Options effectives : null si la vue réaliste est éteinte ou suspendue par un
 * gizmo de peinture Couture / Supports / Fuzzy (le MMU la garde, comme Orca).
 */
export function effectiveRealistic(
	prefs: RealisticPrefs,
	paintChannel: PaintChannel | null = null
): RealisticOptions | null {
	if (!prefs.realisticView) return null;
	if (paintChannel && paintChannel !== 'mmu') return null;
	return {
		phong: prefs.realisticPhong,
		shadows: prefs.realisticShadows,
		smoothNormals: prefs.realisticSmoothNormals
	};
}

/**
 * Géométrie à normales lissées sous 5° (non indexée : l'ordre des triangles
 * est conservé, le faceIndex du picking reste celui du maillage).
 */
export function creasedGeometry(source: THREE.BufferGeometry): THREE.BufferGeometry {
	return toCreasedNormals(source, (CREASE_ANGLE_DEG * Math.PI) / 180);
}

/**
 * Direction monde de la lumière fixe d'Orca pour une caméra (rotation de la
 * direction œil par l'orientation de la caméra).
 */
export function worldLightDirection(cameraQuaternion: THREE.Quaternion): THREE.Vector3 {
	return new THREE.Vector3(...EYE_LIGHT_DIR).applyQuaternion(cameraQuaternion).normalize();
}
