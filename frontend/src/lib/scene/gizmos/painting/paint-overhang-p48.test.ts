// Audit 46 SUPP-a : « Mettre en évidence les zones en surplomb » et
// « Seulement sur les surplombs en évidence » (GLGizmoFdmSupports).
import { describe, expect, test } from 'vitest';
import { PAINT_PARAM_DEFAULTS, strokeFields, supportHighlightAngle } from './paint-tools';

describe('peinture de supports sur les surplombs', () => {
	test('angle initial = seuil des supports auto, sinon 0 (get_selection_support_threshold_angle)', () => {
		expect(
			supportHighlightAngle({
				enable_support: '1',
				support_type: 'normal(auto)',
				support_threshold_angle: '30'
			})
		).toBe(30);
		expect(
			supportHighlightAngle({
				enable_support: true,
				support_type: 'tree(auto)',
				support_threshold_angle: 45
			})
		).toBe(45);
		expect(
			supportHighlightAngle({
				enable_support: '1',
				support_type: 'normal(manual)',
				support_threshold_angle: '30'
			})
		).toBe(0);
		expect(
			supportHighlightAngle({
				enable_support: '0',
				support_type: 'normal(auto)',
				support_threshold_angle: '30'
			})
		).toBe(0);
	});
	test('trait limité aux surplombs : highlight_deg = angle (supports seulement)', () => {
		const p = { ...PAINT_PARAM_DEFAULTS, overhangOnlyDeg: 40 };
		expect(strokeFields('supports', 'brush', p)).toEqual({
			cursor: 'circle',
			tool: 'brush',
			highlight_deg: 40
		});
		expect(strokeFields('supports', 'fill', p).highlight_deg).toBe(40);
		expect(strokeFields('seam', 'brush', p).highlight_deg).toBeUndefined();
		expect(
			strokeFields('supports', 'brush', { ...p, overhangOnlyDeg: 0 }).highlight_deg
		).toBeUndefined();
		expect(strokeFields('supports', 'brush', { ...p, overhangOnlyDeg: 120 }).highlight_deg).toBe(
			90
		);
	});
});
