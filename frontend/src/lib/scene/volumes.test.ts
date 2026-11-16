import { describe, expect, it } from 'vitest';
import { applyVolumeTransform, defaultVolume, parseVolumes, volumeTransform } from './volumes';

describe('transformation des volumes', () => {
	const base = defaultVolume('modifier', [20, 20, 20], 'v1', 'Modificateur 1');
	it('expose position/rotation et une échelle relative', () => {
		expect(volumeTransform(base)).toEqual({
			position: [0, 0, 0],
			rotation: [0, 0, 0],
			scale: [1, 1, 1]
		});
	});
	it('applique position, rotation et échelle (taille × facteur)', () => {
		const next = applyVolumeTransform(base, {
			position: [1, 2, 3],
			rotation: [0, 0, 45],
			scale: [2, 1, 0.5]
		});
		expect(next.offset).toEqual([1, 2, 3]);
		expect(next.rotation).toEqual([0, 0, 45]);
		expect(next.size).toEqual([20, 10, 5]);
		expect(volumeTransform(next).scale).toEqual([1, 1, 1]);
	});
	it('une rotation nulle disparaît du document et survit au parse sinon', () => {
		const flat = applyVolumeTransform(
			{ ...base, rotation: [0, 0, 30] },
			{ ...volumeTransform(base), rotation: [0, 0, 0] }
		);
		expect('rotation' in flat).toBe(false);
		const parsed = parseVolumes([{ ...base, rotation: [10, 0, 0] }]);
		expect(parsed[0].rotation).toEqual([10, 0, 0]);
		expect('rotation' in parseVolumes([base])[0]).toBe(false);
	});
});

describe('volumes maillés (Split to parts)', () => {
	const mesh = {
		id: 'p2',
		type: 'part',
		name: 'pair_2',
		shape: 'mesh',
		modelId: 'm2',
		size: [10, 10, 10],
		offset: [20, 0, 0],
		settings: {}
	};
	it('conserve forme et modèle à la relecture', () => {
		expect(parseVolumes([mesh])[0]).toMatchObject({
			shape: 'mesh',
			modelId: 'm2',
			offset: [20, 0, 0]
		});
	});
	it('écarte un volume maillé sans modèle', () => {
		expect(parseVolumes([{ ...mesh, modelId: undefined }])).toEqual([]);
	});
	it("ignore l'échelle (pas de taille paramétrique)", () => {
		const [v] = parseVolumes([mesh]);
		const out = applyVolumeTransform(v, {
			position: [21, 0, 0],
			rotation: [0, 0, 0],
			scale: [2, 2, 2]
		});
		expect(out.size).toEqual([10, 10, 10]);
		expect(out.offset).toEqual([21, 0, 0]);
	});
});
