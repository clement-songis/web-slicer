import { describe, expect, it } from 'vitest';
import { altLetter, menuForMnemonic, menuMnemonic, moveIndex } from './menu-keys';

describe('menus au clavier (audit 42 A2)', () => {
	it('lettre d’accès = première lettre sans accent', () => {
		expect(menuMnemonic('Fichier')).toBe('f');
		expect(menuMnemonic('Édition')).toBe('e');
		expect(menuMnemonic('Aide')).toBe('a');
	});
	it('Alt+lettre ouvre le menu correspondant (FR et EN)', () => {
		const fr = ['Fichier', 'Édition', 'Vue', 'Calibration', 'Aide'];
		expect(menuForMnemonic(fr, 'f')).toBe(0);
		expect(menuForMnemonic(fr, 'e')).toBe(1);
		expect(menuForMnemonic(fr, 'a')).toBe(4);
		expect(menuForMnemonic(fr, 'n')).toBe(-1);
		const en = ['File', 'Edit', 'View', 'Calibration', 'Help'];
		expect(menuForMnemonic(en, 'h')).toBe(4);
	});
	it('altLetter : Alt seul + touche lettre (code physique)', () => {
		const base = { altKey: true, ctrlKey: false, metaKey: false, shiftKey: false };
		expect(altLetter({ ...base, code: 'KeyF' })).toBe('f');
		expect(altLetter({ ...base, code: 'Digit1' })).toBeNull();
		expect(altLetter({ ...base, ctrlKey: true, code: 'KeyF' })).toBeNull();
		expect(altLetter({ ...base, altKey: false, code: 'KeyF' })).toBeNull();
	});
	it('flèches circulaires, Début / Fin', () => {
		expect(moveIndex(-1, 4, 'ArrowDown')).toBe(0);
		expect(moveIndex(3, 4, 'ArrowDown')).toBe(0);
		expect(moveIndex(0, 4, 'ArrowUp')).toBe(3);
		expect(moveIndex(-1, 4, 'ArrowUp')).toBe(3);
		expect(moveIndex(2, 4, 'Home')).toBe(0);
		expect(moveIndex(1, 4, 'End')).toBe(3);
		expect(moveIndex(0, 0, 'ArrowDown')).toBe(-1);
	});
});
