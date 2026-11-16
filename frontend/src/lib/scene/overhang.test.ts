import { describe, expect, it } from 'vitest';
import { overhangIndices, supportNormalZ } from './overhang';
import type { SceneMesh } from './mesh';

// Cube 10 mm centré : faces bas (-Z) et haut (+Z) + côtés.
function cube(): SceneMesh {
	const v: number[] = [];
	for (let i = 0; i < 8; i++) v.push(i & 1 ? 5 : -5, i & 2 ? 5 : -5, i & 4 ? 5 : -5);
	const faces = [
		[0, 2, 3, 1],
		[4, 5, 7, 6],
		[0, 1, 5, 4],
		[2, 6, 7, 3],
		[0, 4, 6, 2],
		[1, 3, 7, 5]
	];
	const idx: number[] = [];
	for (const [a, b, c, d] of faces) idx.push(a, b, c, a, c, d);
	return {
		positions: new Float32Array(v),
		normals: new Float32Array(0),
		indices: new Uint32Array(idx)
	};
}

describe('surplombs (seuil Orca)', () => {
	it('seuil explicite : angle + 1, inclusif', () => {
		expect(supportNormalZ({ support_threshold_angle: 30 })).toBeCloseTo(
			-Math.cos((31 * Math.PI) / 180)
		);
		expect(supportNormalZ({ support_threshold_angle: 0, support_type: 'tree(auto)' })).toBeCloseTo(
			-Math.cos(Math.PI / 6)
		);
	});
	it('seuil dérivé du recouvrement (supports normaux, angle 0)', () => {
		const z = supportNormalZ({
			support_threshold_angle: 0,
			support_type: 'normal(auto)',
			layer_height: 0.2,
			nozzle_diameter: [0.4],
			outer_wall_line_width: 0.4,
			support_threshold_overlap: '50%'
		});
		expect(z).toBeCloseTo(-Math.cos(Math.atan(0.2 / 0.2)));
	});
	it('fond en l’air = surplomb ; posé sur le plateau = exclu', () => {
		const nz = supportNormalZ({ support_threshold_angle: 30 });
		expect(overhangIndices(cube(), nz, [0, 0, 0], [1, 1, 1], [0, 0, 20]).length).toBe(6);
		expect(overhangIndices(cube(), nz, [0, 0, 0], [1, 1, 1], [0, 0, 5]).length).toBe(0);
		// Retourné (rotation X 180°) : la face « haut » devient le dessous.
		expect(overhangIndices(cube(), nz, [180, 0, 0], [1, 1, 1], [0, 0, 20]).length).toBe(6);
	});
});
