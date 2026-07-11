// Primitives paramétriques (menu contextuel « Add », T113) : génèrent une soupe
// de triangles (9 floats/triangle, repère Z-up) assemblée en `SceneMesh` facetté
// par `meshFromTriangleSoup`. Pur → testable sous bun ; posées sur le plateau
// (base à z=0) comme dans OrcaSlicer.
import { meshFromTriangleSoup } from './loaders';
import type { SceneMesh } from './mesh';

/** Type de primitive (libellés de parité `context_menu`). */
export type PrimitiveKind = 'cube' | 'cylinder' | 'sphere' | 'cone' | 'disc' | 'torus';

type Vec3 = [number, number, number];

/** Ajoute un triangle (3 sommets) à la soupe. */
function tri(out: number[], a: Vec3, b: Vec3, c: Vec3): void {
	out.push(a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2]);
}

/** Cube axé, base à z=0, centré en XY (arête `size`). */
export function cube(size = 20): number[] {
	const h = size / 2;
	const v: Vec3[] = [
		[-h, -h, 0],
		[h, -h, 0],
		[h, h, 0],
		[-h, h, 0],
		[-h, -h, size],
		[h, -h, size],
		[h, h, size],
		[-h, h, size]
	];
	const faces = [
		[0, 1, 2, 3], // bas
		[7, 6, 5, 4], // haut
		[0, 4, 5, 1], // avant
		[1, 5, 6, 2], // droite
		[2, 6, 7, 3], // arrière
		[3, 7, 4, 0] // gauche
	];
	const out: number[] = [];
	for (const [a, b, c, d] of faces) {
		tri(out, v[a], v[b], v[c]);
		tri(out, v[a], v[c], v[d]);
	}
	return out;
}

/** Cylindre vertical (base à z=0), `segments` facettes. */
export function cylinder(radius = 10, height = 20, segments = 32): number[] {
	const out: number[] = [];
	const top: Vec3 = [0, 0, height];
	const bottom: Vec3 = [0, 0, 0];
	for (let i = 0; i < segments; i++) {
		const a = (i / segments) * 2 * Math.PI;
		const b = ((i + 1) / segments) * 2 * Math.PI;
		const p0: Vec3 = [radius * Math.cos(a), radius * Math.sin(a), 0];
		const p1: Vec3 = [radius * Math.cos(b), radius * Math.sin(b), 0];
		const p2: Vec3 = [radius * Math.cos(b), radius * Math.sin(b), height];
		const p3: Vec3 = [radius * Math.cos(a), radius * Math.sin(a), height];
		tri(out, p0, p1, p2); // paroi
		tri(out, p0, p2, p3);
		tri(out, bottom, p1, p0); // fond
		tri(out, top, p3, p2); // dessus
	}
	return out;
}

/** Disque = cylindre plat (hauteur faible). */
export function disc(radius = 15, height = 2, segments = 48): number[] {
	return cylinder(radius, height, segments);
}

/** Cône (base à z=0, apex à z=height). */
export function cone(radius = 10, height = 20, segments = 32): number[] {
	const out: number[] = [];
	const apex: Vec3 = [0, 0, height];
	const bottom: Vec3 = [0, 0, 0];
	for (let i = 0; i < segments; i++) {
		const a = (i / segments) * 2 * Math.PI;
		const b = ((i + 1) / segments) * 2 * Math.PI;
		const p0: Vec3 = [radius * Math.cos(a), radius * Math.sin(a), 0];
		const p1: Vec3 = [radius * Math.cos(b), radius * Math.sin(b), 0];
		tri(out, p0, p1, apex); // paroi
		tri(out, bottom, p1, p0); // fond
	}
	return out;
}

/** Sphère UV, centrée à z=radius (posée sur le plateau). */
export function sphere(radius = 10, segments = 24, rings = 16): number[] {
	const out: number[] = [];
	const cz = radius;
	const at = (ring: number, seg: number): Vec3 => {
		const phi = (ring / rings) * Math.PI; // 0..π (pôle à pôle)
		const theta = (seg / segments) * 2 * Math.PI;
		return [
			radius * Math.sin(phi) * Math.cos(theta),
			radius * Math.sin(phi) * Math.sin(theta),
			cz + radius * Math.cos(phi)
		];
	};
	for (let r = 0; r < rings; r++) {
		for (let s = 0; s < segments; s++) {
			const p0 = at(r, s);
			const p1 = at(r + 1, s);
			const p2 = at(r + 1, s + 1);
			const p3 = at(r, s + 1);
			tri(out, p0, p1, p2);
			tri(out, p0, p2, p3);
		}
	}
	return out;
}

/** Tore centré à z=tube (posé sur le plateau). `ring` = rayon majeur, `tube` = mineur. */
export function torus(ring = 12, tube = 4, segments = 32, sides = 16): number[] {
	const out: number[] = [];
	const at = (i: number, j: number): Vec3 => {
		const u = (i / segments) * 2 * Math.PI;
		const v = (j / sides) * 2 * Math.PI;
		const r = ring + tube * Math.cos(v);
		return [r * Math.cos(u), r * Math.sin(u), tube + tube * Math.sin(v)];
	};
	for (let i = 0; i < segments; i++) {
		for (let j = 0; j < sides; j++) {
			const p0 = at(i, j);
			const p1 = at(i + 1, j);
			const p2 = at(i + 1, j + 1);
			const p3 = at(i, j + 1);
			tri(out, p0, p1, p2);
			tri(out, p0, p2, p3);
		}
	}
	return out;
}

/** Génère le maillage facetté d'une primitive (dimensions par défaut OrcaSlicer-ish). */
export function primitiveMesh(kind: PrimitiveKind): SceneMesh {
	const soup =
		kind === 'cube'
			? cube()
			: kind === 'cylinder'
				? cylinder()
				: kind === 'sphere'
					? sphere()
					: kind === 'cone'
						? cone()
						: kind === 'disc'
							? disc()
							: torus();
	return meshFromTriangleSoup(soup);
}
