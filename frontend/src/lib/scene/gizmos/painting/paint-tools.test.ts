import { describe, expect, test } from 'vitest';
import {
	CHANNEL_TOOLS,
	PAINT_PARAM_DEFAULTS,
	clampGapArea,
	clampHeight,
	coerceTool,
	constrainPointer,
	constraintsFor,
	hasHoverPreview,
	mergeRemapMap,
	paintsOnClick,
	remapBaseExtruder,
	remapMessageMap,
	strokeFields,
	wheelParams
} from './paint-tools';

describe('outils des gizmos de peinture (lot 38)', () => {
	test('outils par canal dans l’ordre d’Orca', () => {
		expect(CHANNEL_TOOLS.mmu).toEqual(['brush', 'sphere', 'pointer', 'height', 'fill', 'gap']);
		expect(CHANNEL_TOOLS.supports).toEqual(['brush', 'sphere', 'fill', 'gap']);
		expect(CHANNEL_TOOLS.fuzzy).toEqual(['brush', 'sphere', 'pointer', 'fill']);
		expect(CHANNEL_TOOLS.seam).toContain('brush');
		expect(coerceTool('supports', 'height')).toBe('brush');
		expect(coerceTool('mmu', 'height')).toBe('height');
		expect(paintsOnClick('gap')).toBe(false);
		expect(hasHoverPreview('fill') && hasHoverPreview('pointer')).toBe(true);
		expect(hasHoverPreview('brush')).toBe(false);
	});

	test('messages stroke : correspondance des outils (LOT38 §3)', () => {
		const p = { ...PAINT_PARAM_DEFAULTS, fillAngle: 45 };
		expect(strokeFields('supports', 'fill', p)).toEqual({
			cursor: 'pointer',
			tool: 'smart_fill',
			fill_angle: 45
		});
		expect(strokeFields('mmu', 'fill', p)).toEqual({
			cursor: 'pointer',
			tool: 'bucket_fill',
			fill_angle: 45
		});
		// Pot sans détection d'arêtes : fill_angle = -1.
		expect(strokeFields('mmu', 'fill', { ...p, edgeDetection: false }).fill_angle).toBe(-1);
		expect(strokeFields('mmu', 'pointer', p)).toEqual({ cursor: 'pointer', tool: 'brush' });
		expect(strokeFields('mmu', 'height', { ...p, height: 1.5 })).toEqual({
			cursor: 'height_range',
			tool: 'brush',
			height: 1.5
		});
		expect(strokeFields('seam', 'brush', p)).toEqual({ cursor: 'circle', tool: 'brush' });
		expect(strokeFields('seam', 'brush', p, false).cursor).toBe('sphere');
		expect(strokeFields('fuzzy', 'sphere', p).cursor).toBe('sphere');
	});

	test('bornes : hauteur 0,1–8, comblement 0–5 par pas de 0,2', () => {
		expect(clampHeight(0)).toBe(0.1);
		expect(clampHeight(12)).toBe(8);
		expect(clampGapArea(5.5)).toBe(5);
		expect(clampGapArea(0.31)).toBe(0.4);
		expect(clampGapArea(-1)).toBe(0);
	});

	test('Ctrl + molette : paramètre de l’outil actif', () => {
		const p = PAINT_PARAM_DEFAULTS;
		expect(wheelParams('brush', p, true).radius).toBe(2.2);
		expect(wheelParams('height', p, true).height).toBe(0.4);
		expect(wheelParams('fill', p, false).fillAngle).toBe(25);
		expect(wheelParams('gap', p, true).gapArea).toBe(0.2);
		expect(wheelParams('pointer', p, true)).toEqual(p);
	});

	test('contrainte Verticale / Horizontale (x ou y du premier point figé)', () => {
		expect(constraintsFor('mmu', 'pointer')).toEqual(['vertical', 'horizontal']);
		expect(constraintsFor('seam', 'brush')).toEqual(['vertical']);
		expect(constraintsFor('supports', 'brush')).toEqual([]);
		expect(constraintsFor('mmu', 'fill')).toEqual([]);
		expect(constrainPointer([0.1, 0.2], [0.5, -0.3], 'vertical')).toEqual([0.1, -0.3]);
		expect(constrainPointer([0.1, 0.2], [0.5, -0.3], 'horizontal')).toEqual([0.5, 0.2]);
		expect(constrainPointer([0.1, 0.2], [0.5, -0.3], null)).toEqual([0.5, -0.3]);
	});

	test('remappage : table Orca → map du message, extrudeur de base', () => {
		// Orca m_extruder_remap [2, 0, 2] : 1 → 3, 2 → 1, 3 inchangé.
		expect(remapMessageMap([2, 0, 2])).toEqual([3, 1, 3]);
		expect(remapMessageMap([0, 1, 2])).toBeNull();
		expect(remapBaseExtruder(1, [2, 0, 2])).toBe(3);
		expect(remapBaseExtruder(0, [2, 0, 2])).toBe(3); // hérité → traité comme 1
		expect(remapBaseExtruder(3, [2, 0, 2])).toBeNull();
		expect(remapBaseExtruder(5, [2, 0, 2])).toBeNull();
	});

	test('fusion : map 1-based du retrait d’un slot', () => {
		// 3 filaments, le 2 (slot 1) fondu dans le 1 : 1→1, 2→1, 3→2.
		const merge = (e: number) => (e === 2 ? 1 : e > 2 ? e - 1 : e);
		expect(mergeRemapMap(3, merge)).toEqual([1, 1, 2]);
		expect(mergeRemapMap(2, (e) => e)).toBeNull();
	});
});
