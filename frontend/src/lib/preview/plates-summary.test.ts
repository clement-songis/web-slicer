import { describe, expect, it } from 'vitest';
import { platesTotals } from './plates-summary';

describe('platesTotals', () => {
	it('additionne temps, grammes et coût des plateaux tranchés', () => {
		const stats: Record<string, unknown> = {
			a: { estimated_time_seconds: 600, total_filament_weight_g: 3.5, total_filament_cost: 0.1 },
			b: { estimated_time_seconds: 300, filament_used_g: [1, 2], filament_cost: [0.2] },
			c: null
		};
		expect(platesTotals(['a', 'b', 'c'], (id) => stats[id])).toEqual({
			plates: 2,
			timeSeconds: 900,
			grams: 6.5,
			cost: expect.closeTo(0.3, 6)
		});
	});
	it('aucun plateau tranché → null', () => {
		expect(platesTotals(['x'], () => undefined)).toBeNull();
	});
});
