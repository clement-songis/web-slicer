// Tests des curseurs de prévisualisation (T069) : plages d'index du curseur
// vertical (couches) et horizontal (moves intra-couche), mode une-couche,
// pas rapide, et résolution des raccourcis du groupe Preview.
import { describe, expect, test } from 'vitest';
import {
	makeLayerRange,
	moveThumb,
	setActiveThumb,
	toggleOneLayerMode,
	visibleRange,
	makeMoveCursor,
	moveCursorBy,
	moveCursorToStart,
	moveCursorToEnd,
	resolvePreviewKey,
	applyPreviewKey,
	makePreviewState,
	PREVIEW_SHORTCUTS,
	FAST_STEP
} from './sliders';

describe('LayerRange (curseur vertical)', () => {
	test('starts spanning every layer with the high thumb active', () => {
		const r = makeLayerRange(50);
		expect(visibleRange(r)).toEqual({ from: 0, to: 49 });
		expect(r.active).toBe('high');
	});

	test('moves the high thumb without crossing below low', () => {
		let r = makeLayerRange(10);
		r = moveThumb(r, -3); // high 9 → 6
		expect(visibleRange(r)).toEqual({ from: 0, to: 6 });
		r = setActiveThumb(r, 'low');
		r = moveThumb(r, 8); // low bloqué à high (6)
		expect(r.low).toBe(6);
	});

	test('clamps thumbs to [0, total-1]', () => {
		let r = makeLayerRange(5);
		r = moveThumb(r, 100);
		expect(r.high).toBe(4);
		r = setActiveThumb(r, 'low');
		r = moveThumb(r, -100);
		expect(r.low).toBe(0);
	});

	test('one-layer mode collapses to a single layer and moves both thumbs', () => {
		let r = makeLayerRange(20); // high = 19
		r = toggleOneLayerMode(r);
		expect(visibleRange(r)).toEqual({ from: 19, to: 19 });
		r = moveThumb(r, -5);
		expect(visibleRange(r)).toEqual({ from: 14, to: 14 });
		r = toggleOneLayerMode(r); // désactivation : low repart de 0
		expect(visibleRange(r)).toEqual({ from: 0, to: 14 });
	});

	test('empty model yields a single degenerate layer', () => {
		const r = makeLayerRange(0);
		expect(visibleRange(r)).toEqual({ from: 0, to: 0 });
	});
});

describe('MoveCursor (curseur horizontal)', () => {
	test('is full by default and clamps to [0, total]', () => {
		let c = makeMoveCursor(120);
		expect(c.position).toBe(120);
		c = moveCursorBy(c, 50);
		expect(c.position).toBe(120);
		c = moveCursorBy(c, -1000);
		expect(c.position).toBe(0);
	});

	test('Home and End jump to the bounds', () => {
		const c = makeMoveCursor(80);
		expect(moveCursorToStart(c).position).toBe(0);
		expect(moveCursorToEnd(moveCursorToStart(c)).position).toBe(80);
	});
});

describe('resolvePreviewKey', () => {
	test('maps arrows to thumb / cursor with fast flag', () => {
		expect(resolvePreviewKey({ key: 'ArrowUp' })).toEqual({
			type: 'thumb',
			dir: 'up',
			fast: false
		});
		expect(resolvePreviewKey({ key: 'ArrowRight', shiftKey: true })).toEqual({
			type: 'cursor',
			dir: 'right',
			fast: true
		});
		expect(resolvePreviewKey({ key: 'ArrowDown', ctrlKey: true })).toEqual({
			type: 'thumb',
			dir: 'down',
			fast: true
		});
	});

	test('maps L/C/Tab/Home/End', () => {
		expect(resolvePreviewKey({ key: 'l' })).toEqual({ type: 'toggle-one-layer' });
		expect(resolvePreviewKey({ key: 'C' })).toEqual({ type: 'toggle-gcode-window' });
		expect(resolvePreviewKey({ key: 'Tab' })).toEqual({ type: 'switch-tab' });
		expect(resolvePreviewKey({ key: 'Home' })).toEqual({ type: 'cursor-start' });
		expect(resolvePreviewKey({ key: 'End' })).toEqual({ type: 'cursor-end' });
	});

	test('returns null for unrelated keys', () => {
		expect(resolvePreviewKey({ key: 'z' })).toBeNull();
	});
});

describe('applyPreviewKey', () => {
	test('fast step moves the thumb 5 layers at once', () => {
		const state = makePreviewState(100, 40);
		const moved = applyPreviewKey(state, { key: 'ArrowDown', shiftKey: true });
		expect(moved.range.high).toBe(99 - FAST_STEP);
	});

	test('arrow-right advances the intra-layer cursor', () => {
		let state = makePreviewState(10, 30);
		state = { ...state, cursor: moveCursorToStart(state.cursor) };
		state = applyPreviewKey(state, { key: 'ArrowRight' });
		expect(state.cursor.position).toBe(1);
	});

	test('C toggles the G-code window', () => {
		const state = makePreviewState(10, 5);
		expect(applyPreviewKey(state, { key: 'C' }).gcodeWindow).toBe(true);
	});

	test('switch-tab leaves the preview state unchanged', () => {
		const state = makePreviewState(10, 5);
		expect(applyPreviewKey(state, { key: 'Tab' })).toEqual(state);
	});
});

describe('PREVIEW_SHORTCUTS', () => {
	test('mirrors the 13 Annexe B Preview shortcuts', () => {
		expect(PREVIEW_SHORTCUTS).toHaveLength(13);
		expect(PREVIEW_SHORTCUTS[0]).toEqual({
			keys: 'Arrow Up',
			action: 'Vertical slider - Move active thumb Up'
		});
	});
});
