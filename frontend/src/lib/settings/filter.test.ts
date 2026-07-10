// Filtre mode + recherche (T042). Cœur pur : rangs de mode, matching, et
// réduction de l'arbre pages/sections.
import { describe, expect, it } from 'bun:test';

import { PARAMS } from '../../generated/params';
import { UI_LAYOUT, type UiPage } from '../../generated/ui-layout';
import { filterLayout, matchesQuery, modeAllows, optionVisible, type DisplayMode } from './filter';

// Une clé réelle de chaque mode pour ancrer les tests.
const byMode = (m: string) => Object.values(PARAMS).find((p) => p.mode === m)!;
const simpleParam = byMode('simple');
const advancedParam = byMode('advanced');
const expertParam = byMode('expert');
const developParam = byMode('develop');

describe('modeAllows', () => {
	it('révèle les modes de rang inférieur ou égal', () => {
		expect(modeAllows('simple', 'simple')).toBe(true);
		expect(modeAllows('advanced', 'simple')).toBe(false);
		expect(modeAllows('advanced', 'advanced')).toBe(true);
		expect(modeAllows('expert', 'advanced')).toBe(false);
		expect(modeAllows('expert', 'expert')).toBe(true);
	});

	it('cache toujours develop dans les modes UI', () => {
		for (const mode of ['simple', 'advanced', 'expert'] as DisplayMode[]) {
			expect(modeAllows('develop', mode)).toBe(false);
		}
	});
});

describe('matchesQuery', () => {
	it('matche clé, libellé et infobulle sans casse', () => {
		const def = PARAMS['layer_height'];
		expect(matchesQuery(def, 'LAYER_HEIGHT')).toBe(true);
		expect(matchesQuery(def, def.label.slice(0, 4).toUpperCase())).toBe(true);
		expect(matchesQuery(def, 'zzz_introuvable')).toBe(false);
	});

	it('requête vide matche tout', () => {
		expect(matchesQuery(PARAMS['layer_height'], '   ')).toBe(true);
	});
});

describe('optionVisible', () => {
	it('applique le filtre de mode hors recherche', () => {
		expect(optionVisible(simpleParam, 'simple', '')).toBe(true);
		expect(optionVisible(advancedParam, 'simple', '')).toBe(false);
		expect(optionVisible(expertParam, 'expert', '')).toBe(true);
		expect(optionVisible(developParam, 'expert', '')).toBe(false);
	});

	it('la recherche traverse les modes mais épargne develop', () => {
		// Un paramètre advanced devient visible en mode simple s'il matche.
		expect(optionVisible(advancedParam, 'simple', advancedParam.key)).toBe(true);
		// Un paramètre develop reste caché même s'il matche.
		expect(optionVisible(developParam, 'expert', developParam.key)).toBe(false);
	});

	it('la recherche filtre les non-correspondances visibles par mode', () => {
		expect(optionVisible(simpleParam, 'expert', 'zzz_introuvable')).toBe(false);
	});
});

describe('filterLayout', () => {
	it('mode simple ⊆ advanced ⊆ expert (options croissantes)', () => {
		const count = (layout: UiPage[]) =>
			layout.reduce((n, p) => n + p.sections.reduce((m, s) => m + s.options.length, 0), 0);
		const simple = count(filterLayout(UI_LAYOUT, 'simple', ''));
		const advanced = count(filterLayout(UI_LAYOUT, 'advanced', ''));
		const expert = count(filterLayout(UI_LAYOUT, 'expert', ''));
		expect(simple).toBeLessThanOrEqual(advanced);
		expect(advanced).toBeLessThanOrEqual(expert);
		expect(simple).toBeGreaterThan(0);
	});

	it('retire les pages et sections vidées par le filtre', () => {
		const filtered = filterLayout(UI_LAYOUT, 'simple', '');
		for (const page of filtered) {
			expect(page.sections.length).toBeGreaterThan(0);
			for (const section of page.sections) {
				expect(section.options.length).toBeGreaterThan(0);
			}
		}
	});

	it('une recherche ciblée ne garde que les correspondances', () => {
		const filtered = filterLayout(UI_LAYOUT, 'simple', 'layer_height');
		const keys = filtered.flatMap((p) => p.sections.flatMap((s) => s.options));
		expect(keys.length).toBeGreaterThan(0);
		for (const key of keys) {
			expect(typeof key).toBe('string');
			expect((key as string).toLowerCase()).toContain('layer_height');
		}
	});

	it('une recherche sans résultat vide l’arbre', () => {
		expect(filterLayout(UI_LAYOUT, 'expert', 'zzz_aucun_param_zzz')).toEqual([]);
	});
});
