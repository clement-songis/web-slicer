// SSAO de la vue réaliste (phase 39, Orca 2.4.1 opengl_phong_ssao, défaut
// faux ; GLCanvas3D::_render_ssao_pass + shaders/110/ssao.fs). Post-traitement
// écran d'une seule passe, comme Orca : la scène est rendue dans une cible
// (couleur + profondeur), puis un quadrilatère plein écran assombrit chaque
// pixel selon 8 échantillons de profondeur voisins (rayon 2–4 px), avec les
// mêmes constantes. Le plateau visible est exclu (Orca : masque stencil du
// rectangle / disque du plateau à z = 0) ; ici le pixel est reprojeté en
// coordonnées monde depuis la profondeur et exclu s'il tombe sur le plateau.
// Particularité reproduite : Orca ne lie aucune texture de normales,
// `normal_texture` lit donc l'unité 0 (la couleur) — « up_factor » vient du
// canal bleu de la couleur. Pur (chaînes GLSL + calcul des uniformes).

export const SSAO_VERTEX = /* glsl */ `
varying vec2 vUv;
void main() {
	vUv = uv;
	gl_Position = vec4(position.xy, 0.0, 1.0);
}
`;

export const SSAO_FRAGMENT = /* glsl */ `
#include <packing>
uniform sampler2D tColor;
uniform sampler2D tDepth;
uniform vec2 invTexSize;
uniform float zNear;
uniform float zFar;
uniform bool ortho;
uniform mat4 projectionInverse;
uniform mat4 viewInverse;
uniform vec4 plate; // xmin, ymin, xmax, ymax (mm)
uniform vec3 plateCircle; // cx, cy, r (r > 0 : plateau circulaire)
uniform float plateTolerance;
varying vec2 vUv;

float linearDepth(float d) {
	return ortho ? -orthographicDepthToViewZ(d, zNear, zFar) : -perspectiveDepthToViewZ(d, zNear, zFar);
}

bool onPlate(vec2 uv, float d) {
	if (d >= 1.0) return false;
	vec4 ndc = vec4(uv * 2.0 - 1.0, d * 2.0 - 1.0, 1.0);
	vec4 view = projectionInverse * ndc;
	view /= view.w;
	vec3 w = (viewInverse * view).xyz;
	// Précision de la profondeur 24 bits : constante en orthographique,
	// en d² / near en perspective.
	float dist = -view.z;
	float tol = ortho ? plateTolerance : max(plateTolerance, 4.0 * dist * dist / (zNear * 16777216.0));
	if (abs(w.z) > tol) return false;
	if (plateCircle.z > 0.0) return distance(w.xy, plateCircle.xy) <= plateCircle.z;
	return w.x >= plate.x && w.x <= plate.z && w.y >= plate.y && w.y <= plate.w;
}

void main() {
	vec4 texel = texture2D(tColor, vUv);
	// Couleur affichée (tone mapping + sRGB), comme le tampon d'Orca.
	vec4 shown = linearToOutputTexel(texel);
	float raw = texture2D(tDepth, vUv).r;
	if (onPlate(vUv, raw)) {
		gl_FragColor = shown;
		return;
	}
	vec3 base = shown.rgb;
	float depthCenter = linearDepth(raw);
	vec3 normalCenter = base * 2.0 - 1.0;
	float upFactor = max(0.0, normalCenter.z);
	float radius = mix(2.0, 4.0, depthCenter / zFar);
	vec2 offsets[8];
	offsets[0] = vec2(1.0, 0.0);
	offsets[1] = vec2(0.707, 0.707);
	offsets[2] = vec2(0.0, 1.0);
	offsets[3] = vec2(-0.707, 0.707);
	offsets[4] = vec2(-1.0, 0.0);
	offsets[5] = vec2(-0.707, -0.707);
	offsets[6] = vec2(0.0, -1.0);
	offsets[7] = vec2(0.707, -0.707);
	// Audit 40 E2 : seuil bas au-dessus du pas de quantification de la
	// profondeur 24 bits (≈ d² / (near · 2²⁴) en perspective, near = 1 mm ici
	// contre un near ajusté à la scène chez Orca) : sans lui, un plan incliné
	// (le plateau, ses bords) donnait des marches de profondeur → rayures.
	float depthStep = ortho ? (zFar - zNear) / 16777216.0 : depthCenter * depthCenter / (zNear * 16777216.0);
	float lowEdge = max(0.001, 4.0 * depthStep);
	float threshold = max(0.015 * (0.5 + depthCenter / zFar), 2.0 * lowEdge);
	float occlusion = 0.0;
	for (int i = 0; i < 8; ++i) {
		vec2 uv = clamp(vUv + offsets[i] * invTexSize * radius, vec2(0.001), vec2(0.999));
		float sampleDepth = linearDepth(texture2D(tDepth, uv).r);
		float depthDiff = max(0.0, depthCenter - sampleDepth);
		float contribution = smoothstep(lowEdge, threshold, depthDiff);
		float diagonalWeight = 1.0 - abs(offsets[i].x * offsets[i].y) * 0.5;
		occlusion += contribution * diagonalWeight;
	}
	occlusion /= 8.0;
	float ao = 1.0 - occlusion * 0.55;
	ao = clamp(ao, mix(0.45, 0.70, upFactor), 1.0);
	ao = pow(ao, 2.2) * (1.0 + upFactor * 0.15);
	ao = clamp(ao, 0.45, 1.05);
	gl_FragColor = vec4(base * ao, shown.a);
}
`;

/** Plateau exclu de l'occlusion : rectangle (mm) ou disque. */
export interface SsaoPlate {
	rect: [number, number, number, number];
	circle: [number, number, number];
}

/**
 * Plateau d'après le lit de la scène : rectangle aligné, sinon disque inscrit
 * (BuildVolume_Type::Circle d'Orca — les plateaux non rectangulaires des
 * profils sont des disques, delta).
 */
export function ssaoPlate(bed: {
	center: { x: number; y: number };
	width: number;
	depth: number;
	rectangular: boolean;
}): SsaoPlate {
	const hw = bed.width / 2;
	const hd = bed.depth / 2;
	const r = bed.rectangular ? 0 : Math.min(bed.width, bed.depth) / 2;
	return {
		rect: [bed.center.x - hw, bed.center.y - hd, bed.center.x + hw, bed.center.y + hd],
		circle: [bed.center.x, bed.center.y, r]
	};
}

/**
 * Tolérance de base (mm) en z pour reconnaître un pixel du plateau (surface
 * posée à −0,05 mm) ; le shader l'élargit au loin selon la précision de la
 * profondeur en perspective.
 */
export const PLATE_TOLERANCE_MM = 0.25;
