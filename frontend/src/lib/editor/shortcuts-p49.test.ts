// Audit 47 KEY-a : N (peinture couleur) et L (peinture de supports) comme Orca
// (GLGizmoMmuSegmentation.cpp:90, GLGizmoFdmSupports.cpp:81, touches simples de
// GLGizmosManager::handle_shortcut).
import { describe, expect, test } from 'vitest';
import { resolveShortcut, shortcutKeyFor } from './shortcuts';

describe('KEY-a', () => {
	test('N → peinture couleur, L → peinture de supports', () => {
		expect(resolveShortcut({ key: 'n' }, false)).toBe('mm-paint');
		expect(resolveShortcut({ key: 'N' }, false)).toBe('mm-paint');
		expect(resolveShortcut({ key: 'l' }, false)).toBe('support-paint');
	});
	test('inactives dans un champ ; Alt+N reste « Nouveau projet »', () => {
		expect(resolveShortcut({ key: 'n' }, true)).toBeNull();
		expect(resolveShortcut({ key: 'n', alt: true }, false)).toBe('project.new');
	});
	test('info-bulles du rail « Nom (N) » / « Nom (L) »', () => {
		expect(shortcutKeyFor('mm-paint')).toBe('N');
		expect(shortcutKeyFor('support-paint')).toBe('L');
	});
});
