// Phase 34 : raccourcis d'Orca absents de sa liste KBShortcutsDialog mais
// câblés dans GLCanvas3D (on_char / on_key).
import { describe, expect, test } from 'vitest';
import { isToolAction, resolveShortcut, rotationFromAction } from './shortcuts';
import { EXTRA_SHORTCUT_GROUP } from '../menus/shortcuts';
import { MAIN_MENUS } from '../menus/menus';

describe('shortcuts : phase 34 (Ctrl+E, Ctrl+7, Page préc./suiv., + / −)', () => {
	test('Ctrl+E bascule les étiquettes, Ctrl+7 vue du plateau courant', () => {
		expect(resolveShortcut({ key: 'e', ctrl: true }, false)).toBe('view.labels');
		expect(resolveShortcut({ key: 'e', meta: true }, false)).toBe('view.labels');
		expect(resolveShortcut({ key: '7', ctrl: true }, false)).toBe('view.plate');
		// E seul reste le gizmo « oreilles de bordure », 7 seul le filament 7.
		expect(resolveShortcut({ key: 'e' }, false)).toBe('brim-ears');
		expect(resolveShortcut({ key: '7' }, false)).toBe('object.extruder.7');
	});

	test('Page préc./suiv. : rotation ±45° (Orca _do_rotate(±π/4))', () => {
		const up = resolveShortcut({ key: 'PageUp' }, false);
		const down = resolveShortcut({ key: 'PageDown' }, false);
		expect(up).toBe('selection.rotate+45');
		expect(down).toBe('selection.rotate-45');
		expect(rotationFromAction(up!)).toBe(45);
		expect(rotationFromAction(down!)).toBe(-45);
		expect(rotationFromAction('rotate')).toBeNull();
		expect(isToolAction(up!)).toBe(false);
	});

	test('« + » / « − » : instances (Maj de la disposition ignorée), Ctrl± laissé au zoom navigateur', () => {
		expect(resolveShortcut({ key: '+' }, false)).toBe('object.increaseInstances');
		expect(resolveShortcut({ key: '+', shift: true }, false)).toBe('object.increaseInstances');
		expect(resolveShortcut({ key: '-' }, false)).toBe('object.decreaseInstances');
		expect(resolveShortcut({ key: '+', ctrl: true }, false)).toBeNull();
		expect(resolveShortcut({ key: '-', ctrl: true }, false)).toBeNull();
	});

	test('champ éditable : aucun des raccourcis de la phase 34 n’est capté', () => {
		for (const chord of [
			{ key: 'e', ctrl: true },
			{ key: '7', ctrl: true },
			{ key: 'PageUp' },
			{ key: '+' },
			{ key: '-' }
		])
			expect(resolveShortcut(chord, true)).toBeNull();
	});
});

describe('aide « Autres raccourcis » et menu Vue', () => {
	test('chaque raccourci affiché dans « Autres raccourcis » est réellement câblé', () => {
		for (const { keys } of EXTRA_SHORTCUT_GROUP.shortcuts) {
			const parts = keys.split('+').filter(Boolean);
			const key = keys === '+' ? '+' : parts.pop()!.replace(' ', '');
			const chord = { key, ctrl: keys !== '+' && parts.includes('Ctrl') };
			expect(resolveShortcut(chord, false), keys).not.toBeNull();
		}
	});
	test('« Show Labels » affiche Ctrl+E (Orca MainFrame)', () => {
		const view = MAIN_MENUS.find((m) => m.title === 'View')!;
		const labels = view.entries.find((e) => e !== 'separator' && e.action === 'view.labels');
		expect(labels && labels !== 'separator' ? labels.shortcut : null).toBe('Ctrl+E');
	});
});
