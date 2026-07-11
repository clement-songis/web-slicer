// Tests des menus principaux et de l'aide raccourcis (T079, Annexe B §B.2/B.6).
import { describe, expect, test } from 'bun:test';
import { MAIN_MENUS, allMenuLabels, menuLabels } from './menus';
import { SHORTCUT_GROUPS, totalShortcuts } from './shortcuts';

describe('MAIN_MENUS', () => {
	test('exposes File / Edit / View / Help (Annexe B §B.2)', () => {
		expect(MAIN_MENUS.map((m) => m.title)).toEqual(['File', 'Edit', 'View', 'Help']);
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
