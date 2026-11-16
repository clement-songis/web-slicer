import { describe, expect, test } from 'vitest';
import { StrokeRecorder } from './stroke';

const pick = (x: number, triangle: number | null = 0) => ({
	point: [x, 0, 0] as [number, number, number],
	localPoint: [x, 0, 0] as [number, number, number],
	triangle,
	localCamera: [0, 0, 100] as [number, number, number]
});

describe('StrokeRecorder', () => {
	test('échantillonne en ignorant les points trop proches et les ratés', () => {
		const r = new StrokeRecorder(0.5);
		expect(r.add(pick(0))).toBe(true);
		expect(r.add(pick(0.2))).toBe(false);
		expect(r.add(pick(1, null))).toBe(false);
		expect(r.add(pick(1, 3))).toBe(true);
		expect(r.facets).toEqual([0, 3]);
		expect(r.camera).toEqual([0, 0, 100]);
	});

	test('lots chevauchants pour les longs traits', () => {
		const r = new StrokeRecorder(0);
		for (let i = 0; i < 10; i++) r.add(pick(i));
		const b = r.batches(4);
		expect(b.map((x) => x.points.length)).toEqual([4, 4, 4]);
		expect(b[1].points[0]).toEqual(b[0].points[3]);
		expect(b[2].points[3]).toEqual([9, 0, 0]);
	});
});
