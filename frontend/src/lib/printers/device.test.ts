import { describe, expect, it } from 'vitest';
import {
	fanGcode,
	formatClock,
	homeGcode,
	moveGcode,
	remainingSeconds,
	temperatureGcode
} from './device';

describe('onglet Appareil', () => {
	it('temps restant extrapolé', () => {
		expect(remainingSeconds(0.25, 600)).toBe(1800);
		expect(remainingSeconds(0, 600)).toBeNull();
		expect(remainingSeconds(1, 600)).toBeNull();
		expect(formatClock(3900)).toBe('1 h 05 min');
		expect(formatClock(252)).toBe('4 min 12 s');
		expect(formatClock(null)).toBe('—');
	});
	it('scripts G-code des contrôles', () => {
		expect(moveGcode('X', 10)).toBe('G91\nG1 X10 F6000\nG90');
		expect(moveGcode('Z', -1)).toBe('G91\nG1 Z-1 F600\nG90');
		expect(homeGcode()).toBe('G28');
		expect(homeGcode('Z')).toBe('G28 Z');
		expect(temperatureGcode('extruder', 215.4)).toBe('M104 S215');
		expect(temperatureGcode('bed', 400)).toBe('M140 S150');
		expect(fanGcode(0)).toBe('M107');
		expect(fanGcode(50)).toBe('M106 S128');
	});
});
