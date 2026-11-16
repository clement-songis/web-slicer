import { describe, expect, it } from 'vitest';
import {
	effectiveRenderMode,
	instanceMatrices,
	sectionPrism,
	VOLUME_SEGMENT_LIMIT
} from './extrusions';

describe('extrusions en volumes', () => {
	it('prisme hexagonal : 20 triangles, normales unitaires', () => {
		const p = sectionPrism();
		expect(p.positions.length / 9).toBe(20);
		for (let i = 0; i < p.normals.length; i += 3)
			expect(Math.hypot(p.normals[i], p.normals[i + 1], p.normals[i + 2])).toBeCloseTo(1, 5);
	});

	it('segment horizontal le long de X : longueur + w/2, largeur, hauteur, centre sous la buse', () => {
		const m = instanceMatrices(
			Float32Array.from([0, 0, 0.2, 10, 0, 0.2]),
			Float32Array.from([0.45, 0.2]),
			1
		);
		expect(m[0]).toBeCloseTo(10.225); // axe X × (L + w/2)
		expect(m[5]).toBeCloseTo(0.45); // largeur sur Y
		expect(m[10]).toBeCloseTo(0.2); // hauteur sur Z
		expect([m[12], m[13], m[14]].map((v) => +v.toFixed(3))).toEqual([5, 0, 0.1]);
	});

	it('segment vertical : base orthonormée de repli', () => {
		const m = instanceMatrices(
			Float32Array.from([0, 0, 0, 0, 0, 1]),
			Float32Array.from([0.4, 0.2]),
			1
		);
		expect(Math.hypot(m[4], m[5], m[6])).toBeCloseTo(0.4);
		expect(Math.hypot(m[8], m[9], m[10])).toBeCloseTo(0.2);
	});

	it('mode auto : volumes sous la limite, lignes au-delà', () => {
		expect(effectiveRenderMode('auto', 1000)).toBe('volumes');
		expect(effectiveRenderMode('auto', VOLUME_SEGMENT_LIMIT + 1)).toBe('lines');
		expect(effectiveRenderMode('lines', 10)).toBe('lines');
	});
});
