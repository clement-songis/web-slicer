import { describe, expect, it } from 'vitest';
import {
	STEP_DEFAULTS,
	STEP_PREFS_KEY,
	STEP_PRESETS,
	loadStepPrefs,
	parseDeflection,
	presetOf,
	saveStepPrefs,
	triangleDelta
} from './step-mesh';

const memory = () => {
	const m = new Map<string, string>();
	return {
		getItem: (k: string) => m.get(k) ?? null,
		setItem: (k: string, v: string) => void m.set(k, v),
		m
	};
};

describe('maillage STEP : interface (phase 21)', () => {
	it('préréglages : « Fin » = défauts d Orca, valeurs dans les bornes des curseurs', () => {
		expect(presetOf(STEP_DEFAULTS)).toBe('fine');
		for (const p of STEP_PRESETS) {
			expect(presetOf(p)).toBe(p.id);
			expect(p.linear_deflection).toBeGreaterThanOrEqual(0.001);
			expect(p.linear_deflection).toBeLessThanOrEqual(0.1);
			expect(p.angle_deflection).toBeLessThanOrEqual(1);
		}
		expect(presetOf({ linear_deflection: 0.02, angle_deflection: 0.5 })).toBeNull();
	});

	it('écart de triangles', () => {
		expect(triangleDelta(100, 220)).toBe('+120 %');
		expect(triangleDelta(200, 130)).toBe('−35 %');
		expect(triangleDelta(100, 100)).toBe('=');
		expect(triangleDelta(null, 100)).toBeNull();
	});

	it('saisie : virgule acceptée, bornes d Orca', () => {
		expect(parseDeflection('0,05', 'linear')).toEqual({ value: 0.05 });
		expect(parseDeflection('0.2', 'linear')).toHaveProperty('error');
		expect(parseDeflection('0', 'linear')).toHaveProperty('error');
		expect(parseDeflection('1', 'angle')).toEqual({ value: 1 });
		expect(parseDeflection('abc', 'angle')).toHaveProperty('error');
	});

	it('préférences mémorisées (app_config d Orca) : défauts, aller-retour, bornage', () => {
		const kv = memory();
		expect(loadStepPrefs(kv)).toEqual({ options: STEP_DEFAULTS, ask: true });
		saveStepPrefs(
			{
				options: { linear_deflection: 0.05, angle_deflection: 1, split_compound: true },
				ask: false
			},
			kv
		);
		expect(loadStepPrefs(kv)).toEqual({
			options: { linear_deflection: 0.05, angle_deflection: 1, split_compound: true },
			ask: false
		});
		kv.m.set(STEP_PREFS_KEY, JSON.stringify({ options: { linear_deflection: 9 } }));
		expect(loadStepPrefs(kv).options.linear_deflection).toBe(0.1);
		kv.m.set(STEP_PREFS_KEY, '{oops');
		expect(loadStepPrefs(kv).ask).toBe(true);
		expect(loadStepPrefs(null).options).toEqual(STEP_DEFAULTS);
	});
});
