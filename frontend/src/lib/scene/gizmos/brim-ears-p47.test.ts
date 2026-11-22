// Audit 45 EARS-a/b — oreilles de bordure (GLGizmoBrimEars d'Orca).
import { describe, expect, test } from 'vitest';
import {
	BrimEars,
	LEGACY_HEAD_RADIUS,
	autoGeneratePoints,
	defaultHeadDiameter,
	detectionRadiusMax,
	douglasPeucker,
	earToWorld,
	firstLayerContours,
	invalidEars,
	worldToEar
} from './brim-ears';

const T = {
	position: [100, 50, 10] as [number, number, number],
	rotation: [0, 0, 90] as [number, number, number],
	scale: [2, 1, 1] as [number, number, number]
};

/** Cube 20 mm recentré (12 triangles). */
function cube(h = 10) {
	const v = [
		[-h, -h, -h],
		[h, -h, -h],
		[h, h, -h],
		[-h, h, -h],
		[-h, -h, h],
		[h, -h, h],
		[h, h, h],
		[-h, h, h]
	];
	const f = [
		[0, 2, 1],
		[0, 3, 2],
		[4, 5, 6],
		[4, 6, 7],
		[0, 1, 5],
		[0, 5, 4],
		[1, 2, 6],
		[1, 6, 5],
		[2, 3, 7],
		[2, 7, 6],
		[3, 0, 4],
		[3, 4, 7]
	];
	return { positions: new Float32Array(v.flat()), indices: new Uint32Array(f.flat()) };
}

describe('BrimEars (repère objet)', () => {
	test('liste = nouvelle copie à chaque appel (toutes les oreilles affichées)', () => {
		const ears = new BrimEars();
		ears.add(1, 2, 0, 3);
		const a = ears.list();
		ears.add(4, 5, 0, 3);
		ears.add(6, 7, 0, 3);
		expect(a).toHaveLength(1);
		expect(ears.list()).toHaveLength(3);
		expect(ears.add(6, 7, 0, 3)).toBeNull();
	});
	test('monde ↔ objet, aller-retour', () => {
		const w = earToWorld({ x: 3, y: -4, z: 0 }, T);
		const l = worldToEar(w, T);
		expect(l[0]).toBeCloseTo(3);
		expect(l[1]).toBeCloseTo(-4);
	});
	test('document : ears (objet + rayon) et points (plateau) ; ancien document repris', () => {
		const ears = new BrimEars();
		ears.add(5, 0, -10, 4.5);
		const doc = ears.serialize(T);
		expect(doc.ears).toEqual([[5, 0, -10, 4.5]]);
		expect(doc.points[0][0]).toBeCloseTo(100);
		expect(doc.points[0][1]).toBeCloseTo(60);
		const back = BrimEars.deserialize(JSON.parse(JSON.stringify(doc)), T);
		expect(back.list()).toEqual([{ x: 5, y: 0, z: -10, r: 4.5 }]);
		expect(doc.points[0][2]).toBe(9);
		const legacy = BrimEars.deserialize({ points: [[100, 60]] }, T);
		expect(legacy.list()[0].x).toBeCloseTo(5);
		expect(legacy.list()[0].r).toBe(LEGACY_HEAD_RADIUS);
		// Import 3MF (serveur) : [x, y, diamètre] plateau.
		const imported = BrimEars.deserialize({ points: [[100, 60, 7]] }, T);
		expect(imported.list()[0].r).toBe(3.5);
	});
	test('removeMany / setRadius', () => {
		const ears = new BrimEars();
		for (let i = 0; i < 4; i++) ears.add(i, 0, 0, 2);
		ears.removeMany([1, 3]);
		ears.setRadius([0], 5);
		expect(ears.list().map((p) => [p.x, p.r])).toEqual([
			[0, 5],
			[2, 2]
		]);
	});
});

describe('géométrie', () => {
	test('diamètre de tête par défaut = largeur 1ʳᵉ couche × 16', () => {
		expect(defaultHeadDiameter('0.5', '0.4')).toBe(8);
		expect(defaultHeadDiameter('125%', '0.4')).toBe(8);
	});
	test('1ʳᵉ couche d’un cube posé : un carré de 20 mm', () => {
		const { positions, indices } = cube();
		const c = firstLayerContours(positions, indices, {
			position: [50, 50, 10],
			rotation: [0, 0, 0],
			scale: [1, 1, 1]
		});
		expect(c).toHaveLength(1);
		const xs = c[0].map((p) => p[0]);
		expect(Math.min(...xs)).toBeCloseTo(40);
		expect(Math.max(...xs)).toBeCloseTo(60);
	});
	test('auto-génération : 4 coins du carré (angle 90° > seuil 180 − 125)', () => {
		const square: [number, number][] = [
			[0, 0],
			[10, 0],
			[20, 0],
			[20, 20],
			[0, 20]
		];
		const pts = autoGeneratePoints([square], 1, 125);
		expect(pts).toHaveLength(4);
		expect(autoGeneratePoints([square], 1, 30)).toHaveLength(0);
	});
	test('Douglas-Peucker retire les points alignés', () => {
		expect(
			douglasPeucker(
				[
					[0, 0],
					[5, 0.1],
					[10, 0]
				],
				1
			)
		).toEqual([
			[0, 0],
			[10, 0]
		]);
	});
	test('oreilles invalides : hors de la couche, sauf chaînées à une oreille valide', () => {
		const square: [number, number][] = [
			[0, 0],
			[20, 0],
			[20, 20],
			[0, 20]
		];
		const ears = [
			{ center: [0, 0] as [number, number], r: 4 },
			{ center: [-7, 0] as [number, number], r: 4 },
			{ center: [-40, -40] as [number, number], r: 4 }
		];
		expect(invalidEars(ears, [square])).toEqual([2]);
	});
	test('rayon de détection max borné à 100', () => {
		const square: [number, number][] = [
			[0, 0],
			[20, 0],
			[20, 20],
			[0, 20]
		];
		const max = detectionRadiusMax([square]);
		expect(max).toBeGreaterThan(0);
		expect(max).toBeLessThanOrEqual(100);
	});
});
