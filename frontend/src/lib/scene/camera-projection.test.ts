import { describe, expect, it } from 'vitest';
import { cameraSpaceNudge, orthoZoom } from './camera';
import type { BedShape } from './bed';

describe('caméra orthogonale', () => {
	it('le zoom diminue avec la taille du plateau', () => {
		const bed = (w: number) => ({ width: w, depth: w }) as BedShape;
		expect(orthoZoom(bed(250))).toBeGreaterThan(orthoZoom(bed(350)));
		expect(orthoZoom(bed(0))).toBeGreaterThan(0);
	});
});

describe('Ctrl+flèches dans le repère caméra', () => {
	it('projette les axes caméra sur XY, avec pas fin Maj', () => {
		const [x, y] = cameraSpaceNudge('default', 'ArrowUp')!;
		expect(x).toBeGreaterThan(0);
		expect(y).toBeGreaterThan(0);
		expect(Math.hypot(x, y)).toBeLessThan(10);
		const fine = cameraSpaceNudge('default', 'ArrowUp', true)!;
		expect(fine[0]).toBeCloseTo(x / 10, 12);
		expect(fine[1]).toBeCloseTo(y / 10, 12);
		expect(cameraSpaceNudge('top', 'ArrowRight')).toEqual([10, 0]);
		expect(cameraSpaceNudge('front', 'ArrowUp')).toEqual([0, 0]);
		expect(cameraSpaceNudge('default', 'a')).toBeNull();
	});
});
