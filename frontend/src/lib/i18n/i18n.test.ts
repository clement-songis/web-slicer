// Tests de l'i18n additif (T080, FR-072) adossé à Paraglide : l'anglais est
// l'identité (clé = libellé de parité), le français est un supplément, et la
// couverture française des menus/raccourcis est complète.
import { describe, expect, test } from 'bun:test';
import { get } from 'svelte/store';
import { isTranslatable, locale, setLocale, t, translate, tr } from './index';
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

	test('additive fallback: unknown key returns the English key', () => {
		expect(translate('fr', 'Totally new label')).toBe('Totally new label');
		expect(isTranslatable('Totally new label')).toBe(false);
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
	test('every main-menu title and label is translatable and non-empty in French', () => {
		const keys = new Set<string>(MAIN_MENUS.map((menu) => menu.title));
		for (const label of allMenuLabels()) keys.add(label);
		const missing = [...keys].filter((k) => !isTranslatable(k) || translate('fr', k).length === 0);
		expect(missing).toEqual([]);
	});

	test('every shortcut group name is translatable in French', () => {
		const missing = SHORTCUT_GROUPS.map((g) => g.group).filter((g) => !isTranslatable(g));
		expect(missing).toEqual([]);
	});

	test('a French translation differs from English where expected', () => {
		// Garde-fou : la table française n'est pas un simple miroir de l'anglais.
		expect(translate('fr', 'New Project')).not.toBe(translate('en', 'New Project'));
	});
});
