import { describe, expect, it } from 'vitest';
import { toolpathsToObj } from './toolpaths-obj';
import type { PreviewSegments } from './decode';

describe('export trajets OBJ', () => {
	it('exclut les déplacements, indices OBJ 1-based', () => {
		const seg = {
			from: 0,
			to: 0,
			count: 2,
			start: new Float32Array([0, 0, 0.2, 1, 0, 0.2]),
			end: new Float32Array([1, 0, 0.2, 5, 5, 0.2]),
			feedrate: new Float32Array(2),
			width: new Float32Array(2),
			height: new Float32Array(2),
			kind: new Uint8Array([1, 20]),
			extruder: new Uint8Array(2),
			layer: new Uint16Array(2)
		} as unknown as PreviewSegments;
		const text = toolpathsToObj([seg]);
		expect(text.match(/^v /gm)).toHaveLength(2);
		expect(text).toContain('l 1 2');
		expect(text).not.toContain('l 3 4');
	});
});
