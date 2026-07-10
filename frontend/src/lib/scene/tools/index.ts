// Outils de scène (T055) : coupe, réparation, booléens, simplification.
export { default as CutTool } from './CutTool.svelte';
export { default as RepairTool } from './RepairTool.svelte';
export { default as SimplifyTool } from './SimplifyTool.svelte';
export { default as BooleanTool } from './BooleanTool.svelte';
export { signedDistance, splitByPlane, connectorGrid, type CutPlane } from './cut';
export { simplifyGrid, triangleCount } from './simplify';
export { default as LayerHeight } from './LayerHeight.svelte';
export { default as AssemblyView } from './AssemblyView.svelte';
export {
	uniformProfile,
	heightAt,
	setBand,
	smooth,
	serialize as serializeLayerProfile,
	deserialize as deserializeLayerProfile,
	clampHeight,
	meshMaxZ,
	type LayerBand
} from './layer-height';
export { sceneCenter, explode, type AssemblyPart } from './assembly';
