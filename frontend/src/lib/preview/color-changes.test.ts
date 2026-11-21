import { beforeAll, afterAll, describe, expect, test } from 'vitest';
import { setLocale } from '$lib/i18n';
import {
	colorChangeColor,
	editColorChangeAt,
	pendingTicks,
	withAuthoredTypes,
	type PlateCustomGcode
} from './color-changes';

describe('changements de couleur au clavier (Aperçu, « + » / « − »)', () => {
	test('« + » ajoute un changement de couleur trié par Z, une seule fois par couche', () => {
		const first = editColorChangeAt([], '+', 2.2, '11', '#00AE42');
		expect(first.changed).toBe(true);
		expect(first.list).toEqual([
			{
				printZ: 2.2,
				type: 'color_change',
				extruder: 1,
				color: expect.stringMatching(/^#[0-9A-F]{6}$/)
			}
		]);
		expect(first.message).toContain('couche 11');
		const second = editColorChangeAt(first.list, '+', 1.0, '5', '#00AE42');
		expect(second.list.map((g) => g.printZ)).toEqual([1.0, 2.2]);
		const dup = editColorChangeAt(second.list, '+', 2.2000001, '11', '#00AE42');
		expect(dup.changed).toBe(false);
		expect(dup.message).toContain('déjà');
	});

	test('« − » retire le code de la couche courante (tout type), sinon message', () => {
		const list: PlateCustomGcode[] = [
			{ printZ: 1, type: 'pause_print' },
			{ printZ: 2, type: 'color_change', color: '#FF0000' }
		];
		const out = editColorChangeAt(list, '-', 1, '5', '#FFFFFF');
		expect(out.list).toEqual([list[1]]);
		expect(out.message).toContain('Code personnalisé retiré');
		expect(editColorChangeAt(list, '-', 3, '15', '#FFFFFF').changed).toBe(false);
	});

	test('couleur proposée : teinte opposée (+65°) de la couleur en cours (Orca get_color_for_tick)', () => {
		// Rouge pur (0°) → teinte 65°, s = v = 0,85.
		expect(colorChangeColor([], 1, '#FF0000')).toBe('#C9D921');
		// Un changement plus bas fait foi (et non le filament).
		const below: PlateCustomGcode[] = [{ printZ: 0.5, type: 'color_change', color: '#FF0000' }];
		expect(colorChangeColor(below, 1, '#0000FF')).toBe('#C9D921');
		// Couleur illisible : vert Orca par défaut, sans exception.
		expect(colorChangeColor([], 1, 'nope')).toMatch(/^#[0-9A-F]{6}$/);
	});

	test('repères en attente : codes du plateau absents du résultat tranché', () => {
		const layers = [{ z: 0.2 }, { z: 0.4 }, { z: 0.6 }];
		const codes: PlateCustomGcode[] = [
			{ printZ: 0.4, type: 'color_change' },
			{ printZ: 0.6, type: 'pause_print' },
			{ printZ: 9, type: 'color_change' }
		];
		expect(pendingTicks(codes, layers, [{ layer: 2, type: 'pause_print' }])).toEqual([
			{ layer: 1, type: 'color_change' }
		]);
	});

	test('repères tranchés : le type posé fait foi (Orca 2.4.1 écrit ;CUSTOM_GCODE)', () => {
		const layers = [{ z: 0.2 }, { z: 0.4 }, { z: 0.6 }];
		// Marqueur du moteur « custom » (golden cube_custom.orca.gcode : pas de
		// M600 ni ;COLOR_CHANGE) + code posé color_change à la même couche.
		const codes: PlateCustomGcode[] = [{ printZ: 0.4, type: 'color_change' }];
		expect(withAuthoredTypes([{ layer: 1, type: 'custom' }], codes, layers)).toEqual([
			{ layer: 1, type: 'color_change' }
		]);
		// Marqueurs sans code posé (G-code ouvert) : inchangés, même couche.
		expect(withAuthoredTypes([{ layer: 1, type: 'custom' }], [], layers)).toEqual([
			{ layer: 1, type: 'custom' }
		]);
		// Pause posée : le marqueur pause_print garde son type (le G-code porte
		// ;PAUSE_PRINT) ; couche hors bornes : marqueur inchangé.
		expect(
			withAuthoredTypes(
				[{ layer: 2, type: 'pause_print' }],
				[{ printZ: 0.6, type: 'pause_print' }],
				layers
			)
		).toEqual([{ layer: 2, type: 'pause_print' }]);
		expect(withAuthoredTypes([{ layer: 9, type: 'custom' }], codes, layers)).toEqual([
			{ layer: 9, type: 'custom' }
		]);
	});
});

// Les libellés de ces modules sont traduits (tr) : valeurs attendues en français.
beforeAll(() => setLocale('fr'));
afterAll(() => setLocale('en'));
