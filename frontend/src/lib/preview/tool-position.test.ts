import { describe, expect, it } from 'vitest';
import type { PreviewSegments } from './decode';
import type { PreviewTrack } from './track';
import { toolPositionAt } from './tool-position';

/** Trois segments : couche 0 (1 segment), couche 1 (2 segments). */
function segs(): PreviewSegments {
	return {
		from: 0,
		to: 1,
		count: 3,
		start: Float32Array.from([0, 0, 0.2, 10, 0, 0.4, 10, 10, 0.4]),
		end: Float32Array.from([10, 0, 0.2, 10, 10, 0.4, 0, 10, 0.4]),
		feedrate: Float32Array.from([1200, 3000, 6000]),
		width: Float32Array.from([0.45, 0.42, 0.5]),
		height: Float32Array.from([0.2, 0.2, 0.2]),
		kind: Uint8Array.from([2, 2, 4]),
		extruder: Uint8Array.from([0, 0, 1]),
		layer: Uint16Array.from([0, 1, 1])
	};
}

describe('toolPositionAt', () => {
	it('fin du dernier segment affiché de la couche, grandeurs du segment', () => {
		const t = toolPositionAt([segs()], 1, 1)!;
		expect(t.index).toBe(0);
		expect(t.position).toEqual([10, 10, expect.closeTo(0.4, 5)]);
		expect(t.kind).toBe(2);
		expect(t.speed).toBe(50);
		expect(t.flow).toBeCloseTo(0.42 * 0.2 * 50, 6);
		const end = toolPositionAt([segs()], 1, 2)!;
		expect(end.position.slice(0, 2)).toEqual([0, 10]);
		expect(end.extruder).toBe(1);
	});
	it('position 0 : départ de la couche, sans grandeurs', () => {
		const t = toolPositionAt([segs()], 1, 0)!;
		expect(t.index).toBe(-1);
		expect(t.position.slice(0, 2)).toEqual([10, 0]);
		expect(t.kind).toBeNull();
		expect(t.speed).toBeNull();
	});
	it('couche absente → null', () => {
		expect(toolPositionAt([segs()], 5, 3)).toBeNull();
	});
	it('lit les grandeurs de la piste alignée (ventilateur, température)', () => {
		const layer = {
			z: 0.4,
			segments: 2,
			time: 12.5,
			values: {
				fan: Float32Array.from([30, 80]),
				temperature: Float32Array.from([210, 215]),
				acceleration: Float32Array.from([NaN, NaN]),
				jerk: new Float32Array(0),
				pa: Float32Array.from([0.02, 0.03]),
				actualSpeed: new Float32Array(0),
				actualFlow: new Float32Array(0)
			}
		};
		const track = {
			layers: [{ segments: 1, values: {}, time: 1 }, layer]
		} as unknown as PreviewTrack;
		const t = toolPositionAt([segs()], 1, 2, track)!;
		expect(t.channels).toEqual({ fan: 80, temperature: 215, pa: expect.closeTo(0.03, 6) });
		expect(t.layerTime).toBe(12.5);
	});
});
