import { describe, expect, it } from 'vitest';
import { squareCrop } from './plate-thumbnail';

describe('vignette de plateau', () => {
	it('carré centré du canvas', () => {
		expect(squareCrop(1600, 900)).toEqual({ sx: 350, sy: 0, side: 900 });
		expect(squareCrop(800, 1000)).toEqual({ sx: 0, sy: 100, side: 800 });
	});
});
