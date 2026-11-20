import { describe, expect, it } from 'vitest';
import {
	ARRANGE_DEFAULTS,
	clampArrangeSpacing,
	DEFAULT_PREFERENCES,
	parsePreferences
} from './preferences';

describe('préférences lot 34-35 (3MF, arrangement)', () => {
	it('comportement d’ouverture et avertissement G-code : défauts et lecture', () => {
		expect(DEFAULT_PREFERENCES.projectLoadBehaviour).toBe('load_all');
		expect(DEFAULT_PREFERENCES.noWarnWhenModifiedGcodes).toBe(false);
		expect(DEFAULT_PREFERENCES.arrange).toEqual(ARRANGE_DEFAULTS);

		const prefs = parsePreferences(
			JSON.stringify({
				projectLoadBehaviour: 'always_ask',
				noWarnWhenModifiedGcodes: true,
				arrange: { spacing: 12, enableRotation: true, alignToYAxis: true }
			})
		);
		expect(prefs.projectLoadBehaviour).toBe('always_ask');
		expect(prefs.noWarnWhenModifiedGcodes).toBe(true);
		expect(prefs.arrange).toEqual({
			...ARRANGE_DEFAULTS,
			spacing: 12,
			enableRotation: true,
			alignToYAxis: true
		});
	});

	it('valeurs inconnues retombent sur les défauts (lecture tolérante)', () => {
		const prefs = parsePreferences(
			JSON.stringify({ projectLoadBehaviour: 'nimportequoi', arrange: { spacing: 'large' } })
		);
		expect(prefs.projectLoadBehaviour).toBe('load_all');
		expect(prefs.arrange.spacing).toBe(0);
	});

	it('espacement borné 0–100 (slider d’Orca), 0 = auto', () => {
		expect(clampArrangeSpacing(-5)).toBe(0);
		expect(clampArrangeSpacing(0)).toBe(0);
		expect(clampArrangeSpacing(8.456)).toBe(8.46);
		expect(clampArrangeSpacing(120)).toBe(100);
		expect(clampArrangeSpacing(Number.NaN)).toBe(0);
	});
});
