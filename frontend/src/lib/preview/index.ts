// Prévisualisation G-code (T068) : décodage des buffers `WSPv`, colorations
// FR-041 (type/vitesse/hauteur/largeur/débit/température/filament) et géométrie
// de lignes prête pour Three.js. Composant `PreviewLines.svelte` pour le rendu.
export { decodePreview, PreviewFormatError, RECORD_BYTES, HEADER_BYTES } from './decode';
export type { PreviewSegments } from './decode';
export {
	SEGMENT_ROLE_COLORS,
	SEGMENT_ROLE_NAMES,
	RANGE_PALETTE,
	DEFAULT_FILAMENT_COLORS,
	NO_DATA_COLOR,
	sampleRange,
	rangeColor
} from './colorations';
export type { Coloration, Rgb, Legend, LegendScale, LegendList, LegendEntry } from './colorations';
export { buildPreviewGeometry, buildLegend, computeFlowRange, flowValue } from './geometry';
export type { PreviewGeometry, GeometryOptions, PreviewRanges, PreviewType } from './geometry';
export { default as PreviewLines } from './PreviewLines.svelte';
