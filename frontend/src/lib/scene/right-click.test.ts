// Audit 46 PREF-c : glisser au bouton droit → pas de menu (ignore_right_up d'Orca).
import { describe, expect, test } from 'vitest';
import { RightClickGuard } from './right-click';

function setup(moves = true) {
	const g = new RightClickGuard(() => moves);
	let opened = 0;
	const open = () => opened++;
	return { g, open, count: () => opened };
}

describe('RightClickGuard', () => {
	test('clic droit sans déplacement, contextmenu à l’appui : menu au relâchement', () => {
		const { g, open, count } = setup();
		g.pointerDown(2, 10, 10);
		g.contextMenu(open);
		expect(count()).toBe(0);
		g.pointerUp(2);
		expect(count()).toBe(1);
	});
	test('clic droit sans déplacement, contextmenu au relâchement : menu immédiat', () => {
		const { g, open, count } = setup();
		g.pointerDown(2, 10, 10);
		g.pointerUp(2);
		g.contextMenu(open);
		expect(count()).toBe(1);
	});
	test('glisser droit (caméra déplacée) : aucun menu, quel que soit le moment du contextmenu', () => {
		const a = setup();
		a.g.pointerDown(2, 10, 10);
		a.g.contextMenu(a.open);
		a.g.pointerMove(2, 130, 10);
		a.g.pointerUp(2);
		expect(a.count()).toBe(0);
		const b = setup();
		b.g.pointerDown(2, 10, 10);
		b.g.pointerMove(2, 130, 10);
		b.g.pointerUp(2);
		b.g.contextMenu(b.open);
		expect(b.count()).toBe(0);
		// Le clic suivant, sans glisser, rouvre le menu.
		b.g.pointerDown(2, 5, 5);
		b.g.pointerUp(2);
		b.g.contextMenu(b.open);
		expect(b.count()).toBe(1);
	});
	test('bouton droit « Aucun » : la caméra ne bouge pas, le menu s’ouvre (Orca)', () => {
		const { g, open, count } = setup(false);
		g.pointerDown(2, 10, 10);
		g.pointerMove(2, 130, 10);
		g.pointerUp(2);
		g.contextMenu(open);
		expect(count()).toBe(1);
	});
	test('déplacement sans bouton droit enfoncé ignoré', () => {
		const { g, open, count } = setup();
		g.pointerDown(2, 10, 10);
		g.pointerMove(1, 130, 10);
		g.pointerUp(2);
		g.contextMenu(open);
		expect(count()).toBe(1);
	});
});
