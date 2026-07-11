// Tests de l'orchestrateur d'outil du rail (T103).
import { describe, expect, test } from 'bun:test';
import {
	gizmoModeOf,
	initialTools,
	isTransformTool,
	paintChannelOf,
	setTool,
	TOOL_ORDER,
	TRANSFORM_TOOLS
} from './tools';

describe('tools', () => {
	test('défaut : outil Déplacer', () => {
		expect(initialTools().active).toBe('move');
	});

	test('le rail expose les 16 gizmos OrcaSlicer', () => {
		expect(TOOL_ORDER.length).toBe(16);
		// Transformation en tête.
		expect(TOOL_ORDER.slice(0, 3)).toEqual(['move', 'rotate', 'scale']);
	});

	test('setTool sélectionne, re-cliquer désactive (bascule immuable)', () => {
		const a = initialTools({ active: null });
		const b = setTool(a, 'cut');
		expect(b.active).toBe('cut');
		expect(a.active).toBeNull();
		expect(setTool(b, 'cut').active).toBeNull();
	});

	test('isTransformTool + gizmoModeOf', () => {
		expect(TRANSFORM_TOOLS.every((t) => isTransformTool(t))).toBe(true);
		expect(isTransformTool('cut')).toBe(false);
		expect(gizmoModeOf('move')).toBe('translate');
		expect(gizmoModeOf('rotate')).toBe('rotate');
		expect(gizmoModeOf('scale')).toBe('scale');
		expect(gizmoModeOf('cut')).toBeNull();
		expect(gizmoModeOf(null)).toBeNull();
	});

	test('paintChannelOf mappe les quatre outils de peinture', () => {
		expect(paintChannelOf('support-paint')).toBe('supports');
		expect(paintChannelOf('seam-paint')).toBe('seam');
		expect(paintChannelOf('fuzzy-paint')).toBe('fuzzy');
		expect(paintChannelOf('mm-paint')).toBe('mmu');
		expect(paintChannelOf('cut')).toBeNull();
		expect(paintChannelOf(null)).toBeNull();
	});
});
