import { describe, expect, it } from 'vitest';
import {
	OBJECT_CONTEXT_ITEMS,
	VOLUME_SHAPES,
	menuLeaves,
	parseVolumeShapeAction,
	volumeShapeItems,
	type ObjectMenuItem
} from './menus';
import { genericVolumePlacement } from './volumes';

// Audit 43 M1-a : MenuFactory::append_submenu_add_generic (GUI_Factories.cpp:541).
describe('sous-menu de formes des volumes', () => {
	it('mêmes entrées et même ordre qu’Orca', () => {
		expect(VOLUME_SHAPES.map((s) => s.label)).toEqual([
			'Load...',
			'Cube',
			'Cylinder',
			'Sphere',
			'Cone',
			'Disc',
			'Torus',
			'Text',
			'SVG'
		]);
	});
	it('les 5 ajouts de volume portent le sous-menu, l’action d’origine reste une feuille', () => {
		const parts = OBJECT_CONTEXT_ITEMS.find(
			(e) => e !== 'separator' && e.action === 'section.parts'
		) as ObjectMenuItem;
		const split = parts.items!.filter((i) => i.split);
		expect(split.map((i) => i.action)).toEqual([
			'object.addPart',
			'object.addNegativeVolume',
			'object.addModifier',
			'object.addSupportBlocker',
			'object.addSupportEnforcer'
		]);
		for (const item of split) expect(item.items).toEqual(volumeShapeItems(item.action));
		const leaves = menuLeaves(OBJECT_CONTEXT_ITEMS).map((l) => l.action);
		expect(leaves).toContain('object.addModifier');
		expect(leaves.some((a) => a.includes(':'))).toBe(false);
	});
	it('analyse des actions de forme', () => {
		expect(parseVolumeShapeAction('object.addModifier:cylinder')).toEqual({
			base: 'object.addModifier',
			shape: 'cylinder'
		});
		expect(parseVolumeShapeAction('object.addPart:load')?.shape).toBe('load');
		expect(parseVolumeShapeAction('object.addModifier')).toBeNull();
		expect(parseVolumeShapeAction('volume.type:part')).toBeNull();
	});
});

// ObjectList::load_generic_subobject : coin avant droit de la boîte, posé au plateau.
describe('pose d’une forme générique', () => {
	const cube = new Float32Array([-10, -10, -10, 10, 10, 10]);
	it('objet sans rotation : (max.x, min.y, min.z) + ½ taille, repère recentré', () => {
		const p = genericVolumePlacement(
			cube,
			{ position: [100, 100, 10], rotation: [0, 0, 0], scale: [1, 1, 1] },
			[35, 35, 35]
		);
		expect(p.offset).toEqual([27.5, 7.5, 7.5]);
		expect(p.rotation).toBeUndefined();
	});
	it('objet à l’échelle 2 : décalage divisé par l’échelle', () => {
		const p = genericVolumePlacement(
			cube,
			{ position: [100, 100, 20], rotation: [0, 0, 0], scale: [2, 2, 2] },
			[10, 10, 10]
		);
		expect(p.offset).toEqual([12.5, -7.5, -7.5]);
	});
	it('objet tourné de 90° en Z : rotation inverse, forme alignée sur le plateau', () => {
		const p = genericVolumePlacement(
			cube,
			{ position: [0, 0, 10], rotation: [0, 0, 90], scale: [1, 1, 1] },
			[10, 10, 10]
		);
		expect(p.rotation?.[2]).toBeCloseTo(-90);
		// Monde : centre (15, −5, 5) → local = Rz(−90)·(15, −5, −5) = (−5, −15, −5).
		expect(p.offset[0]).toBeCloseTo(-5);
		expect(p.offset[1]).toBeCloseTo(-15);
		expect(p.offset[2]).toBeCloseTo(-5);
	});
});
