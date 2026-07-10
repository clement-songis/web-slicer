// Scène 3D de préparation (T050) : composants Threlte et logique de scène.

export { default as Scene } from './Scene.svelte';
export { default as Viewport } from './Viewport.svelte';
export { default as Bed } from './Bed.svelte';
export { default as ModelObject } from './ModelObject.svelte';
export {
	bedFromValues,
	parseAreaPoints,
	isRectangular,
	gridDivisions,
	type BedShape,
	type Point2
} from './bed';
export { frameBed, fitDistance, type CameraPose } from './camera';
export { applyPick, isSelected } from './selection';
export { decodeMesh, type SceneMesh, type SceneObject } from './mesh';
export {
	previewFormat,
	parseStl,
	parseObj,
	parse3mf,
	loadPreview,
	uploadModel,
	fetchMesh,
	type PreviewFormat
} from './loaders';
export {
	IDENTITY,
	MIN_SCALE,
	clampScale,
	normalizeAngle,
	uniformScale,
	layFlatRotation,
	rotateVectorByEulerDeg,
	quaternionFromUnitVectors,
	eulerDegFromQuaternion,
	type Transform
} from './transform';
export { TransformGizmo, TransformPanel, GizmoToolbar, type GizmoMode } from './gizmos';
