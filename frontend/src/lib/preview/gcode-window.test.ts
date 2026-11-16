import { describe, expect, it } from 'vitest';
import { gcodeWindowFor, layerStartLines } from './gcode-window';

const lines = [
	'G28',
	'; CHANGE_LAYER',
	'G1 X1',
	';LAYER_CHANGE',
	'G1 X2',
	'G1 X3',
	'; CHANGE_LAYER',
	'G1 X4'
];

describe('fenêtre G-code', () => {
	it('repère les marqueurs Bambu et non Bambu', () => {
		expect(layerStartLines(lines)).toEqual([1, 3, 6]);
	});
	it('fenêtre synchronisée avec la couche', () => {
		const starts = layerStartLines(lines);
		const w = gcodeWindowFor(lines, starts, 1, 3, 0);
		expect(w.current).toBe(3);
		expect(w.lines).toEqual([';LAYER_CHANGE', 'G1 X2', 'G1 X3']);
		expect(gcodeWindowFor(lines, starts, 99, 3, 0).current).toBe(6);
		expect(gcodeWindowFor([], [], 0).lines).toEqual([]);
	});
});
