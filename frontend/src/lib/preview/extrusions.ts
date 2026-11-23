// Rendu des extrusions en volumes pleins (parité GCodeViewer d'Orca) : chaque
// segment devient une instance d'un prisme à section ovale (hexagone aplati,
// largeur × hauteur réelles du segment), orienté le long du segment et centré
// sous la buse (la ligne G-code est le dessus du cordon). Module pur : produit
// la section et les matrices d'instance ; le composant les pose dans un
// `THREE.InstancedMesh` éclairé.

/** Nombre de côtés de la section (6 : ovale lisible, 20 triangles/instance). */
export const SECTION_SIDES = 6;

/** Au-delà, rendu en lignes par défaut (mode auto) : 8 M triangles, ≈ 29 Mo de
 *  tampons d'instance, ≈ 95 ms de construction CPU (mesuré, phase6/build-cost). */
export const VOLUME_SEGMENT_LIMIT = 400_000;

export type RenderMode = 'auto' | 'volumes' | 'lines';

/**
 * Mode effectif : `auto` → volumes sous la limite, lignes au-delà — et lignes
 * en rendu WebGL **logiciel** (pas de GPU : SwiftShader, llvmpipe…), où une
 * image en volumes bloque le fil principal plusieurs secondes (mesuré phase 25 :
 * 2,8 à 4 s pour 100 couches, au-delà de 15 s sous charge).
 */
export function effectiveRenderMode(
	mode: RenderMode,
	segments: number,
	software = false
): 'volumes' | 'lines' {
	if (mode !== 'auto') return mode;
	if (software) return 'lines';
	return segments <= VOLUME_SEGMENT_LIMIT ? 'volumes' : 'lines';
}

/** Moteurs WebGL logiciels (aucune accélération matérielle). */
const SOFTWARE_RENDERERS = /swiftshader|llvmpipe|softpipe|software|basic render/i;

/** Le nom du moteur WebGL désigne-t-il un rendu logiciel ? */
export function isSoftwareRenderer(renderer: string | null | undefined): boolean {
	return !!renderer && SOFTWARE_RENDERERS.test(renderer);
}

/** Détecte (une fois) un rendu WebGL logiciel dans ce navigateur. */
let softwareGl: boolean | null = null;
export function detectSoftwareRenderer(): boolean {
	if (softwareGl !== null) return softwareGl;
	softwareGl = false;
	try {
		const canvas = document.createElement('canvas');
		const gl = (canvas.getContext('webgl2') ??
			canvas.getContext('webgl')) as WebGLRenderingContext | null;
		if (gl) {
			const info = gl.getExtension('WEBGL_debug_renderer_info');
			const name = info
				? gl.getParameter(info.UNMASKED_RENDERER_WEBGL)
				: gl.getParameter(gl.RENDERER);
			softwareGl = isSoftwareRenderer(String(name ?? ''));
			gl.getExtension('WEBGL_lose_context')?.loseContext();
		}
	} catch {
		// Pas de WebGL : rien à adapter.
	}
	return softwareGl;
}

/**
 * Prisme unité le long de X (−0,5…0,5), section de diamètre 1 dans le plan YZ :
 * flancs à normales radiales (ombrage lisse après mise à l'échelle) et
 * bouchons plats. Non indexé (positions + normales, 3 sommets par triangle).
 */
export function sectionPrism(sides = SECTION_SIDES): {
	positions: Float32Array;
	normals: Float32Array;
} {
	const ring: [number, number][] = [];
	for (let k = 0; k < sides; k++) {
		const a = (2 * Math.PI * k) / sides;
		ring.push([0.5 * Math.cos(a), 0.5 * Math.sin(a)]);
	}
	const pos: number[] = [];
	const nor: number[] = [];
	const v = (x: number, y: number, z: number, nx: number, ny: number, nz: number) => {
		pos.push(x, y, z);
		nor.push(nx, ny, nz);
	};
	for (let k = 0; k < sides; k++) {
		const [y0, z0] = ring[k];
		const [y1, z1] = ring[(k + 1) % sides];
		const n0: [number, number] = [y0 * 2, z0 * 2];
		const n1: [number, number] = [y1 * 2, z1 * 2];
		// Deux triangles du flanc (sens antihoraire vu de l'extérieur).
		v(-0.5, y0, z0, 0, n0[0], n0[1]);
		v(0.5, y0, z0, 0, n0[0], n0[1]);
		v(0.5, y1, z1, 0, n1[0], n1[1]);
		v(-0.5, y0, z0, 0, n0[0], n0[1]);
		v(0.5, y1, z1, 0, n1[0], n1[1]);
		v(-0.5, y1, z1, 0, n1[0], n1[1]);
	}
	for (let k = 1; k < sides - 1; k++) {
		const [ya, za] = ring[0];
		const [yb, zb] = ring[k];
		const [yc, zc] = ring[k + 1];
		v(0.5, ya, za, 1, 0, 0);
		v(0.5, yb, zb, 1, 0, 0);
		v(0.5, yc, zc, 1, 0, 0);
		v(-0.5, ya, za, -1, 0, 0);
		v(-0.5, yc, zc, -1, 0, 0);
		v(-0.5, yb, zb, -1, 0, 0);
	}
	return { positions: Float32Array.from(pos), normals: Float32Array.from(nor) };
}

/**
 * Matrices d'instance (16 flottants, ordre colonnes de three.js) : axe X =
 * segment (allongé d'une demi-largeur pour fermer les jonctions), Y = largeur
 * (horizontale, perpendiculaire), Z = hauteur ; centre abaissé de h/2.
 */
export function instanceMatrices(
	positions: Float32Array,
	sizes: Float32Array,
	count: number
): Float32Array {
	const out = new Float32Array(count * 16);
	for (let i = 0; i < count; i++) {
		const sx = positions[i * 6];
		const sy = positions[i * 6 + 1];
		const sz = positions[i * 6 + 2];
		const dx = positions[i * 6 + 3] - sx;
		const dy = positions[i * 6 + 4] - sy;
		const dz = positions[i * 6 + 5] - sz;
		const w = sizes[i * 2] || 0.4;
		const h = sizes[i * 2 + 1] || 0.2;
		let len = Math.hypot(dx, dy, dz);
		let ax = 1;
		let ay = 0;
		let az = 0;
		if (len > 1e-6) {
			ax = dx / len;
			ay = dy / len;
			az = dz / len;
		} else len = 0;
		// Côté horizontal : Z × axe (repli Y si le segment est vertical).
		let px = -ay;
		let py = ax;
		const pl = Math.hypot(px, py);
		if (pl < 1e-6) {
			px = 0;
			py = 1;
		} else {
			px /= pl;
			py /= pl;
		}
		// Haut = axe × côté.
		const ux = ay * 0 - az * py;
		const uy = az * px - ax * 0;
		const uz = ax * py - ay * px;
		const L = len + w * 0.5;
		const o = i * 16;
		out[o] = ax * L;
		out[o + 1] = ay * L;
		out[o + 2] = az * L;
		out[o + 4] = px * w;
		out[o + 5] = py * w;
		out[o + 6] = 0;
		out[o + 8] = ux * h;
		out[o + 9] = uy * h;
		out[o + 10] = uz * h;
		out[o + 12] = sx + dx / 2 - ux * (h / 2);
		out[o + 13] = sy + dy / 2 - uy * (h / 2);
		out[o + 14] = sz + dz / 2 - uz * (h / 2);
		out[o + 15] = 1;
	}
	return out;
}

/** Couleurs d'instance (une par segment) depuis les couleurs de sommets. */
export function instanceColors(colors: Float32Array, count: number): Float32Array {
	const out = new Float32Array(count * 3);
	for (let i = 0; i < count; i++) {
		out[i * 3] = colors[i * 6];
		out[i * 3 + 1] = colors[i * 6 + 1];
		out[i * 3 + 2] = colors[i * 6 + 2];
	}
	return out;
}
