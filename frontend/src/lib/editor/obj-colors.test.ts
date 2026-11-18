import { describe, expect, it } from 'vitest';
import { decodeFacet, encodeFacet } from '$lib/scene/gizmos/painting/facets';
import {
	MAX_FILAMENTS,
	colorDistance,
	defaultTargets,
	groupPalette,
	isMtl,
	mapExtruder,
	nearestSlot,
	paletteHex,
	pickMtl,
	planObjColors,
	remapMmu
} from './obj-colors';

const PALETTE = ['#FF0000FF', '#00FF00FF', '#0000FFFF', '#F00A0AFF'];

describe('obj-colors (ObjColorDialog)', () => {
	it('normalise la palette « #RRGGBBAA » du serveur', () => {
		expect(paletteHex('#FF8000FF')).toBe('#ff8000');
		expect(paletteHex('#ff8000')).toBe('#ff8000');
		expect(paletteHex('rouge')).toBeNull();
	});

	it('distance perçue et filament le plus proche', () => {
		expect(colorDistance('#ff0000', '#ff0000')).toBe(0);
		expect(colorDistance('#ff0000', '#f00a0a')).toBeLessThan(colorDistance('#ff0000', '#00ff00'));
		expect(nearestSlot('#e01010', ['#ffffff', '#00ff00', '#ff0000'])).toBe(2);
	});

	it('réduit le nombre de couleurs en fusionnant les plus proches', () => {
		expect(groupPalette(PALETTE, 4)).toHaveLength(4);
		const three = groupPalette(PALETTE, 3);
		expect(three).toHaveLength(3);
		expect(three[0].members).toEqual([0, 3]); // les deux rouges
		expect(three[0].hex).toBe('#f80505');
		expect(groupPalette(PALETTE, 1)[0].members).toEqual([0, 1, 2, 3]);
		expect(groupPalette(PALETTE, 99)).toHaveLength(4);
	});

	it('affectation par défaut : nouveaux filaments (Orca) ou plus proches', () => {
		const groups = groupPalette(PALETTE, 3);
		expect(defaultTargets(groups, ['#ffffff'], 'append')).toEqual([
			{ kind: 'new' },
			{ kind: 'new' },
			{ kind: 'new' }
		]);
		expect(defaultTargets(groups, ['#00ff00', '#ff0000', '#1010f0'], 'match')).toEqual([
			{ kind: 'slot', slot: 1 },
			{ kind: 'slot', slot: 0 },
			{ kind: 'slot', slot: 2 }
		]);
		// Limite de 16 filaments : le surplus va au plus proche.
		const full = Array.from({ length: MAX_FILAMENTS - 1 }, () => '#0000ff');
		const t = defaultTargets(groups, full, 'append');
		expect(t[0]).toEqual({ kind: 'new' });
		expect(t[1]).toEqual({ kind: 'slot', slot: 0 });
	});

	it('plan : nouveaux slots numérotés après les existants, membres fusionnés ensemble', () => {
		const groups = groupPalette(PALETTE, 3);
		const plan = planObjColors(
			PALETTE,
			groups,
			[{ kind: 'new' }, { kind: 'slot', slot: 0 }, { kind: 'new' }],
			2
		);
		expect(plan.newColours).toEqual(['#f80505', '#0000ff']);
		expect(plan.extruderOf).toEqual([3, 1, 4, 3]);
		expect(mapExtruder(plan, 2)).toBe(1);
		expect(mapExtruder(plan, 9)).toBe(9);
	});

	it('renumérote la peinture MMU (facettes entières), garde les triangles subdivisés', () => {
		const plan = { newColours: [], extruderOf: [3, 1, 4, 3] };
		const out = remapMmu(
			{ 0: encodeFacet(1), 1: encodeFacet(2), 2: encodeFacet(3), 3: encodeFacet(4), 4: 'A1' },
			plan
		);
		expect(decodeFacet(out[0])).toBe(3);
		expect(decodeFacet(out[1])).toBe(1);
		expect(decodeFacet(out[2])).toBe(4);
		expect(decodeFacet(out[3])).toBe(3);
		expect(out[4]).toBe('A1');
	});

	it('associe le MTL : mtllib, même nom, ou seul MTL déposé', () => {
		expect(pickMtl('cube.obj', 'mtllib couleurs.mtl\nv 0 0 0', ['autre.mtl', 'Couleurs.MTL'])).toBe(
			'Couleurs.MTL'
		);
		expect(pickMtl('cube.obj', null, ['x.mtl', 'cube.mtl'])).toBe('cube.mtl');
		expect(pickMtl('cube.obj', null, ['x.mtl'])).toBe('x.mtl');
		expect(pickMtl('cube.obj', null, ['x.mtl', 'y.mtl'])).toBeNull();
		expect(pickMtl('cube.obj', null, [])).toBeNull();
		expect(isMtl('A.MTL')).toBe(true);
		expect(isMtl('a.obj')).toBe(false);
	});
});
