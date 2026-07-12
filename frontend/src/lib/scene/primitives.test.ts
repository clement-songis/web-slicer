// Tests des primitives paramétriques (T113).
import { describe, expect, test } from 'vitest';
import { cube, cylinder, cone, sphere, disc, torus, primitiveMesh } from './primitives';

// Boîte englobante d'une soupe de triangles (9 floats/triangle).
function bounds(soup: number[]) {
	const min = [Infinity, Infinity, Infinity];
	const max = [-Infinity, -Infinity, -Infinity];
	for (let i = 0; i < soup.length; i += 3) {
		for (let a = 0; a < 3; a++) {
			min[a] = Math.min(min[a], soup[i + a]);
			max[a] = Math.max(max[a], soup[i + a]);
		}
	}
	return { min, max };
}

describe('primitives', () => {
	test('chaque primitive produit une soupe non vide, multiple de 9', () => {
		for (const soup of [cube(), cylinder(), cone(), sphere(), disc(), torus()]) {
			expect(soup.length).toBeGreaterThan(0);
			expect(soup.length % 9).toBe(0);
		}
	});

	test('cube : base à z=0, arête = size, centré en XY', () => {
		const b = bounds(cube(20));
		expect(b.min).toEqual([-10, -10, 0]);
		expect(b.max).toEqual([10, 10, 20]);
	});

	test('cylindre : rayon et hauteur respectés, base à z=0', () => {
		const b = bounds(cylinder(10, 20, 32));
		expect(b.min[2]).toBeCloseTo(0);
		expect(b.max[2]).toBeCloseTo(20);
		expect(b.max[0]).toBeCloseTo(10, 1);
	});

	test('cône : apex à la hauteur, base à z=0', () => {
		const b = bounds(cone(10, 20, 32));
		expect(b.min[2]).toBeCloseTo(0);
		expect(b.max[2]).toBeCloseTo(20);
	});

	test('sphère : posée sur le plateau (min z ≈ 0)', () => {
		const b = bounds(sphere(10));
		expect(b.min[2]).toBeCloseTo(0, 1);
		expect(b.max[2]).toBeCloseTo(20, 1);
	});

	test('primitiveMesh : assemble un SceneMesh facetté', () => {
		const mesh = primitiveMesh('cube');
		expect(mesh.positions.length).toBe(cube().length);
		expect(mesh.indices.length).toBe(mesh.positions.length / 3);
	});
});
