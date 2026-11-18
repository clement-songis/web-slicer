// Tests des menus principaux et de l'aide raccourcis (T079, Annexe B §B.2/B.6).
import { describe, expect, test } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { MAIN_MENUS, allMenuLabels, menuLabels } from './menus';
import { SHORTCUT_GROUPS, totalShortcuts } from './shortcuts';
import type { MenuItem } from './menus';
import { isSaveChord, resolveShortcut, type KeyChord } from '../editor/shortcuts';

/** Convertit un raccourci affiché (« Ctrl+Shift+S », « Del ») en accord clavier. */
function parseDisplayed(shortcut: string): KeyChord {
	const parts = shortcut.split('+');
	const key = parts.pop()!;
	const mods = new Set(parts.map((p) => p.toLowerCase()));
	return {
		key: key === 'Del' ? 'Delete' : key === 'Esc' ? 'Escape' : key.toLowerCase(),
		ctrl: mods.has('ctrl'),
		shift: mods.has('shift'),
		alt: mods.has('alt')
	};
}

/** Tous les items (feuilles + sous-items) portant un raccourci affiché. */
function itemsWithShortcut(): MenuItem[] {
	const out: MenuItem[] = [];
	for (const menu of MAIN_MENUS) {
		for (const entry of menu.entries) {
			if (entry === 'separator') continue;
			for (const it of [entry, ...(entry.items ?? [])]) if (it.shortcut) out.push(it);
		}
	}
	return out;
}

// Accélérateurs affichés mais non interceptables (exclusions.md) : le navigateur
// les capte ; l'item de menu reste fonctionnel au clic.
const BROWSER_RESERVED = new Set(['Ctrl+N']);
// `C` (fenêtre G-code) n'existe qu'en Aperçu, résolu par preview/sliders.ts.
const PREVIEW_ONLY = new Set(['C']);

describe('MAIN_MENUS', () => {
	test('exposes File / Edit / View / Help (Annexe B §B.2)', () => {
		expect(MAIN_MENUS.map((m) => m.title)).toEqual(['File', 'Edit', 'View', 'Calibration', 'Help']);
	});

	test('File menu carries the core project + import/export actions', () => {
		const file = MAIN_MENUS.find((m) => m.title === 'File')!;
		const labels = menuLabels(file);
		for (const expected of [
			'New Project',
			'Open Project…',
			'Save Project',
			'Import 3MF/STL/STEP/SVG/OBJ/AMF…',
			'Export G-code…',
			'Preferences'
		]) {
			expect(labels).toContain(expected);
		}
	});

	test('Edit menu covers undo/redo, clipboard and selection', () => {
		const edit = MAIN_MENUS.find((m) => m.title === 'Edit')!;
		const labels = menuLabels(edit);
		for (const expected of ['Undo', 'Redo', 'Cut', 'Copy', 'Paste', 'Select all', 'Deselect all']) {
			expect(labels).toContain(expected);
		}
	});

	test('View menu has the seven camera views and display toggles', () => {
		const view = MAIN_MENUS.find((m) => m.title === 'View')!;
		const labels = menuLabels(view);
		for (const expected of ['Default View', 'Top', 'Bottom', 'Front', 'Rear', 'Left', 'Right']) {
			expect(labels).toContain(expected);
		}
	});

	test('every leaf action id is unique', () => {
		const ids: string[] = [];
		for (const menu of MAIN_MENUS) {
			for (const entry of menu.entries) {
				if (entry === 'separator') continue;
				ids.push(entry.action);
				for (const sub of entry.items ?? []) ids.push(sub.action);
			}
		}
		expect(new Set(ids).size).toBe(ids.length);
	});

	test('every displayed shortcut has exactly one keyboard owner for its action', () => {
		for (const item of itemsWithShortcut()) {
			const shown = item.shortcut!;
			if (BROWSER_RESERVED.has(shown) || PREVIEW_ONLY.has(shown)) continue;
			const chord = parseDisplayed(shown);
			const resolved = resolveShortcut(chord, false);
			if (item.action === 'project.save') {
				// Ctrl+S : SaveControls seul ; le dispatcher ne doit pas le doubler.
				expect(isSaveChord(chord), shown).toBe(true);
				expect(resolved, shown).toBeNull();
			} else {
				expect(resolved, `${shown} → ${item.action}`).toBe(item.action);
				expect(isSaveChord(chord), shown).toBe(false);
			}
		}
	});

	test('Save / Save as display distinct, non-colliding chords', () => {
		const byAction = Object.fromEntries(itemsWithShortcut().map((i) => [i.action, i.shortcut]));
		expect(byAction['project.save']).toBe('Ctrl+S');
		expect(byAction['project.saveAs']).toBe('Ctrl+Shift+S');
		expect(isSaveChord(parseDisplayed('Ctrl+Shift+S'))).toBe(false);
		expect(resolveShortcut(parseDisplayed('Ctrl+Shift+S'), false)).toBe('project.saveAs');
	});

	test('allMenuLabels is non-empty and includes Keyboard Shortcuts', () => {
		expect(allMenuLabels()).toContain('Keyboard Shortcuts');
	});
});

describe('SHORTCUT_GROUPS', () => {
	test('lists all 92 shortcuts (Annexe B §B.6)', () => {
		expect(totalShortcuts()).toBe(92);
	});

	test('covers the five inventory groups with exact counts', () => {
		const counts = Object.fromEntries(SHORTCUT_GROUPS.map((g) => [g.group, g.shortcuts.length]));
		expect(counts).toEqual({
			'Global shortcuts': 18,
			Plater: 45,
			Gizmo: 4,
			'Objects List': 12,
			Preview: 13
		});
	});

	test('the help shortcut ? is present in the global group', () => {
		const global = SHORTCUT_GROUPS.find((g) => g.group === 'Global shortcuts')!;
		expect(global.shortcuts.some((s) => s.keys === '?')).toBe(true);
	});
});

describe('menu Calibration : libellés clarifiés (phase 25)', () => {
	const calib = MAIN_MENUS.find((m) => m.title === 'Calibration')!;
	const items = calib.entries.filter((e): e is MenuItem => e !== 'separator');
	const read = (loc: string) =>
		JSON.parse(readFileSync(resolve(import.meta.dirname, `../../../messages/${loc}.json`), 'utf8'));
	test('libellés et actions inchangés (clés de parité ciblées par les specs gelés)', () => {
		expect(items.map((i) => i.label)).toEqual([
			'Temperature',
			'Max flowrate',
			'Pressure advance',
			'Pressure advance pattern',
			'Pressure advance history',
			'Flow ratio',
			'Flow rate (pass 1)',
			'Flow rate (pass 2)',
			'Retraction test',
			'Cornering',
			'Input Shaping Frequency',
			'Input Shaping Damping/zeta factor',
			'VFA',
			'Calibration Guide'
		]);
	});
	test('chaque entrée a une description traduite en français', () => {
		const en = read('en');
		const fr = read('fr');
		const key = (text: string) => Object.keys(en).find((k) => en[k] === text);
		for (const i of items) {
			expect(i.hint, i.label).toBeTruthy();
			expect(fr[key(i.hint!) ?? ''], i.hint).toBeTruthy();
		}
		// Les passes 1/2 sont nommées comme des passes du ratio de débit.
		expect(fr[key('Flow rate (pass 1)')!]).toBe('Ratio de débit : passe 1');
		expect(fr[key('Flow ratio')!]).toBe('Ratio de débit (assistant)');
	});
});

