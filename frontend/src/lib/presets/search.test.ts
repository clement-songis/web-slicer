import { describe, expect, it } from 'vitest';
import {
	filterPresetList,
	GENERIC_GROUP,
	isLikelyCompatible,
	matchesQuery,
	normalizeSearch,
	SELECTED_GROUP,
	USER_GROUP
} from './search';

const p = (id: string, name: string, vendor?: string, origin = 'system') => ({
	id,
	name,
	vendor,
	origin
});

const presets = [
	p('v1', '0.20mm Standard @Voron 0.4 nozzle', 'Voron'),
	p('v2', '0.30mm Draft @Voron 0.6 nozzle', 'Voron'),
	p('v3', '0.15mm Optimal @Voron', 'Voron'),
	p('e1', '0.20mm Standard @Elegoo CC 0.4 nozzle', 'Elegoo'),
	p('g1', '0.20mm Standard (générique)', 'Custom'),
	p('g2', '0.12mm Fine', undefined),
	p('u1', 'Mon réglage Voron', 'Voron', 'user')
];
const voron = { name: 'Voron 2.4 350 0.4 nozzle', vendor: 'Voron' };
const ids = (r: ReturnType<typeof filterPresetList>) =>
	r.groups.flatMap((g) => g.presets.map((x) => x.id));

describe('normalizeSearch / matchesQuery', () => {
	it('ignore accents, casse et espaces', () => {
		expect(normalizeSearch('  Générique   ÉLÉGOO ')).toBe('generique elegoo');
	});
	it('exige tous les mots, dans le nom ou le fabricant', () => {
		expect(matchesQuery(presets[4], 'GENERIQUE 0.20')).toBe(true);
		expect(matchesQuery(presets[0], 'voron draft')).toBe(false);
		expect(matchesQuery(p('x', '0.2mm Fine', 'Élégoo'), 'elegoo')).toBe(true);
		expect(matchesQuery(presets[0], '   ')).toBe(true);
	});
});

describe('isLikelyCompatible', () => {
	it('écarte un autre fabricant et une autre buse', () => {
		expect(isLikelyCompatible(presets[0], voron)).toBe(true);
		expect(isLikelyCompatible(presets[1], voron)).toBe(false);
		expect(isLikelyCompatible(presets[3], voron)).toBe(false);
	});
	it('garde génériques, presets sans buse et presets utilisateur', () => {
		expect(isLikelyCompatible(presets[2], voron)).toBe(true);
		expect(isLikelyCompatible(presets[4], voron)).toBe(true);
		expect(isLikelyCompatible(presets[5], voron)).toBe(true);
		expect(isLikelyCompatible(p('u', 'X @Elegoo 0.8 nozzle', 'Elegoo', 'user'), voron)).toBe(true);
	});
	it('sans imprimante active, tout est compatible', () => {
		expect(isLikelyCompatible(presets[3], null)).toBe(true);
	});
});

describe('filterPresetList', () => {
	it('ne montre que les compatibles, groupés : utilisateur, fabricant actif, génériques', () => {
		const r = filterPresetList(presets, { printer: voron });
		expect(r.groups.map((g) => g.label)).toEqual([USER_GROUP, 'Voron', GENERIC_GROUP]);
		expect(ids(r)).toEqual(['u1', 'v1', 'v3', 'g1', 'g2']);
		expect(r).toMatchObject({ shown: 5, compatible: 5, total: 7 });
	});
	it('« tous » regroupe chaque fabricant, celui de l’imprimante en tête', () => {
		const r = filterPresetList(presets, { printer: voron, showAll: true });
		expect(r.groups.map((g) => g.label)).toEqual([USER_GROUP, 'Voron', GENERIC_GROUP, 'Elegoo']);
		expect(r.shown).toBe(7);
	});
	it('combine recherche et compatibilité', () => {
		const r = filterPresetList(presets, { printer: voron, query: 'standard' });
		expect(ids(r)).toEqual(['v1', 'g1']);
		expect(r.compatible).toBe(5);
		expect(ids(filterPresetList(presets, { printer: voron, query: 'elegoo' }))).toEqual([]);
		expect(
			ids(filterPresetList(presets, { printer: voron, query: 'elegoo', showAll: true }))
		).toEqual(['e1']);
	});
	it('garde la sélection masquée dans un groupe à part', () => {
		const r = filterPresetList(presets, { printer: voron, query: 'fine', selectedId: 'e1' });
		expect(r.groups[0]).toEqual({ label: SELECTED_GROUP, presets: [presets[3]] });
		expect(r.shown).toBe(1);
		const visible = filterPresetList(presets, { printer: voron, selectedId: 'v1' });
		expect(visible.groups.some((g) => g.label === SELECTED_GROUP)).toBe(false);
	});
	it('sans imprimante : utilisateur, génériques, puis fabricants par ordre alphabétique', () => {
		const r = filterPresetList(presets, {});
		expect(r.groups.map((g) => g.label)).toEqual([USER_GROUP, GENERIC_GROUP, 'Elegoo', 'Voron']);
		expect(r.shown).toBe(7);
	});
});
