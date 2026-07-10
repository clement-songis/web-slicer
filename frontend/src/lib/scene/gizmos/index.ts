// Gizmos de transformation (T052) : composants Threlte + barre d'outils.
export { default as TransformGizmo } from './TransformGizmo.svelte';
export { default as TransformPanel } from './TransformPanel.svelte';
export { default as GizmoToolbar } from './GizmoToolbar.svelte';
export { GIZMO_MODE_LABELS, type GizmoMode } from './types';
export { default as MeasureTool } from './MeasureTool.svelte';
export { default as BrimEarsTool } from './BrimEarsTool.svelte';
export { default as EmbossTool } from './EmbossTool.svelte';
export {
	distance,
	delta,
	angleDeg,
	angleBetweenNormals,
	measurePoints,
	type PointMeasurement
} from './measure';
export { BrimEars, BRIM_EARS_KEYS, type BrimEarPoint, type BrimEarsDocument } from './brim-ears';
export {
	defaultEmbossParams,
	validateEmboss,
	type EmbossParams,
	type EmbossSource
} from './emboss';
export {
	PaintToolbar,
	TrianglePainting,
	trianglesInRadius,
	encodeFacet,
	decodeFacet,
	ENFORCER,
	BLOCKER,
	CHANNELS,
	CHANNEL_ATTR,
	type PaintChannel,
	type PaintDocument,
	type PaintTool
} from './painting/index';
