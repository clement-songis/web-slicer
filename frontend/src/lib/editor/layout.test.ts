// Tests de l'orchestrateur de disposition de l'éditeur (T097).
import { describe, expect, test } from 'bun:test';
import {
	EDITOR_DEFAULT_THEME,
	EDITOR_TABS,
	initialLayout,
	setTab,
	showsPrepare,
	showsPreview
} from './layout';

describe('layout', () => {
	test('défaut : onglet Préparer, thème sombre', () => {
		const s = initialLayout();
		expect(s.tab).toBe('prepare');
		expect(EDITOR_DEFAULT_THEME).toBe('dark');
		expect(showsPrepare(s)).toBe(true);
		expect(showsPreview(s)).toBe(false);
	});

	test('les 4 onglets OrcaSlicer sont exposés dans l’ordre', () => {
		expect(EDITOR_TABS).toEqual(['prepare', 'preview', 'device', 'project']);
	});

	test('setTab bascule de façon immuable', () => {
		const a = initialLayout();
		const b = setTab(a, 'preview');
		expect(b.tab).toBe('preview');
		expect(a.tab).toBe('prepare'); // source inchangée
		expect(showsPreview(b)).toBe(true);
		expect(showsPrepare(b)).toBe(false);
	});

	test('overrides à la construction', () => {
		expect(initialLayout({ tab: 'device' }).tab).toBe('device');
	});
});
