import { describe, expect, it } from 'vitest';
import { SHORTCUT_GROUPS } from './shortcuts';
import { SHORTCUTS_FR, shortcutKeysText, shortcutText } from './shortcuts-fr';

describe('aide raccourcis en français (phase 26)', () => {
	it('chaque description et chaque groupe a une traduction Orca', () => {
		for (const g of SHORTCUT_GROUPS) {
			// « Gizmo » est identique en français (msgstr = msgid).
			if (g.group !== 'Gizmo') expect(SHORTCUTS_FR[g.group], g.group).toBeTruthy();
			for (const s of g.shortcuts) expect(SHORTCUTS_FR[s.action], s.action).toBeTruthy();
		}
	});

	it('textes Orca (msgstr fr) et repli anglais', () => {
		expect(shortcutText('New Project', 'fr')).toBe('Nouveau projet');
		expect(shortcutText('Arrange all objects', 'en')).toBe('Arrange all objects');
		expect(shortcutText('inconnu', 'fr')).toBe('inconnu');
	});

	it('touches composées traduites touche par touche', () => {
		expect(shortcutKeysText('Ctrl+Left mouse button', 'fr')).toBe(
			'Ctrl+' + SHORTCUTS_FR['Left mouse button']
		);
		expect(shortcutKeysText('Arrow Up', 'fr')).toBe(SHORTCUTS_FR['Arrow Up']);
		expect(shortcutKeysText('Ctrl+Shift+S', 'fr')).toBe('Ctrl+Shift+S');
		expect(shortcutKeysText('?', 'fr')).toBe('?');
		expect(shortcutKeysText('Ctrl+Left mouse button', 'en')).toBe('Ctrl+Left mouse button');
	});
});
