// Types du gizmo de peinture (T056).
import type { PaintChannel } from './painting';

/** Outil de peinture : pinceau (rayon), sphère (volume), remplissage (région). */
export type PaintTool = 'brush' | 'sphere' | 'fill';

export const PAINT_TOOL_LABELS: Record<PaintTool, string> = {
	brush: 'Pinceau',
	sphere: 'Sphère',
	fill: 'Remplissage'
};

export const CHANNEL_LABELS: Record<PaintChannel, string> = {
	supports: 'Supports',
	seam: 'Couture',
	fuzzy: 'Fuzzy skin',
	mmu: 'Multi-matériaux'
};
