import { describe, expect, it } from 'vitest';
import type { ProjectResponse } from '#lib/api/types';
import { limitRecent, startPath } from './library';
import { clampRecentCount, parsePreferences } from '#lib/settings/preferences';

const p = (id: string, updated_at: string) => ({ id, updated_at }) as unknown as ProjectResponse;

describe('Préférences Orca default_page / max_recent_count', () => {
	it('défauts d’Orca : Accueil, 18 projets récents', () => {
		const prefs = parsePreferences(null);
		expect(prefs.defaultPage).toBe('home');
		expect(prefs.maxRecent).toBe(18);
	});
	it('lecture tolérante et bornes 0–999', () => {
		const prefs = parsePreferences(JSON.stringify({ defaultPage: 'prepare', maxRecent: 5000 }));
		expect(prefs.defaultPage).toBe('prepare');
		expect(prefs.maxRecent).toBe(999);
		expect(parsePreferences(JSON.stringify({ defaultPage: 'x', maxRecent: 'a' }))).toMatchObject({
			defaultPage: 'home',
			maxRecent: 18
		});
		expect(clampRecentCount(-3)).toBe(0);
		expect(clampRecentCount(4.6)).toBe(5);
		expect(clampRecentCount(Number.NaN)).toBe(18);
	});
	it('page de démarrage : dernier projet modifié ou bibliothèque', () => {
		const list = [p('a', '2026-01-01T00:00:00Z'), p('b', '2026-03-01T00:00:00Z')];
		expect(startPath('prepare', list)).toBe('/projects/b');
		expect(startPath('prepare', [])).toBe('/library');
		expect(startPath('home', list)).toBe('/library');
	});
	it('vue récente bornée, « Tout afficher » lève la borne', () => {
		const list = [1, 2, 3, 4, 5];
		expect(limitRecent(list, 3, false)).toEqual({ items: [1, 2, 3], hidden: 2 });
		expect(limitRecent(list, 3, true)).toEqual({ items: list, hidden: 0 });
		expect(limitRecent(list, 0, false)).toEqual({ items: [], hidden: 5 });
		expect(limitRecent(list, 18, false).hidden).toBe(0);
	});
});
