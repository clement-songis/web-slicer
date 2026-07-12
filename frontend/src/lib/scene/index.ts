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
export { frameBed, fitDistance, viewPose, viewUp, type CameraPose, type NamedView } from './camera';
export { applyPick, isSelected } from './selection';
export { decodeMesh, centerMesh, type SceneMesh, type SceneObject } from './mesh';
export { uploadModel, fetchMesh } from './loaders';
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
export {
	TransformGizmo,
	TransformPanel,
	GizmoToolbar,
	ToolRail,
	PaintToolbar,
	TrianglePainting,
	trianglesInRadius,
	encodeFacet,
	decodeFacet,
	MeasureTool,
	BrimEarsTool,
	EmbossTool,
	measurePoints,
	angleDeg,
	BrimEars,
	defaultEmbossParams,
	validateEmboss,
	type GizmoMode,
	type PaintChannel,
	type PaintDocument,
	type EmbossParams,
	type BrimEarPoint
} from './gizmos';
export { ObjectTree, type SceneNode, type NodeKind } from './objects.svelte';
export { default as ObjectList } from './ObjectList.svelte';
export { PlateSet, DEFAULT_PLATE_TYPE, type Plate, type PlatesDocument } from './plates.svelte';
export { default as PlateBar } from './PlateBar.svelte';
export { default as PlateToolbar } from './PlateToolbar.svelte';
export { default as ContextMenu } from './ContextMenu.svelte';
export { footprint, arrangeItems, applyPlacements } from './arrange';
export {
	MAIN_TOOLBAR,
	ASSEMBLE_TOOLBAR,
	PLATER_MENU,
	OBJECT_CONTEXT_ITEMS,
	SCENE_ADD_ITEMS,
	PLATER_SHORTCUTS,
	OBJECTS_LIST_SHORTCUTS,
	GIZMO_SHORTCUTS,
	type ToolbarButton,
	type ContextMenuItem,
	type ObjectMenuEntry,
	type Shortcut
} from './menus';
export {
	serializeScene,
	parseScene,
	serializeObject,
	deserializeObject,
	SCENE_SCHEMA_VERSION,
	type SceneDocument,
	type SceneObjectDoc,
	type SceneObjectState
} from './document';
export {
	saveScene,
	classifySaveError,
	onSaveShortcut,
	captureThumbnail,
	type SaveOutcome
} from './save';
export { default as SaveControls } from './SaveControls.svelte';
export {
	CutTool,
	RepairTool,
	SimplifyTool,
	BooleanTool,
	LayerHeight,
	AssemblyView,
	signedDistance,
	splitByPlane,
	connectorGrid,
	simplifyGrid,
	triangleCount,
	uniformProfile,
	heightAt,
	setBand,
	smooth,
	serializeLayerProfile,
	deserializeLayerProfile,
	explode,
	type CutPlane,
	type LayerBand,
	type AssemblyPart
} from './tools';
export { meshFromTriangleSoup } from './loaders';
export {
	cube,
	cylinder,
	cone,
	sphere,
	disc,
	torus,
	primitiveMesh,
	type PrimitiveKind
} from './primitives';
