import { describe, expect, it } from 'vitest';
import { parseGcodeFooterStats, parseOrcaDuration } from './footer-stats';

describe('parseOrcaDuration', () => {
	it('lit les durées Orca', () => {
		expect(parseOrcaDuration('1d 2h 3m 4s')).toBe(93784);
		expect(parseOrcaDuration('8m 52s')).toBe(532);
		expect(parseOrcaDuration('n/a')).toBeNull();
	});
});

describe('parseGcodeFooterStats', () => {
	it('lit le pied de G-code Orca (mêmes clés que le serveur)', () => {
		const g = [
			'; HEADER_BLOCK_START',
			'; total layer number: 50',
			'G1 X1 E1',
			'; filament used [mm] = 1234.56, 10',
			'; filament used [cm3] = 2.97, 0.02',
			'; filament used [g] = 3.68, 0.03',
			'; filament cost = 0.07, 0.01',
			'; total filament used [g] = 3.71',
			'; total filament cost = 0.08',
			'; total filament change = 2',
			'; total layers count = 50',
			'; estimated printing time (normal mode) = 1h 2m 3s'
		].join('\n');
		expect(parseGcodeFooterStats(g)).toEqual({
			filament_used_mm: [1234.56, 10],
			filament_used_cm3: [2.97, 0.02],
			filament_used_g: [3.68, 0.03],
			filament_cost: [0.07, 0.01],
			total_filament_weight_g: 3.71,
			total_filament_cost: 0.08,
			total_toolchanges: 2,
			layer_count: 50,
			estimated_time_seconds: 3723,
			estimated_time_text: '1h 2m 3s'
		});
	});
	it('totaux déduits des listes ; null sans clé connue', () => {
		const s = parseGcodeFooterStats('; filament used [g] = 1, 2\n; filament cost = 0.5, 0.25');
		expect(s?.total_filament_weight_g).toBe(3);
		expect(s?.total_filament_cost).toBe(0.75);
		expect(parseGcodeFooterStats('G28\nG1 X1')).toBeNull();
	});
});
