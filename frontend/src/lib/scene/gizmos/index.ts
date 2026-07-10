// Gizmos de transformation (T052) : composants Threlte + barre d'outils.
export { default as TransformGizmo } from './TransformGizmo.svelte';
export { default as TransformPanel } from './TransformPanel.svelte';
export { default as GizmoToolbar } from './GizmoToolbar.svelte';
export { GIZMO_MODE_LABELS, type GizmoMode } from './types';
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
