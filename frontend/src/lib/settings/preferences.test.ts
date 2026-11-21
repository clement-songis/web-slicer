import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { setLocale } from '$lib/i18n';
import { DEFAULT_PREFERENCES, formatLength, formatMoney, parsePreferences } from './preferences';

// Montants au format de la langue (audit 40 A4) : valeurs attendues en français.
beforeAll(() => setLocale('fr'));
afterAll(() => setLocale('en'));

describe('préférences', () => {
	it('valeurs par défaut et lecture tolérante', () => {
		expect(parsePreferences(null)).toEqual(DEFAULT_PREFERENCES);
		expect(parsePreferences('{oops')).toEqual(DEFAULT_PREFERENCES);
		expect(parsePreferences('{"units":"in","confirmClose":false}')).toEqual({
			...DEFAULT_PREFERENCES,
			units: 'in',
			confirmClose: false
		});
		expect(parsePreferences('{"units":"km"}').units).toBe('mm');
	});
	it('devise des coûts', () => {
		expect(parsePreferences('{"currency":" CHF "}').currency).toBe('CHF');
		expect(parsePreferences('{"currency":""}').currency).toBe('€');
		expect(formatMoney(0.08, '€')).toBe('0,08 €');
	});
	it('panneau latéral replié persistant', () => {
		expect(parsePreferences('{"sidebarCollapsed":true}').sidebarCollapsed).toBe(true);
		expect(parsePreferences('{"sidebarCollapsed":"oui"}').sidebarCollapsed).toBe(false);
	});
	it('unités d’affichage', () => {
		expect(formatLength(25.4, 'in')).toBe('1.00 in');
		expect(formatLength(12.5, 'mm', 1)).toBe('12.5 mm');
	});
});
