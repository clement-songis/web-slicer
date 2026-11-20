import { describe, expect, test } from 'vitest';
import {
	applyLayerCode,
	codeAt,
	layerCodeMenu,
	remapCodesAfterFilamentRemoval
} from './layer-code-menu';
import type { PlateCustomGcode } from './color-changes';

const colors = ['#FF0000', '#00FF00', '#0000FF'];

describe('menu du curseur de couches (IMSlider d’Orca)', () => {
	test('couche libre : pause, G-code perso, couleur ; modèle si template non vide ; filaments si > 1', () => {
		const single = layerCodeMenu({ codes: [], z: 1, filamentColors: ['#FFF'] });
		expect(single.map((i) => i.action)).toEqual(['add-pause', 'add-custom', 'add-color']);
		const multi = layerCodeMenu({
			codes: [],
			z: 1,
			filamentColors: colors,
			templateGcode: 'M117 {layer_num}'
		});
		expect(multi.map((i) => i.action)).toEqual([
			'add-pause',
			'add-custom',
			'add-template',
			'add-color',
			'tool-change',
			'tool-change',
			'tool-change'
		]);
		expect(multi.filter((i) => i.action === 'tool-change').map((i) => i.extruder)).toEqual([
			1, 2, 3
		]);
		expect(multi[0].hint).toContain('pause');
	});

	test('objet par objet : ajouts grisés comme dmSequentialFffPrint', () => {
		const items = layerCodeMenu({ codes: [], z: 1, filamentColors: ['#FFF'], sequential: true });
		expect(items.find((i) => i.action === 'add-pause')?.disabled).toBe(true);
		expect(items.find((i) => i.action === 'add-custom')?.disabled).toBe(true);
	});

	test('couche occupée : modifier / supprimer selon le type', () => {
		const codes: PlateCustomGcode[] = [
			{ printZ: 1, type: 'custom', extruder: 1, extra: 'G4 S1' },
			{ printZ: 2, type: 'pause_print', extruder: 1 },
			{ printZ: 3, type: 'tool_change', extruder: 2 }
		];
		const custom = layerCodeMenu({ codes, z: 1, filamentColors: colors });
		expect(custom.map((i) => i.label)).toEqual([
			'Modifier le G-code personnalisé',
			'Supprimer le G-code personnalisé'
		]);
		expect(layerCodeMenu({ codes, z: 2, filamentColors: colors }).map((i) => i.label)).toEqual([
			'Supprimer la pause'
		]);
		const tool = layerCodeMenu({ codes, z: 3, filamentColors: colors });
		expect(
			tool.filter((i) => i.action === 'tool-change').find((i) => i.extruder === 2)?.disabled
		).toBe(true);
		expect(tool.at(-1)?.label).toBe('Supprimer le changement de filament');
	});

	test('ajouts : pause, G-code perso (vide refusé), modèle, couleur, triés par Z', () => {
		let res = applyLayerCode([], { action: 'add-pause' }, 2, '10');
		expect(res.changed).toBe(true);
		expect(res.list).toEqual([{ printZ: 2, type: 'pause_print', extruder: 1 }]);
		expect(res.message).toContain('Pause ajouté à la couche 10');
		expect(
			applyLayerCode(res.list, { action: 'add-custom' }, 1, '5', { extra: '  ' }).changed
		).toBe(false);
		res = applyLayerCode(res.list, { action: 'add-custom' }, 1, '5', { extra: ' M117 hi ' });
		expect(res.list.map((g) => [g.printZ, g.type, g.extra])).toEqual([
			[1, 'custom', 'M117 hi'],
			[2, 'pause_print', undefined]
		]);
		res = applyLayerCode(res.list, { action: 'add-template' }, 3, '15');
		expect(res.list.at(-1)?.type).toBe('template');
		res = applyLayerCode(res.list, { action: 'add-color' }, 4, '20', { filamentColors: colors });
		expect(res.list.at(-1)).toMatchObject({
			type: 'color_change',
			color: expect.stringMatching(/^#/)
		});
		const dup = applyLayerCode(res.list, { action: 'add-pause' }, 2.00001, '10');
		expect(dup.changed).toBe(false);
	});

	test('changement de filament : ajout, bascule de filament (switch_code_for_tick), refus sur un autre code', () => {
		let res = applyLayerCode([], { action: 'tool-change', extruder: 2 }, 1, '4', {
			filamentColors: colors
		});
		expect(res.list).toEqual([{ printZ: 1, type: 'tool_change', extruder: 2, color: '#00FF00' }]);
		res = applyLayerCode(res.list, { action: 'tool-change', extruder: 3 }, 1, '4', {
			filamentColors: colors
		});
		expect(res.list[0]).toMatchObject({ extruder: 3, color: '#0000FF' });
		const pause = applyLayerCode([], { action: 'add-pause' }, 1, '4').list;
		expect(applyLayerCode(pause, { action: 'tool-change', extruder: 2 }, 1, '4').changed).toBe(
			false
		);
	});

	test('modifier puis supprimer un G-code perso', () => {
		const codes: PlateCustomGcode[] = [{ printZ: 1, type: 'custom', extruder: 1, extra: 'A' }];
		const edited = applyLayerCode(codes, { action: 'edit-custom' }, 1, '3', { extra: 'B' });
		expect(edited.list[0].extra).toBe('B');
		expect(codeAt(edited.list, 1)?.extra).toBe('B');
		expect(
			applyLayerCode(edited.list, { action: 'edit-custom' }, 1, '3', { extra: 'B' }).changed
		).toBe(false);
		const removed = applyLayerCode(edited.list, { action: 'delete' }, 1, '3');
		expect(removed.list).toEqual([]);
		expect(removed.message).toContain('G-code personnalisé retiré de la couche 3');
		expect(codes[0].extra).toBe('A');
	});

	test('fusion / retrait de filament : changements de filament réaffectés (on_filaments_delete)', () => {
		const codes: PlateCustomGcode[] = [
			{ printZ: 1, type: 'tool_change', extruder: 2 },
			{ printZ: 2, type: 'tool_change', extruder: 3 },
			{ printZ: 3, type: 'pause_print', extruder: 1 }
		];
		// Slot 1 (filament 2) fusionné dans le slot 2 (filament 3 → devient 2).
		expect(remapCodesAfterFilamentRemoval(codes, 1, 2).map((g) => g.extruder)).toEqual([2, 2, 1]);
		// Fusionné dans le slot 0 (filament 1).
		expect(remapCodesAfterFilamentRemoval(codes, 1, 0).map((g) => g.extruder)).toEqual([1, 2, 1]);
		// Simple suppression : le changement vers le filament retiré disparaît.
		expect(remapCodesAfterFilamentRemoval(codes, 1, null).map((g) => g.printZ)).toEqual([2, 3]);
	});
});
