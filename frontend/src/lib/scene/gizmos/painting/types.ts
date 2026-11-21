// Types du gizmo de peinture (T056 ; outils du lot 38 dans paint-tools.ts).
import type { PaintChannel } from './painting';

export { PAINT_TOOL_LABELS, type PaintTool } from './paint-tools';

/** Libellés des canaux : clés i18n anglaises, traduites au rendu ($t). */
export const CHANNEL_LABELS: Record<PaintChannel, string> = {
	supports: 'Supports',
	seam: 'Seam',
	fuzzy: 'Fuzzy skin',
	mmu: 'Multi-material'
};
