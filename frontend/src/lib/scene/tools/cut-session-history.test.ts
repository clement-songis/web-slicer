// Audit 40 B2 : annuler / rétablir propres à l'outil Coupe (Orca GizmoAction).
import { describe, expect, test } from 'vitest';
import { CutSession } from './cut-session.svelte';

function note(s: CutSession, t: number) {
	s.noteChange(s.historyKey(), t);
}

describe('historique de l’outil Coupe', () => {
	test('chaque connecteur ajouté est une étape ; Ctrl+Z les retire un par un, Ctrl+Y les remet', () => {
		const s = new CutSession();
		note(s, 0);
		s.pickUv(1, 1);
		note(s, 1000);
		s.pickUv(5, 5);
		note(s, 2000);
		expect(s.connectors).toHaveLength(2);
		expect(s.undoCount).toBe(2);
		expect(s.undo()).toBe(true);
		note(s, 2100);
		expect(s.connectors).toHaveLength(1);
		expect(s.undo()).toBe(true);
		note(s, 2200);
		expect(s.connectors).toHaveLength(0);
		expect(s.undo()).toBe(false);
		expect(s.redo()).toBe(true);
		note(s, 2300);
		expect(s.connectors).toHaveLength(1);
		expect(s.redoCount).toBe(1);
	});
	test('un glisser continu (rafale < 400 ms) ne fait qu’une étape', () => {
		const s = new CutSession();
		note(s, 0);
		s.pickUv(1, 1);
		note(s, 1000);
		const id = s.connectors[0].id;
		for (let i = 1; i <= 10; i++) {
			s.moveConnector(id, 1 + i, 1);
			note(s, 3000 + i * 50);
		}
		expect(s.undoCount).toBe(2);
		s.undo();
		expect(s.connectors[0].u).toBe(1);
	});
	test('le déplacement du plan est annulable ; une nouvelle action vide le rétablissement', () => {
		const s = new CutSession();
		note(s, 0);
		s.offset = 4;
		note(s, 1000);
		s.undo();
		note(s, 1100);
		expect(s.offset).toBe(0);
		s.offset = 2;
		note(s, 3000);
		expect(s.redoCount).toBe(0);
		expect(s.redo()).toBe(false);
	});
	test('changer d’objet oublie l’historique', () => {
		const s = new CutSession();
		note(s, 0);
		s.pickUv(1, 1);
		note(s, 1000);
		s.clearHistory();
		expect(s.undo()).toBe(false);
	});
});
