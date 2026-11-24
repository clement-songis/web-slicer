import { describe, expect, test } from 'vitest';
import { DEFAULT_PREFERENCES, parsePreferences } from './preferences';
import { displayUnit } from './widgets/types';
import { languageName } from '#lib/i18n';
import { effectiveRealistic } from '#lib/scene/realistic';
import { supportFilamentAfterRemoval } from '#lib/editor/presets';

describe('phase 38 — préférences, unités, langue', () => {
	test('vue réaliste : défauts d’Orca (mode off, Phong on, ombres et lissage off)', () => {
		expect(DEFAULT_PREFERENCES.graphics).toMatchObject({
			realisticView: false,
			realisticPhong: true,
			realisticShadows: false,
			realisticSmoothNormals: false
		});
		const p = parsePreferences(
			JSON.stringify({ graphics: { realisticView: true, realisticShadows: 'x' } })
		);
		expect(p.graphics.realisticView).toBe(true);
		expect(p.graphics.realisticShadows).toBe(false);
	});

	test('vue réaliste suspendue pendant supports / couture / fuzzy, pas en MMU', () => {
		const g = {
			...DEFAULT_PREFERENCES.graphics,
			realisticView: true,
			realisticSmoothNormals: true
		};
		expect(effectiveRealistic(g)).toEqual({ phong: true, shadows: false, smoothNormals: true });
		expect(effectiveRealistic(g, 'seam')).toBeNull();
		expect(effectiveRealistic(g, 'supports')).toBeNull();
		expect(effectiveRealistic(g, 'fuzzy')).toBeNull();
		expect(effectiveRealistic(g, 'mmu')).not.toBeNull();
		expect(effectiveRealistic(DEFAULT_PREFERENCES.graphics)).toBeNull();
	});

	test('unités monétaires : devise des préférences au lieu de « argent »', () => {
		const tr = (k: string) => (k === 'mm/s' ? 'mm/s' : k === 'layers' ? 'couches' : k);
		expect(displayUnit('money/kg', tr, '€')).toBe('€/kg');
		expect(displayUnit('money/h', tr, 'CHF')).toBe('CHF/h');
		expect(displayUnit('money/kg', tr, '  ')).toBe('€/kg');
		expect(displayUnit('layers', tr, '€')).toBe('couches');
		expect(displayUnit('', tr, '€')).toBe('');
	});

	test('langue affichée par son nom', () => {
		expect(languageName('fr')).toBe('Français');
		expect(languageName('en')).toBe('English');
	});

	test('filament de support après retrait d’un slot (Orca on_filaments_delete)', () => {
		expect(supportFilamentAfterRemoval(2, 1)).toBe(0); // retiré → Défaut
		expect(supportFilamentAfterRemoval(3, 1)).toBe(2); // décalé
		expect(supportFilamentAfterRemoval('1', 1)).toBe(1);
		expect(supportFilamentAfterRemoval(0, 0)).toBe(0);
		expect(supportFilamentAfterRemoval('abc', 0)).toBeNull();
	});
});
