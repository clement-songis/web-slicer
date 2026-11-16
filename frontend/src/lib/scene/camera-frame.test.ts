import { describe, expect, it } from 'vitest';
import { boundsOf, frameBed, frameBox } from './camera';

describe("frameBox / boundsOf (cadrage de l'aperçu sur les objets)", () => {
	it('boîte englobante des sommets', () => {
		expect(boundsOf(Float32Array.from([1, 2, 3, -1, 5, 0]))).toEqual({
			min: [-1, 2, 0],
			max: [1, 5, 3]
		});
		expect(boundsOf(new Float32Array(0))).toBeNull();
	});
	it('cible le centre de la boîte, plus près que le cadrage plateau', () => {
		const box = frameBox([165, 165, 0], [185, 185, 20]);
		expect(box.target).toEqual([175, 175, 10]);
		const bed = frameBed({
			width: 350,
			depth: 350,
			height: 340,
			center: { x: 175, y: 175 }
		} as never);
		const dist = (p: { position: number[]; target: number[] }) =>
			Math.hypot(...p.position.map((v, i) => v - p.target[i]));
		expect(dist(box)).toBeLessThan(dist(bed) / 5);
	});
});
