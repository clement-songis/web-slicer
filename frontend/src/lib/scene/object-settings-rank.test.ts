import { describe, expect, it } from 'vitest';
import { PARAMS } from '../../generated/params';
import { queryRank } from '$lib/settings/filter';
import { searchOverridable } from './object-settings';

describe('recherche « + Réglage » classée par pertinence (phase 26)', () => {
	it('libellé avant clé avant infobulle', () => {
		const sparse = PARAMS['sparse_infill_density'];
		expect(queryRank(sparse, 'sparse')).toBe(0);
		expect(queryRank(sparse, 'infill')).toBe(1);
		expect(queryRank(sparse, 'zzz-absent')).toBe(5);
		expect(queryRank(sparse, '')).toBe(0);
		// Réglage dont seule la clé contient la requête.
		const bottom = PARAMS['bottom_solid_infill_flow_ratio'];
		expect(queryRank(bottom, 'infill')).toBeGreaterThan(queryRank(sparse, 'infill'));
	});

	it('« infill » : la densité de remplissage passe avant les ratios de débit', () => {
		const keys = searchOverridable('infill', 'advanced', false, {}, 60).map((d) => d.key);
		expect(keys).toContain('sparse_infill_density');
		const i = keys.indexOf('sparse_infill_density');
		const j = keys.indexOf('bottom_solid_infill_flow_ratio');
		if (j >= 0) expect(i).toBeLessThan(j);
	});

	it('sans requête : ordre par catégorie puis clé (inchangé)', () => {
		const defs = searchOverridable('', 'advanced', false, {}, 1000);
		const sorted = [...defs].sort(
			(a, b) => a.category.localeCompare(b.category) || a.key.localeCompare(b.key)
		);
		expect(defs.map((d) => d.key)).toEqual(sorted.map((d) => d.key));
	});
});
