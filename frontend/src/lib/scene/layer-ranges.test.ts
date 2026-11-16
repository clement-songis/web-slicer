import { describe, expect, it } from 'vitest';
import {
	addRangeAfter,
	addRootRange,
	parseLayerRanges,
	removeRange,
	setRangeBounds
} from './layer-ranges';

describe('plages de hauteur (layer_config_ranges)', () => {
	it('première plage [0, 2] avec layer_height et extruder 0', () => {
		expect(addRootRange([], 0.2)).toEqual([
			{ min: 0, max: 2, settings: { layer_height: 0.2, extruder: 0 } }
		]);
	});
	it('après la dernière : [max, max + 2]', () => {
		const r = addRootRange(addRootRange([], 0.2), 0.2);
		expect(r.map((x) => [x.min, x.max])).toEqual([
			[0, 2],
			[2, 4]
		]);
	});
	it('comble un trou avant la plage suivante', () => {
		const base = parseLayerRanges([
			{ min: 0, max: 2, settings: {} },
			{ min: 5, max: 6, settings: {} }
		]);
		expect(addRangeAfter(base, 0, 0.2).map((x) => [x.min, x.max])).toEqual([
			[0, 2],
			[2, 5],
			[5, 6]
		]);
	});
	it('scinde la plage suivante contiguë en son milieu', () => {
		const base = parseLayerRanges([
			{ min: 0, max: 2, settings: {} },
			{ min: 2, max: 4, settings: { layer_height: 0.1 } }
		]);
		const out = addRangeAfter(base, 0, 0.2);
		expect(out.map((x) => [x.min, x.max])).toEqual([
			[0, 2],
			[2, 3],
			[3, 4]
		]);
		expect(out[2].settings).toEqual({ layer_height: 0.1 });
	});
	it('refuse des bornes chevauchantes ou vides', () => {
		const base = addRootRange(addRootRange([], 0.2), 0.2);
		expect(setRangeBounds(base, 0, 0, 3)).toEqual(base);
		expect(setRangeBounds(base, 0, 1, 1)).toEqual(base);
		expect(setRangeBounds(base, 0, 0.5, 1.5)[0]).toMatchObject({ min: 0.5, max: 1.5 });
		expect(removeRange(base, 0)).toHaveLength(1);
	});
});
