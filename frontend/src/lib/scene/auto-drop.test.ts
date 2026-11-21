import { describe, expect, it } from 'vitest';
import { autoDrop, objectMinZ } from './auto-drop';
import type { SceneMesh } from './mesh';
import type { Transform } from './transform';

/** Cube 20 mm centré sur l'origine (8 sommets suffisent pour la boîte). */
function cube(size = 20): SceneMesh {
	const h = size / 2;
	const p: number[] = [];
	for (const x of [-h, h]) for (const y of [-h, h]) for (const z of [-h, h]) p.push(x, y, z);
	return {
		positions: new Float32Array(p),
		normals: new Float32Array(p.length),
		indices: new Uint32Array(0)
	} as unknown as SceneMesh;
}
const t = (z: number, scale = 1, rotation: [number, number, number] = [0, 0, 0]): Transform => ({
	position: [100, 100, z],
	rotation,
	scale: [scale, scale, scale]
});

describe('autoDrop (audit 41 V2, GLCanvas3D::do_scale)', () => {
	it('cube posé mis à 150 % : reposé (Z 10 → 15), point bas à 0', () => {
		const next = autoDrop(cube(), t(10), t(10, 1.5));
		expect(next.position[2]).toBeCloseTo(15, 6);
		expect(objectMinZ(cube(), next)).toBeCloseTo(0, 6);
	});

	it('réduction à 50 % : ne flotte pas (Z 10 → 5)', () => {
		expect(autoDrop(cube(), t(10), t(10, 0.5)).position[2]).toBeCloseTo(5, 6);
	});

	it('rotation de 45° autour de X : reposé sur l’arête', () => {
		const next = autoDrop(cube(), t(10), t(10, 1, [45, 0, 0]));
		expect(objectMinZ(cube(), next)).toBeCloseTo(0, 5);
		expect(next.position[2]).toBeCloseTo(10 * Math.SQRT2, 4);
	});

	it('objet volontairement enfoncé et encore enfoncé après : laissé tel quel', () => {
		const sunk = t(4); // point bas à -6
		expect(autoDrop(cube(), sunk, t(4, 1.2)).position[2]).toBe(4);
	});

	it('objet flottant mis à l’échelle : reposé (Orca : flying → plateau)', () => {
		expect(autoDrop(cube(), t(30), t(30, 2)).position[2]).toBeCloseTo(20, 6);
	});

	it('déplacement seul (Z saisi) ou maillage absent : inchangé', () => {
		expect(autoDrop(cube(), t(10), t(3)).position[2]).toBe(3);
		expect(autoDrop(null, t(10), t(10, 2)).position[2]).toBe(10);
	});
});
