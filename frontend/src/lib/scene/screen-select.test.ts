import { describe, expect, test } from 'vitest';
import {
	pointInBox,
	rectFromDrag,
	rectHits,
	rectsIntersect,
	sectionNormal,
	sectionPlane,
	selectByRect,
	snapPosition,
	stepSectionRatio
} from './screen-select';

describe('rectangle de sélection (Maj+glisser)', () => {
	test('rectangle normalisé quel que soit le sens du glisser', () => {
		expect(rectFromDrag([50, 80], [10, 20])).toEqual({ x0: 10, y0: 20, x1: 50, y1: 80 });
	});

	test('objets dont la projection intersecte le rectangle', () => {
		const boxes = {
			a: { x0: 0, y0: 0, x1: 10, y1: 10 },
			b: { x0: 100, y0: 100, x1: 120, y1: 120 },
			c: { x0: 9, y0: 50, x1: 30, y1: 60 }
		};
		expect(rectHits({ x0: 5, y0: 5, x1: 20, y1: 55 }, boxes)).toEqual(['a', 'c']);
		expect(rectHits({ x0: 200, y0: 200, x1: 300, y1: 300 }, boxes)).toEqual([]);
		expect(rectsIntersect({ x0: 0, y0: 0, x1: 1, y1: 1 }, { x0: 1, y0: 1, x1: 2, y1: 2 })).toBe(
			true
		);
	});

	test('Maj+rectangle ajoute à la sélection existante', () => {
		expect([...selectByRect(new Set(['x']), ['a', 'x'])].sort()).toEqual(['a', 'x']);
	});

	test('point dans une emprise avec marge', () => {
		const box = { x0: 10, y0: 10, x1: 20, y1: 20 };
		expect(pointInBox([5, 15], box)).toBe(false);
		expect(pointInBox([5, 15], box, 6)).toBe(true);
	});
});

describe('Maj pendant le glisser du gizmo Move', () => {
	test('arrondi au millimètre', () => {
		expect(snapPosition([10.4, -2.6, 0.49])).toEqual([10, -3, 0]);
		expect(Object.is(snapPosition([-0.2, 0, 0])[0], -0)).toBe(false);
	});
});

describe('Alt+molette : plan de coupe des gizmos de peinture', () => {
	test('pas de 0.01, borné à [0, 1]', () => {
		expect(stepSectionRatio(0, 1)).toBe(0.01);
		expect(stepSectionRatio(0, -1)).toBe(0);
		expect(stepSectionRatio(1, 1)).toBe(1);
		expect(stepSectionRatio(0.29, 1)).toBe(0.3);
	});

	test('normale = opposée de la direction de vue (vers la caméra)', () => {
		expect(sectionNormal([0, 0, -2])).toEqual([0, 0, 1]);
		const n = sectionNormal([0.6, 0.8, -0.7]);
		expect(Math.hypot(...n)).toBeCloseTo(1, 12);
		expect(n[2]).toBeGreaterThan(0);
	});

	test('plan : inactif à 0, balaie la sphère englobante côté caméra', () => {
		const n: [number, number, number] = [0, 0, 1];
		expect(sectionPlane(n, [5, 5, 10], 10, 0)).toBeNull();
		// Ratio 0.25 : on garde z <= 10 + 10 − 0.25 × 20 = 15.
		expect(sectionPlane(n, [5, 5, 10], 10, 0.25)).toEqual({ normal: n, offset: 15 });
		expect(sectionPlane(n, [5, 5, 10], 10, 1)!.offset).toBe(0);
		// Vue de face (normale −Y) : offset relatif au centre projeté.
		expect(sectionPlane([0, -1, 0], [0, 4, 0], 2, 0.5)!.offset).toBe(-4);
	});
});
