import { describe, expect, it } from 'vitest';
import {
	STEP_DEFAULTS,
	formatTriangles,
	isStep,
	normalizeStepOptions,
	sameStepOptions
} from './step-mesh';

describe('maillage STEP (StepMeshDialog)', () => {
	it('défauts d Orca', () => {
		expect(STEP_DEFAULTS).toEqual({
			linear_deflection: 0.003,
			angle_deflection: 0.5,
			split_compound: false
		});
		expect(normalizeStepOptions({})).toEqual(STEP_DEFAULTS);
	});

	it('bornes et pas des curseurs (0,001–0,1 mm ; 0,01–1 rad)', () => {
		expect(normalizeStepOptions({ linear_deflection: 0, angle_deflection: 5 })).toMatchObject({
			linear_deflection: 0.001,
			angle_deflection: 1
		});
		expect(
			normalizeStepOptions({ linear_deflection: 0.0234, angle_deflection: 0.333 })
		).toMatchObject({
			linear_deflection: 0.023,
			angle_deflection: 0.33
		});
		expect(normalizeStepOptions({ linear_deflection: Number.NaN }).linear_deflection).toBe(0.001);
	});

	it('comparaison, détection, format', () => {
		expect(sameStepOptions(STEP_DEFAULTS, { ...STEP_DEFAULTS })).toBe(true);
		expect(sameStepOptions(STEP_DEFAULTS, { ...STEP_DEFAULTS, split_compound: true })).toBe(false);
		expect(isStep('a.STP')).toBe(true);
		expect(isStep('a.step')).toBe(true);
		expect(isStep('a.stl')).toBe(false);
		expect(formatTriangles(12345)).toMatch(/^12\s?345$/);
	});
});
