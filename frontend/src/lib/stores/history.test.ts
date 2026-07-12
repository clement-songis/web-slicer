// Tests du store d'historique undo/redo (T052).
import { describe, expect, test } from 'vitest';
import { History } from './history';

describe('History', () => {
	test("démarre sur l'état initial sans undo/redo", () => {
		const h = new History('a');
		expect(h.current).toBe('a');
		expect(h.canUndo).toBe(false);
		expect(h.canRedo).toBe(false);
	});

	test('push puis undo/redo parcourt les états', () => {
		const h = new History('a');
		h.push('b');
		h.push('c');
		expect(h.current).toBe('c');
		expect(h.canUndo).toBe(true);

		expect(h.undo()).toBe('b');
		expect(h.undo()).toBe('a');
		expect(h.undo()).toBeUndefined();
		expect(h.canUndo).toBe(false);

		expect(h.redo()).toBe('b');
		expect(h.redo()).toBe('c');
		expect(h.redo()).toBeUndefined();
	});

	test('un push après un undo tronque la branche redo', () => {
		const h = new History('a');
		h.push('b');
		h.push('c');
		h.undo(); // → b
		h.push('d'); // nouvelle branche
		expect(h.current).toBe('d');
		expect(h.canRedo).toBe(false);
		expect(h.undo()).toBe('b');
	});

	test("ignore un push identique à l'état courant", () => {
		const h = new History('a');
		h.push('a');
		expect(h.canUndo).toBe(false);
		expect(h.depth.undo).toBe(0);
	});

	test('respecte la limite de profondeur', () => {
		const h = new History('0', 2);
		h.push('1');
		h.push('2');
		h.push('3'); // oublie '0'
		expect(h.depth.undo).toBe(2);
		expect(h.undo()).toBe('2');
		expect(h.undo()).toBe('1');
		expect(h.undo()).toBeUndefined(); // '0' oublié
	});

	test("reset vide les piles autour d'un état", () => {
		const h = new History('a');
		h.push('b');
		h.reset('x');
		expect(h.current).toBe('x');
		expect(h.canUndo).toBe(false);
		expect(h.canRedo).toBe(false);
	});
});
