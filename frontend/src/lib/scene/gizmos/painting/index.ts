// Gizmos de peinture (T056) : supports, couture, fuzzy skin, segmentation MMU.
export { default as PaintToolbar } from './PaintToolbar.svelte';
export {
	TrianglePainting,
	trianglesInRadius,
	CHANNELS,
	CHANNEL_ATTR,
	type PaintChannel,
	type PaintDocument
} from './painting';
export { encodeFacet, decodeFacet, ENFORCER, BLOCKER } from './facets';
export { PAINT_TOOL_LABELS, CHANNEL_LABELS, type PaintTool } from './types';
