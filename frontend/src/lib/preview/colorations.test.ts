// Tests des colorations (T068) : fidélité de la palette de rôles, interpolation
// du dégradé de valeurs et bornes.
import { describe, expect, test } from 'bun:test';
import { RANGE_PALETTE, SEGMENT_ROLE_COLORS, rangeColor, sampleRange } from './colorations';

describe('sampleRange', () => {
	test('maps the endpoints to the first and last stop', () => {
		expect(sampleRange(0)).toEqual(RANGE_PALETTE[0]);
		expect(sampleRange(1)).toEqual(RANGE_PALETTE[RANGE_PALETTE.length - 1]);
	});

	test('clamps out-of-range t', () => {
		expect(sampleRange(-5)).toEqual(RANGE_PALETTE[0]);
		expect(sampleRange(5)).toEqual(RANGE_PALETTE[RANGE_PALETTE.length - 1]);
	});

	test('interpolates between two stops', () => {
		// t=0.05 tombe entre l'arrêt 0 et 1 (11 arrêts → pas = 0.1).
		const c = sampleRange(0.05);
		for (let k = 0; k < 3; k++) {
			const mid = (RANGE_PALETTE[0][k] + RANGE_PALETTE[1][k]) / 2;
			expect(c[k]).toBeCloseTo(mid, 5);
		}
	});
});

describe('rangeColor', () => {
	test('degenerate range returns the palette midpoint', () => {
		expect(rangeColor(5, 3, 3)).toEqual(sampleRange(0.5));
	});

	test('maps min and max to the palette ends', () => {
		expect(rangeColor(10, 10, 20)).toEqual(RANGE_PALETTE[0]);
		expect(rangeColor(20, 10, 20)).toEqual(RANGE_PALETTE[RANGE_PALETTE.length - 1]);
	});
});

describe('SEGMENT_ROLE_COLORS', () => {
	test('matches the OrcaSlicer palette for key roles', () => {
		// Outer wall = {255,125,56} (libvgcode DEFAULT_EXTRUSION_ROLES_COLORS).
		expect(SEGMENT_ROLE_COLORS[2]).toEqual([255 / 255, 125 / 255, 56 / 255]);
		// Travel = {56,72,155} (DEFAULT_OPTIONS_COLORS Travels).
		expect(SEGMENT_ROLE_COLORS[20]).toEqual([56 / 255, 72 / 255, 155 / 255]);
	});
});
