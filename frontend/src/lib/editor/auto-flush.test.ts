import { describe, expect, it } from 'vitest';
import { changedSlots, mergeFlushSlots } from './auto-flush';

const s = (colours: string[], filaments: string[]) => ({ colours, filaments });

describe('purge automatique (Orca auto_calculate_flush)', () => {
	it('slots changés selon le mode', () => {
		const a = s(['#ff0000', '#00ff00'], ['p1', 'p2']);
		expect(changedSlots(a, s(['#FF0000', '#0000ff'], ['p1', 'p2']), 'all')).toEqual([1]);
		expect(changedSlots(a, s(['#ff0000', '#00ff00'], ['p1', 'p3']), 'all')).toEqual([1]);
		expect(changedSlots(a, s(['#ff0000', '#00ff00'], ['p1', 'p3']), 'color')).toEqual([]);
		expect(changedSlots(a, s(['#000000', '#00ff00'], ['p1', 'p2']), 'disabled')).toEqual([]);
		expect(changedSlots(null, a, 'all')).toEqual([]);
		expect(changedSlots(a, s(['#ff0000', '#00ff00', '#fff'], ['p1', 'p2', 'p3']), 'all')).toEqual(
			[]
		);
	});
	it('fusion : lignes/colonnes recalculées, autres valeurs conservées', () => {
		const current = [0, 1, 2, 3, 0, 5, 6, 7, 0];
		const computed = [0, 10, 20, 30, 0, 50, 60, 70, 0];
		expect(mergeFlushSlots(current, computed, 3, [1])).toEqual([0, 10, 2, 30, 0, 50, 6, 70, 0]);
		expect(mergeFlushSlots(undefined, computed, 3, [1])).toEqual(computed);
		expect(mergeFlushSlots(current, [1, 2], 3, [1])).toEqual(current);
	});
});
