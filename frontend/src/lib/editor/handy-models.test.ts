import { describe, expect, it } from 'vitest';
import {
	HANDY_MENU_LABEL,
	handyKeyOf,
	handyLabel,
	suggestionApplies,
	withHandyMenu,
	type HandyModelInfo
} from './handy-models';
import { SCENE_ADD_ITEMS } from '#lib/scene/menus';

const MODELS: HandyModelInfo[] = [
	{ key: 'orca-cube', label: 'Orca Cube', files: ['a.drc', 'b.drc'], arrange_after_import: true },
	{
		key: 'orca-string-hell',
		label: 'Orca String Hell',
		files: ['s.drc'],
		arrange_after_import: false,
		suggestion: {
			message: 'm',
			when_all: ['only_one_wall_top', 'min_width_top_surface'],
			apply: { min_width_top_surface: '0' }
		}
	}
];

describe('modèles pratiques (Orca GUI_Factories.cpp:576)', () => {
	it('sous-menu inséré après les primitives', () => {
		const menu = withHandyMenu(SCENE_ADD_ITEMS, MODELS);
		const torus = menu.findIndex((e) => e !== 'separator' && e.action === 'add.torus');
		const sub = menu[torus + 2];
		expect(menu[torus + 1]).toBe('separator');
		expect(sub !== 'separator' && sub.label).toBe(HANDY_MENU_LABEL);
		expect(sub !== 'separator' && sub.items?.map((i) => i.action)).toEqual([
			'handy.orca-cube',
			'handy.orca-string-hell'
		]);
		expect(withHandyMenu(SCENE_ADD_ITEMS, [])).toEqual(SCENE_ADD_ITEMS);
	});
	it('action → clé', () => {
		expect(handyKeyOf('handy.3dbenchy')).toBe('3dbenchy');
		expect(handyKeyOf('add.cube')).toBeNull();
		expect(handyLabel({ label: 'Stanford Bunny' })).toBe('Lapin de Stanford');
		expect(handyLabel({ label: 'Inconnu' })).toBe('Inconnu');
	});
	it('suggestion String Hell : toutes les clés vraies ou > 0', () => {
		const s = MODELS[1].suggestion!;
		expect(suggestionApplies(s, { only_one_wall_top: '1', min_width_top_surface: '300%' })).toBe(
			true
		);
		expect(suggestionApplies(s, { only_one_wall_top: true, min_width_top_surface: 2 })).toBe(true);
		expect(suggestionApplies(s, { only_one_wall_top: '0', min_width_top_surface: '300%' })).toBe(
			false
		);
		expect(suggestionApplies(s, { only_one_wall_top: '1', min_width_top_surface: '0' })).toBe(
			false
		);
		expect(suggestionApplies(s, {})).toBe(false);
		expect(suggestionApplies(undefined, {})).toBe(false);
	});
});
