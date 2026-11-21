// Audit 42 I1 — rendu d'Orca 2.4.1 d'un objet à cheval sur le bord du volume
// imprimable (gouraud.fs, « if the fragment is outside the print volume -> use
// darker color ») : couleur mélangée à 1/3 vers le noir hors de la boîte du
// plateau. Activé seulement pour les objets « partly inside » (Orca :
// print_volume.type = -1 sinon). Injection dans les matériaux three.js.
import * as THREE from 'three';

export interface PrintVolumeUniforms {
	uPvOn: { value: number };
	uPvMin: { value: THREE.Vector3 };
	uPvMax: { value: THREE.Vector3 };
	/** Repère de l'objet → repère du plateau (matrice locale du maillage). */
	uPvMatrix: { value: THREE.Matrix4 };
}

export function printVolumeUniforms(): PrintVolumeUniforms {
	return {
		uPvOn: { value: 0 },
		uPvMin: { value: new THREE.Vector3() },
		uPvMax: { value: new THREE.Vector3() },
		uPvMatrix: { value: new THREE.Matrix4() }
	};
}

/** Mélange d'Orca vers le noir (mix(color, ZERO, 0.3333)). */
export const OUTSIDE_DARKEN = 0.3333;

/** Ajoute l'assombrissement hors volume au matériau (même programme pour tous). */
export function attachPrintVolume(material: THREE.Material, uniforms: PrintVolumeUniforms): void {
	material.onBeforeCompile = (shader) => {
		Object.assign(shader.uniforms, uniforms);
		shader.vertexShader = shader.vertexShader
			.replace(
				'#include <common>',
				'#include <common>\nuniform mat4 uPvMatrix;\nvarying vec3 vPvPos;'
			)
			.replace(
				'#include <begin_vertex>',
				'#include <begin_vertex>\nvPvPos = (uPvMatrix * vec4(transformed, 1.0)).xyz;'
			);
		shader.fragmentShader = shader.fragmentShader
			.replace(
				'#include <common>',
				'#include <common>\nuniform float uPvOn;\nuniform vec3 uPvMin;\nuniform vec3 uPvMax;\nvarying vec3 vPvPos;'
			)
			.replace(
				'#include <color_fragment>',
				`#include <color_fragment>
if (uPvOn > 0.5 && (any(lessThan(vPvPos, uPvMin)) || any(greaterThan(vPvPos, uPvMax))))
	diffuseColor.rgb = mix(diffuseColor.rgb, vec3(0.0), ${OUTSIDE_DARKEN.toFixed(4)});`
			);
	};
	material.customProgramCacheKey = () => 'ws-print-volume';
}
