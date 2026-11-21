// Tests de l'orchestrateur de raccourcis clavier de l'éditeur (T114).
import { describe, expect, test } from 'vitest';
import {
	chordString,
	resolveShortcut,
	resolveNudge,
	extruderFromAction,
	isToolAction,
	isSaveChord,
	shortcutKeyFor,
	withShortcut
} from './shortcuts';
import { readFileSync } from 'node:fs';

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
		expect(resolveShortcut({ key: 'S', ctrl: true, shift: true }, false)).toBe('project.saveAs');
		expect(resolveShortcut({ key: 'g', ctrl: true }, false)).toBe('export.plate');
		expect(resolveShortcut({ key: 'G', ctrl: true, shift: true }, false)).toBe('export.print');
		expect(resolveShortcut({ key: 'p', meta: true }, false)).toBe('app.preferences');
		expect(resolveShortcut({ key: 'c', ctrl: true }, false)).toBe('edit.copy');
		expect(resolveShortcut({ key: 'v', ctrl: true }, false)).toBe('edit.paste');
		expect(resolveShortcut({ key: '1', ctrl: true }, false)).toBe('view.top');
		expect(resolveShortcut({ key: 'Delete' }, false)).toBe('edit.deleteSelected');
		expect(resolveShortcut({ key: 'Escape' }, false)).toBe('edit.deselectAll');
		expect(resolveShortcut({ key: '?' }, false)).toBe('help.shortcuts');
	});

	test('Ctrl/Cmd+S : unique voie = SaveControls (pas de double save via le dispatcher)', () => {
		// Le dispatcher ne résout jamais la sauvegarde simple…
		expect(resolveShortcut({ key: 's', ctrl: true }, false)).toBeNull();
		expect(resolveShortcut({ key: 's', meta: true }, false)).toBeNull();
		expect(resolveShortcut({ key: 'S', ctrl: true }, false)).toBeNull();
		// …c'est isSaveChord (utilisé par SaveControls) qui la reconnaît.
		expect(isSaveChord({ key: 's', ctrl: true })).toBe(true);
		expect(isSaveChord({ key: 's', meta: true })).toBe(true);
		expect(isSaveChord({ key: 'S', ctrl: true })).toBe(true);
	});

	test('isSaveChord ignore Ctrl+Shift+S (Enregistrer sous) et les variantes', () => {
		expect(isSaveChord({ key: 'S', ctrl: true, shift: true })).toBe(false);
		expect(isSaveChord({ key: 's', ctrl: true, alt: true })).toBe(false);
		expect(isSaveChord({ key: 's' })).toBe(false);
		expect(isSaveChord({ key: 'o', ctrl: true })).toBe(false);
	});

	test('Ctrl+S et Ctrl+Shift+S : jamais deux propriétaires pour un même accord', () => {
		for (const chord of [
			{ key: 's', ctrl: true },
			{ key: 'S', ctrl: true, shift: true },
			{ key: 's', meta: true },
			{ key: 'S', meta: true, shift: true }
		]) {
			const owners = Number(isSaveChord(chord)) + Number(resolveShortcut(chord, false) !== null);
			expect(owners).toBe(1);
		}
	});

	test('touches simples → outils/plateau, seulement hors champ éditable', () => {
		expect(resolveShortcut({ key: 'm' }, false)).toBe('move');
		expect(resolveShortcut({ key: 'c' }, false)).toBe('cut');
		expect(resolveShortcut({ key: 'a' }, false)).toBe('arrange');
		expect(resolveShortcut({ key: 'q' }, false)).toBe('orient');
		expect(resolveShortcut({ key: 'i' }, false)).toBe('view.zoomIn');
		expect(resolveShortcut({ key: 'o' }, false)).toBe('view.zoomOut');
		expect(resolveShortcut({ key: 'v' }, false)).toBe('object.togglePrintable');
		expect(resolveShortcut({ key: 'Tab', shift: true }, false)).toBe('view.toggleSidebar');
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

	test('audit 41 S1 : Alt+N = Nouveau projet (remplace Ctrl+N, réservé)', () => {
		expect(resolveShortcut({ key: 'n', alt: true }, false)).toBe('project.new');
		expect(resolveShortcut({ key: 'n', alt: true }, true)).toBeNull();
	});

	test('isToolAction distingue outils et actions de menu/plateau', () => {
		expect(isToolAction('move')).toBe(true);
		expect(isToolAction('cut')).toBe(true);
		expect(isToolAction('edit.copy')).toBe(false);
		expect(isToolAction('arrange')).toBe(false);
		expect(isToolAction('orient')).toBe(false);
	});
});

// Parité KBShortcutsDialog d'Orca (audit/ui_inventory.json › keyboard_shortcuts).
describe('shortcuts : parité Orca (groupes Plater / Objects List / Global)', () => {
	test('« ? » saisi avec Maj (dispositions réelles) ouvre la liste des raccourcis', () => {
		expect(resolveShortcut({ key: '?', shift: true }, false)).toBe('help.shortcuts');
		expect(chordString({ key: '?', shift: true })).toBe('?');
		// Maj reste significatif pour les lettres.
		expect(chordString({ key: 'A', shift: true })).toBe('shift+a');
	});

	test('Maj+A arrange, Maj+Q oriente tout le plateau actif', () => {
		expect(resolveShortcut({ key: 'A', shift: true }, false)).toBe('arrange');
		expect(resolveShortcut({ key: 'Q', shift: true }, false)).toBe('plate.orientAll');
		expect(isToolAction('plate.orientAll')).toBe(false);
	});

	test('Ctrl+R tranche, Tab bascule Préparer/Aperçu, Espace renomme', () => {
		expect(resolveShortcut({ key: 'r', ctrl: true }, false)).toBe('slice.plate');
		expect(resolveShortcut({ key: 'Tab' }, false)).toBe('view.switchTab');
		expect(resolveShortcut({ key: ' ' }, false)).toBe('object.renameSelected');
		// R seul reste le gizmo de rotation.
		expect(resolveShortcut({ key: 'r' }, false)).toBe('rotate');
	});

	test('1–9 : extrudeur de la sélection ; 0 et Ctrl+chiffre ne sont pas des filaments', () => {
		for (let n = 1; n <= 9; n++) {
			const action = resolveShortcut({ key: String(n) }, false);
			expect(action).toBe(`object.extruder.${n}`);
			expect(extruderFromAction(action!)).toBe(n);
		}
		expect(resolveShortcut({ key: '0' }, false)).toBeNull();
		expect(resolveShortcut({ key: '3', ctrl: true }, false)).toBe('view.front');
		expect(extruderFromAction('object.extruder.10')).toBeNull();
		expect(extruderFromAction('edit.copy')).toBeNull();
	});

	test('flèches : 10 mm, 1 mm avec Maj ; Ctrl/Alt et champ éditable ignorés', () => {
		expect(resolveNudge({ key: 'ArrowUp' }, false)).toEqual([0, 10]);
		expect(resolveNudge({ key: 'ArrowDown' }, false)).toEqual([0, -10]);
		expect(resolveNudge({ key: 'ArrowLeft' }, false)).toEqual([-10, 0]);
		expect(resolveNudge({ key: 'ArrowRight' }, false)).toEqual([10, 0]);
		expect(resolveNudge({ key: 'ArrowRight', shift: true }, false)).toEqual([1, 0]);
		expect(resolveNudge({ key: 'ArrowUp', ctrl: true }, false)).toBeNull();
		expect(resolveNudge({ key: 'ArrowUp', alt: true }, false)).toBeNull();
		expect(resolveNudge({ key: 'ArrowUp' }, true)).toBeNull();
		expect(resolveNudge({ key: 'a' }, false)).toBeNull();
	});

	test('champ éditable : aucun des nouveaux raccourcis n’est capté', () => {
		for (const key of ['1', ' ', 'Tab']) expect(resolveShortcut({ key }, true)).toBeNull();
		expect(resolveShortcut({ key: 'r', ctrl: true }, true)).toBeNull();
	});

	test('chaque raccourci clavier d’Orca réalisable est résolu (inventaire)', () => {
		const inv = JSON.parse(
			readFileSync(new URL('../../../../audit/ui_inventory.json', import.meta.url), 'utf8')
		) as { keyboard_shortcuts: { group: string; shortcuts: { keys: string }[] }[] };
		// Accord Orca → accord DOM ; `null` = hors clavier (souris), réservé par le
		// navigateur, ou propriété d'un autre composant (Ctrl+S → SaveControls).
		const toChord = (keys: string) => {
			const parts = keys.split('+');
			const key = parts.pop()!;
			return {
				key: key === 'Del' ? 'Delete' : key,
				ctrl: parts.includes('Ctrl'),
				shift: parts.includes('Shift')
			};
		};
		const plater = inv.keyboard_shortcuts
			.find((g) => g.group === 'Plater')!
			.shortcuts.map((s) => s.keys);
		const expected = [
			'A',
			'Shift+A',
			'Q',
			'Shift+Q',
			'Esc',
			'Ctrl+0',
			'Ctrl+1',
			'Ctrl+2',
			'Ctrl+3',
			'Ctrl+4',
			'Ctrl+5',
			'Ctrl+6',
			'Ctrl+A',
			'Ctrl+D',
			'Ctrl+Z',
			'Ctrl+Y',
			'M',
			'R',
			'S',
			'F',
			'C',
			'B',
			'H',
			'P',
			'T',
			'U',
			'Y',
			'E',
			'Shift+Tab',
			'I',
			'O',
			'V',
			'Tab'
		];
		for (const keys of expected) {
			expect(plater, keys).toContain(keys);
			expect(resolveShortcut(toChord(keys), false), keys).not.toBeNull();
		}
		const global = inv.keyboard_shortcuts.find((g) => g.group === 'Global shortcuts')!.shortcuts;
		for (const keys of [
			'Ctrl+O',
			'Ctrl+Shift+S',
			'Ctrl+I',
			'Ctrl+G',
			'Ctrl+R',
			'Ctrl+X',
			'Ctrl+C',
			'Ctrl+V',
			'Ctrl+P',
			'Del'
		]) {
			expect(
				global.map((s) => s.keys),
				keys
			).toContain(keys);
			expect(resolveShortcut(toChord(keys), false), keys).not.toBeNull();
		}
	});
});

describe('shortcutKeyFor / withShortcut (info-bulles du rail)', () => {
	test('annonce la touche câblée au format Orca', () => {
		expect(shortcutKeyFor('move')).toBe('M');
		expect(shortcutKeyFor('arrange')).toBe('A');
		expect(shortcutKeyFor('brim-ears')).toBe('E');
	});
	test('aucun raccourci inventé pour un outil non câblé', () => {
		expect(shortcutKeyFor('simplify')).toBeNull();
		expect(withShortcut('Simplifier le modèle', null)).toBe('Simplifier le modèle');
		expect(withShortcut('Déplacer', 'M')).toBe('Déplacer (M)');
	});
});
