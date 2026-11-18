import { describe, expect, it } from 'vitest';
import { calibrationReading, kAtHeight } from './calibration-result';

describe('lecture du résultat de calibration', () => {
	it('tour de PA : K = début + pas × hauteur, bornée à la tour', () => {
		const r = calibrationReading({ mode: 'pa_tower', start: 0, end: 0.1, step: 0.002 });
		expect(r).toEqual({ kind: 'pa_tower', start: 0, step: 0.002, maxHeight: 50 });
		if (r?.kind !== 'pa_tower') throw new Error('tour attendue');
		expect(kAtHeight(r, 17)).toBe(0.034);
		expect(kAtHeight(r, 0)).toBe(0);
		expect(kAtHeight(r, 51)).toBeNull();
		expect(kAtHeight(r, Number.NaN)).toBeNull();
	});
	it('tour de température : paliers du bas vers le haut', () => {
		expect(calibrationReading({ mode: 'temp_tower', start: 230, end: 190, step: 5 })).toEqual({
			kind: 'temp_tower',
			temperatures: [230, 225, 220, 215, 210, 205, 200, 195, 190]
		});
	});
	it('lignes / motif : K lu directement ; autres modes ignorés', () => {
		expect(calibrationReading({ mode: 'pa_line', start: 0, end: 0.1, step: 0.002 })?.kind).toBe(
			'pa_direct'
		);
		expect(
			calibrationReading({ mode: 'vol_speed_tower', start: 5, end: 20, step: 0.5 })
		).toBeNull();
		expect(calibrationReading({ mode: 'pa_tower', start: 0, end: 0.1, step: 0 })).toBeNull();
		expect(calibrationReading(null)).toBeNull();
	});
});
