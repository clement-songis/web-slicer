// Tests des outils de scène purs (T055) : coupe (split par plan, connecteurs)
// et simplification (regroupement de sommets).
import { describe, expect, test } from 'vitest';
import { meshFromTriangleSoup } from '../loaders';
import { signedDistance, splitByPlane, connectorGrid, type CutPlane } from './cut';
import { simplifyGrid, triangleCount } from './simplify';

// Un quad dans le plan XY (z=0), de (-10,-10) à (10,10), en deux triangles.
function quadXY(): ReturnType<typeof meshFromTriangleSoup> {
	return meshFromTriangleSoup([
		-10, -10, 0, 10, -10, 0, 10, 10, 0, -10, -10, 0, 10, 10, 0, -10, 10, 0
	]);
}

describe('signedDistance', () => {
	test('positif du côté de la normale, nul sur le plan', () => {
		const plane: CutPlane = { point: [0, 0, 0], normal: [0, 0, 1] };
		expect(signedDistance(plane, [0, 0, 5])).toBeCloseTo(5);
		expect(signedDistance(plane, [0, 0, -3])).toBeCloseTo(-3);
		expect(signedDistance(plane, [7, 2, 0])).toBeCloseTo(0);
	});
});

describe('splitByPlane', () => {
	test('coupe un quad en deux moitiés non vides', () => {
		// Plan vertical x=0, normale +X.
		const plane: CutPlane = { point: [0, 0, 0], normal: [1, 0, 0] };
		const { above, below } = splitByPlane(quadXY(), plane);
		expect(triangleCount(above)).toBeGreaterThan(0);
		expect(triangleCount(below)).toBeGreaterThan(0);
		// Tous les sommets « above » ont x >= 0 (à epsilon près).
		for (let i = 0; i < above.positions.length; i += 3) {
			expect(above.positions[i]).toBeGreaterThanOrEqual(-1e-6);
		}
		for (let i = 0; i < below.positions.length; i += 3) {
			expect(below.positions[i]).toBeLessThanOrEqual(1e-6);
		}
	});

	test('un plan hors du maillage laisse un côté vide', () => {
		const plane: CutPlane = { point: [100, 0, 0], normal: [1, 0, 0] };
		const { above, below } = splitByPlane(quadXY(), plane);
		expect(triangleCount(above)).toBe(0);
		expect(triangleCount(below)).toBeGreaterThan(0);
	});
});

describe('connectorGrid', () => {
	test("place des connecteurs centrés dans l'emprise", () => {
		const pts = connectorGrid([0, 0, 0], [20, 20, 0], 10, 0);
		expect(pts.length).toBe(4);
		expect(pts[0]).toEqual([5, 5, 0]);
	});

	test('pas nul → aucun connecteur', () => {
		expect(connectorGrid([0, 0, 0], [20, 20, 0], 0, 0)).toEqual([]);
	});
});

describe('simplifyGrid', () => {
	test('supprime les facettes dégénérées après soudure', () => {
		// Triangle minuscule : ses trois sommets tombent dans la même cellule.
		const tiny = meshFromTriangleSoup([0, 0, 0, 0.1, 0, 0, 0, 0.1, 0]);
		expect(triangleCount(simplifyGrid(tiny, 10))).toBe(0);
	});

	test('préserve un triangle plus grand que la cellule', () => {
		const big = meshFromTriangleSoup([0, 0, 0, 100, 0, 0, 0, 100, 0]);
		expect(triangleCount(simplifyGrid(big, 1))).toBe(1);
	});

	test('cell <= 0 renvoie le maillage inchangé', () => {
		const m = quadXY();
		expect(simplifyGrid(m, 0)).toBe(m);
	});
});
