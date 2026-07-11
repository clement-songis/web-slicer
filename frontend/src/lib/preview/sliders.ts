// Curseurs de prévisualisation (T069, Annexe B « Preview ») : curseur vertical
// de plage de couches (deux poignées low/high, mode une-couche), curseur
// horizontal de progression intra-couche (moves de la couche du haut), et
// raccourcis clavier du groupe Preview. Logique pure et testable (plages
// d'index) : les composants n'y ajoutent aucune règle.

import type { Shortcut } from '../scene/menus';

/** Pas rapide (Shift/Ctrl + flèche/molette) : ×5 (Annexe B). */
export const FAST_STEP = 5;

const clamp = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, v));

// --- Curseur vertical : plage de couches -------------------------------------

/** Poignée active du curseur vertical. */
export type Thumb = 'low' | 'high';

/** État du curseur vertical (plage de couches visibles, incluses). */
export interface LayerRange {
	/** Nombre total de couches. */
	total: number;
	/** Couche basse visible (incluse). */
	low: number;
	/** Couche haute visible (incluse). */
	high: number;
	/** Poignée déplacée par les flèches. */
	active: Thumb;
	/** Mode « une couche » (`L`) : la plage suit une couche unique. */
	oneLayerMode: boolean;
}

/** Initialise la plage sur toutes les couches (poignée haute active). */
export function makeLayerRange(total: number): LayerRange {
	const last = Math.max(0, total - 1);
	return { total, low: 0, high: last, active: 'high', oneLayerMode: false };
}

/** Sélectionne la poignée active. */
export function setActiveThumb(range: LayerRange, active: Thumb): LayerRange {
	return { ...range, active };
}

/**
 * Déplace la poignée active de `delta` couches (bornée). En mode une-couche,
 * les deux poignées se déplacent ensemble (couche unique). Sinon, `low` ne
 * dépasse jamais `high` et réciproquement.
 */
export function moveThumb(range: LayerRange, delta: number): LayerRange {
	const last = Math.max(0, range.total - 1);
	if (range.oneLayerMode) {
		const layer = clamp(range.high + delta, 0, last);
		return { ...range, low: layer, high: layer };
	}
	if (range.active === 'low') {
		const low = clamp(range.low + delta, 0, range.high);
		return { ...range, low };
	}
	const high = clamp(range.high + delta, range.low, last);
	return { ...range, high };
}

/**
 * Bascule le mode une-couche. À l'activation, la plage se réduit à la couche
 * haute courante ; à la désactivation, la couche basse repart de 0.
 */
export function toggleOneLayerMode(range: LayerRange): LayerRange {
	if (range.oneLayerMode) {
		return { ...range, oneLayerMode: false, low: 0, active: 'high' };
	}
	return { ...range, oneLayerMode: true, low: range.high, active: 'high' };
}

/** Indices de couches à afficher (bornes incluses), pour `?from&to`. */
export function visibleRange(range: LayerRange): { from: number; to: number } {
	return { from: range.low, to: range.high };
}

// --- Curseur horizontal : progression intra-couche ---------------------------

/** État du curseur horizontal : nombre de segments affichés de la couche haute. */
export interface MoveCursor {
	/** Nombre de segments (moves) de la couche du haut. */
	total: number;
	/** Segments affichés depuis le début `[0, total]`. */
	position: number;
}

/** Curseur plein (toute la couche affichée) sur `total` segments. */
export function makeMoveCursor(total: number): MoveCursor {
	return { total, position: total };
}

/** Déplace le curseur de `delta` segments (borné). */
export function moveCursorBy(cursor: MoveCursor, delta: number): MoveCursor {
	return { ...cursor, position: clamp(cursor.position + delta, 0, cursor.total) };
}

/** Position de départ (`Home`) : aucun segment affiché. */
export function moveCursorToStart(cursor: MoveCursor): MoveCursor {
	return { ...cursor, position: 0 };
}

/** Position finale (`End`) : couche entière affichée. */
export function moveCursorToEnd(cursor: MoveCursor): MoveCursor {
	return { ...cursor, position: cursor.total };
}

/** Réinitialise le curseur sur une nouvelle couche (change de nombre de moves). */
export function retargetCursor(total: number): MoveCursor {
	return makeMoveCursor(total);
}

// --- Raccourcis clavier du groupe Preview ------------------------------------

/** Raccourcis du groupe « Preview » (Annexe B §B.6), verbatim. */
export const PREVIEW_SHORTCUTS: Shortcut[] = [
	{ keys: 'Arrow Up', action: 'Vertical slider - Move active thumb Up' },
	{ keys: 'Arrow Down', action: 'Vertical slider - Move active thumb Down' },
	{ keys: 'Arrow Left', action: 'Horizontal slider - Move active thumb Left' },
	{ keys: 'Arrow Right', action: 'Horizontal slider - Move active thumb Right' },
	{ keys: 'L', action: 'On/Off one layer mode of the vertical slider' },
	{ keys: 'C', action: 'On/Off G-code window' },
	{ keys: 'Tab', action: 'Switch between Prepare/Preview' },
	{ keys: 'Shift+Any arrow', action: 'Move slider 5x faster' },
	{ keys: 'Shift+Mouse wheel', action: 'Move slider 5x faster' },
	{ keys: 'Ctrl+Any arrow', action: 'Move slider 5x faster' },
	{ keys: 'Ctrl+Mouse wheel', action: 'Move slider 5x faster' },
	{ keys: 'Home', action: 'Horizontal slider - Move to start position' },
	{ keys: 'End', action: 'Horizontal slider - Move to last position' }
];

/** Action de prévisualisation résolue depuis une touche. */
export type PreviewAction =
	| { type: 'thumb'; dir: 'up' | 'down'; fast: boolean }
	| { type: 'cursor'; dir: 'left' | 'right'; fast: boolean }
	| { type: 'cursor-start' }
	| { type: 'cursor-end' }
	| { type: 'toggle-one-layer' }
	| { type: 'toggle-gcode-window' }
	| { type: 'switch-tab' };

/** Entrée clavier minimale (compatible `KeyboardEvent`). */
export interface KeyLike {
	key: string;
	shiftKey?: boolean;
	ctrlKey?: boolean;
}

/** Résout une touche du groupe Preview en action (ou `null`). */
export function resolvePreviewKey(e: KeyLike): PreviewAction | null {
	const fast = Boolean(e.shiftKey || e.ctrlKey);
	switch (e.key) {
		case 'ArrowUp':
			return { type: 'thumb', dir: 'up', fast };
		case 'ArrowDown':
			return { type: 'thumb', dir: 'down', fast };
		case 'ArrowLeft':
			return { type: 'cursor', dir: 'left', fast };
		case 'ArrowRight':
			return { type: 'cursor', dir: 'right', fast };
		case 'Home':
			return { type: 'cursor-start' };
		case 'End':
			return { type: 'cursor-end' };
		case 'l':
		case 'L':
			return { type: 'toggle-one-layer' };
		case 'c':
		case 'C':
			return { type: 'toggle-gcode-window' };
		case 'Tab':
			return { type: 'switch-tab' };
		default:
			return null;
	}
}

// --- État combiné + réducteur ------------------------------------------------

/** État de prévisualisation piloté au clavier. */
export interface PreviewState {
	range: LayerRange;
	cursor: MoveCursor;
	/** Fenêtre G-code ouverte (`C`). */
	gcodeWindow: boolean;
}

/** Initialise l'état (plage complète, curseur plein, fenêtre G-code fermée). */
export function makePreviewState(layerCount: number, topLayerSegments: number): PreviewState {
	return {
		range: makeLayerRange(layerCount),
		cursor: makeMoveCursor(topLayerSegments),
		gcodeWindow: false
	};
}

/**
 * Applique une action clavier à l'état. `switch-tab` est externe (bascule
 * Prepare/Preview) : l'état est renvoyé inchangé, au composant de router.
 */
export function applyPreviewKey(state: PreviewState, e: KeyLike): PreviewState {
	const action = resolvePreviewKey(e);
	if (!action) return state;
	const step = (fast: boolean) => (fast ? FAST_STEP : 1);
	switch (action.type) {
		case 'thumb':
			return {
				...state,
				range: moveThumb(state.range, action.dir === 'up' ? step(action.fast) : -step(action.fast))
			};
		case 'cursor':
			return {
				...state,
				cursor: moveCursorBy(
					state.cursor,
					action.dir === 'right' ? step(action.fast) : -step(action.fast)
				)
			};
		case 'cursor-start':
			return { ...state, cursor: moveCursorToStart(state.cursor) };
		case 'cursor-end':
			return { ...state, cursor: moveCursorToEnd(state.cursor) };
		case 'toggle-one-layer':
			return { ...state, range: toggleOneLayerMode(state.range) };
		case 'toggle-gcode-window':
			return { ...state, gcodeWindow: !state.gcodeWindow };
		case 'switch-tab':
			return state;
	}
}
