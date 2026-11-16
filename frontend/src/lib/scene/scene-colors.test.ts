// Palette 3D par thème : la vue suit la préférence (clair / sombre) et reste
// lisible (grille et pièce distinctes du plateau, WCAG 1.4.11 ≥ 3:1 visé).
import { describe, expect, it } from 'vitest';
import { get } from 'svelte/store';
import { resolvedTheme } from '$lib/theme/theme';
import { SCENE_PALETTES, sceneColors } from './scene-colors';
import { contrastRatio as ratio, parseHex, relativeLuminance } from '$lib/ui/color';

const lum = (h: string) => relativeLuminance(parseHex(h)!);
const contrastRatio = (a: string, b: string) => ratio(lum(a), lum(b));

const hex = (n: number) => '#' + n.toString(16).padStart(6, '0');

describe('scene-colors', () => {
	it('suit le thème résolu', () => {
		resolvedTheme.set('light');
		expect(get(sceneColors)).toBe(SCENE_PALETTES.light);
		resolvedTheme.set('dark');
		expect(get(sceneColors)).toBe(SCENE_PALETTES.dark);
	});

	it.each(['light', 'dark'] as const)('mêmes clés dans les deux palettes (%s)', (t) => {
		expect(Object.keys(SCENE_PALETTES[t]).sort()).toEqual(Object.keys(SCENE_PALETTES.dark).sort());
	});

	it('clair : grille principale ≥ 3:1 sur le plateau, pièce distincte du plateau', () => {
		const p = SCENE_PALETTES.light;
		expect(contrastRatio(hex(p.gridMajor), p.bed)).toBeGreaterThanOrEqual(3);
		expect(contrastRatio(p.part, '#f1f5f9')).toBeGreaterThanOrEqual(3);
	});
});
