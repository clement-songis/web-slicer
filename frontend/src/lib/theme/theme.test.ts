// Tests du système de thème (T093) — logique pure : résolution de préférence et
// normalisation de la valeur stockée. La bascule DOM (`setTheme`/`initTheme`) est
// couverte par les tests d'intégration navigateur.
import { describe, expect, it } from 'vitest';
import { resolveTheme, parseThemePref, THEME_PREFS } from './theme';

describe('resolveTheme', () => {
	it('rend le thème explicite tel quel, sans regarder l’OS', () => {
		expect(resolveTheme('light', true)).toBe('light');
		expect(resolveTheme('light', false)).toBe('light');
		expect(resolveTheme('dark', false)).toBe('dark');
		expect(resolveTheme('dark', true)).toBe('dark');
	});

	it('suit l’OS quand la préférence est `system`', () => {
		expect(resolveTheme('system', true)).toBe('dark');
		expect(resolveTheme('system', false)).toBe('light');
	});
});

describe('parseThemePref', () => {
	it('accepte les préférences valides', () => {
		for (const pref of THEME_PREFS) {
			expect(parseThemePref(pref)).toBe(pref);
		}
	});

	it('retombe sur `system` pour toute valeur invalide', () => {
		expect(parseThemePref(null)).toBe('system');
		expect(parseThemePref('')).toBe('system');
		expect(parseThemePref('auto')).toBe('system');
		expect(parseThemePref(42)).toBe('system');
		expect(parseThemePref(undefined)).toBe('system');
	});
});
