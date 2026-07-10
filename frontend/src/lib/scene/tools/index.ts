// Outils de scène (T055) : coupe, réparation, booléens, simplification.
export { default as CutTool } from './CutTool.svelte';
export { default as RepairTool } from './RepairTool.svelte';
export { default as SimplifyTool } from './SimplifyTool.svelte';
export { default as BooleanTool } from './BooleanTool.svelte';
export { signedDistance, splitByPlane, connectorGrid, type CutPlane } from './cut';
export { simplifyGrid, triangleCount } from './simplify';
