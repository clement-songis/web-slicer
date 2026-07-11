// Tests de l'i18n additif (T080, FR-072) : l'anglais est l'identité (clé =
// libellé de parité), le français est un supplément, et la couverture française
// des menus/raccourcis est complète.
import { describe, expect, test } from 'bun:test';
import { get } from 'svelte/store';
import { DICTIONARIES, locale, setLocale, t, translate, tr } from './index';
import { allMenuLabels, MAIN_MENUS } from '$lib/menus/menus';
import { SHORTCUT_GROUPS } from '$lib/menus/shortcuts';

describe('translate', () => {
	test('English is the identity: the key is the parity label', () => {
		expect(translate('en', 'New Project')).toBe('New Project');
		expect(translate('en', 'Anything unmapped')).toBe('Anything unmapped');
	});

	test('French translates known keys', () => {
		expect(translate('fr', 'New Project')).toBe('Nouveau projet');
		expect(translate('fr', 'Undo')).toBe('Annuler');
	});

	test('additive fallback: unknown French key returns the English key', () => {
		expect(translate('fr', 'Totally new label')).toBe('Totally new label');
	});
});

describe('locale store + helpers', () => {
	test('setLocale updates the reactive translator', () => {
		setLocale('fr');
		expect(get(t)('View')).toBe('Vue');
		expect(tr('View')).toBe('Vue');
		setLocale('en');
		expect(get(t)('View')).toBe('View');
		locale.set('en');
	});
});

describe('French coverage (additive completeness)', () => {
	test('every main-menu label and title has a French translation', () => {
		const keys = new Set<string>(MAIN_MENUS.map((m) => m.title));
		for (const label of allMenuLabels()) keys.add(label);
		const missing = [...keys].filter((k) => !(k in DICTIONARIES.fr));
		expect(missing).toEqual([]);
	});

	test('every shortcut group name has a French translation', () => {
		const missing = SHORTCUT_GROUPS.map((g) => g.group).filter((g) => !(g in DICTIONARIES.fr));
		expect(missing).toEqual([]);
	});

	test('no French entry is empty', () => {
		for (const [key, value] of Object.entries(DICTIONARIES.fr)) {
			expect(value.length, `traduction vide pour ${key}`).toBeGreaterThan(0);
		}
	});
});
