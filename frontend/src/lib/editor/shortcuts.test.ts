// Tests de l'orchestrateur de raccourcis clavier de l'éditeur (T114).
import { describe, expect, test } from 'vitest';
import { chordString, resolveShortcut, isToolAction } from './shortcuts';

describe('shortcuts', () => {
	test('chordString : modificateurs triés, Cmd≡Ctrl, touches spéciales', () => {
		expect(chordString({ key: 's', ctrl: true })).toBe('ctrl+s');
		expect(chordString({ key: 's', meta: true })).toBe('ctrl+s');
		expect(chordString({ key: 'S', ctrl: true, shift: true })).toBe('ctrl+shift+s');
		expect(chordString({ key: 'Delete' })).toBe('del');
		expect(chordString({ key: 'Escape' })).toBe('esc');
		expect(chordString({ key: 'a' })).toBe('a');
	});

	test('accords modificateurs → actions de menu', () => {
		expect(resolveShortcut({ key: 's', ctrl: true }, false)).toBe('project.save');
		expect(resolveShortcut({ key: 'c', ctrl: true }, false)).toBe('edit.copy');
		expect(resolveShortcut({ key: 'v', ctrl: true }, false)).toBe('edit.paste');
		expect(resolveShortcut({ key: '1', ctrl: true }, false)).toBe('view.top');
		expect(resolveShortcut({ key: 'Delete' }, false)).toBe('edit.deleteSelected');
		expect(resolveShortcut({ key: 'Escape' }, false)).toBe('edit.deselectAll');
		expect(resolveShortcut({ key: '?' }, false)).toBe('help.shortcuts');
	});

	test('touches simples → outils/plateau, seulement hors champ éditable', () => {
		expect(resolveShortcut({ key: 'm' }, false)).toBe('move');
		expect(resolveShortcut({ key: 'c' }, false)).toBe('cut');
		expect(resolveShortcut({ key: 'a' }, false)).toBe('arrange');
		expect(resolveShortcut({ key: 'q' }, false)).toBe('orient');
	});

	test('isolation : aucun raccourci quand un champ est focalisé', () => {
		expect(resolveShortcut({ key: 'm' }, true)).toBeNull();
		expect(resolveShortcut({ key: 's', ctrl: true }, true)).toBeNull();
		expect(resolveShortcut({ key: 'Delete' }, true)).toBeNull();
	});

	test('touche non mappée → null', () => {
		expect(resolveShortcut({ key: 'z' }, false)).toBeNull();
		expect(resolveShortcut({ key: 'n', ctrl: true }, false)).toBeNull(); // réservé navigateur
	});

	test('isToolAction distingue outils et actions de menu/plateau', () => {
		expect(isToolAction('move')).toBe(true);
		expect(isToolAction('cut')).toBe(true);
		expect(isToolAction('edit.copy')).toBe(false);
		expect(isToolAction('arrange')).toBe(false);
		expect(isToolAction('orient')).toBe(false);
	});
});
