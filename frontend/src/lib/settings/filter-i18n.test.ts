// Recherche de paramètres dans la locale courante (passe ergonomie) : le
// libellé et l'infobulle traduits sont cherchés en plus de l'anglais Orca, sans
// tenir compte des accents. Couverture FR du dialogue de réglages.
import { afterEach, beforeAll, describe, expect, it } from 'vitest';
import { PARAMS } from '../../generated/params';
import { UI_LAYOUT } from '../../generated/ui-layout';
import { isTranslatable, setLocale, translate } from '$lib/i18n';
import { filterLayout, matchesQuery } from './filter';
import { loadSettingsTranslations } from './i18n';

// Dictionnaire paresseux des réglages : chargé comme à l'ouverture du dialogue.
beforeAll(() => loadSettingsTranslations('fr'));
afterEach(() => setLocale('en'));

describe('recherche traduite', () => {
	it('trouve un paramètre par son libellé français', () => {
		setLocale('fr');
		expect(matchesQuery(PARAMS['seam_position'], 'couture')).toBe(true);
		expect(matchesQuery(PARAMS['layer_height'], 'hauteur de couche')).toBe(true);
	});

	it('garde la recherche anglaise et par clé en français', () => {
		setLocale('fr');
		expect(matchesQuery(PARAMS['seam_position'], 'seam')).toBe(true);
		expect(matchesQuery(PARAMS['seam_position'], 'seam_position')).toBe(true);
	});

	it('ignore les accents et la casse', () => {
		setLocale('fr');
		const label = translate('fr', PARAMS['seam_position'].label);
		expect(label).toMatch(/couture/i);
		expect(matchesQuery(PARAMS['wall_loops'], 'PAROIS')).toBe(true);
		expect(matchesQuery(PARAMS['print_sequence'], 'sequence')).toBe(true);
	});

	it('filterLayout en français ne retient que les pages correspondantes', () => {
		setLocale('fr');
		const pages = filterLayout(
			UI_LAYOUT.filter((p) => p.kind === 'process'),
			'expert',
			'couture'
		);
		const keys = pages.flatMap((p) => p.sections.flatMap((s) => s.options));
		expect(keys).toContain('seam_position');
	});

	it("en anglais, un terme français ne matche pas (pas d'effet de bord)", () => {
		setLocale('en');
		expect(matchesQuery(PARAMS['seam_position'], 'couture')).toBe(false);
	});
});

describe('couverture FR du dialogue de réglages', () => {
	const process = UI_LAYOUT.filter((p) => p.kind === 'process');
	const defs = process
		.flatMap((p) =>
			p.sections.flatMap((s) =>
				s.options.filter((o): o is string => typeof o === 'string' && !!PARAMS[o])
			)
		)
		.map((k) => PARAMS[k]);

	it('titres de pages et sections du process traduits', () => {
		const titles = process.flatMap((p) => [
			p.title,
			...p.sections.map((s) => s.title).filter(Boolean)
		]);
		const missing = titles.filter((t) => !isTranslatable(t));
		expect(missing).toEqual([]);
	});

	it('≥ 98 % des libellés, infobulles et valeurs d’énumération traduits', () => {
		const texts = defs.flatMap((d) => [d.label, d.tooltip, ...d.enumLabels].filter(Boolean));
		const ok = texts.filter((t) => isTranslatable(t)).length;
		expect(ok / texts.length).toBeGreaterThanOrEqual(0.98);
	});
});
