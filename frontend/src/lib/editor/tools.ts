// Orchestrateur pur de l'outil actif du rail d'outils (T103) : miroir des gizmos
// OrcaSlicer (`traceability-map.json#gizmos`). Découplé de Svelte → testable sous
// bun. Le rail sélectionne un outil ; les outils de transformation (move/rotate/
// scale) pilotent en plus le mode du `TransformGizmo` dans la scène, les autres
// ouvrent leur panneau dédié (câblés par T104/T105).
import type { GizmoMode } from '../scene/gizmos/types';

/** Identifiant d'outil du rail (parité gizmos OrcaSlicer). */
export type EditorTool =
	| 'move'
	| 'rotate'
	| 'scale'
	| 'flatten'
	| 'cut'
	| 'boolean'
	| 'support-paint'
	| 'seam-paint'
	| 'fuzzy-paint'
	| 'mm-paint'
	| 'emboss'
	| 'svg'
	| 'measure'
	| 'assembly'
	| 'simplify'
	| 'brim-ears';

/** Outils de transformation directe (pilotent le mode du gizmo de scène). */
export const TRANSFORM_TOOLS = ['move', 'rotate', 'scale'] as const;

/** Ordre d'affichage du rail (transformation en tête, puis outils de maillage). */
export const TOOL_ORDER: readonly EditorTool[] = [
	'move',
	'rotate',
	'scale',
	'flatten',
	'cut',
	'boolean',
	'support-paint',
	'seam-paint',
	'fuzzy-paint',
	'mm-paint',
	'emboss',
	'svg',
	'measure',
	'assembly',
	'simplify',
	'brim-ears'
];

/** État de l'outil actif (immuable). */
export interface ToolsState {
	active: EditorTool | null;
}

/** État initial : outil Déplacer (parité, gizmo de translation par défaut). */
export function initialTools(overrides: Partial<ToolsState> = {}): ToolsState {
	return { active: 'move', ...overrides };
}

/** Sélectionne un outil ; re-cliquer sur l'outil actif le désactive (bascule). */
export function setTool(state: ToolsState, tool: EditorTool): ToolsState {
	return { active: state.active === tool ? null : tool };
}

/** L'outil est-il un outil de transformation directe (move/rotate/scale) ? */
export function isTransformTool(tool: EditorTool | null): boolean {
	return tool === 'move' || tool === 'rotate' || tool === 'scale';
}

/** Mode de gizmo correspondant à un outil de transformation (sinon null). */
export function gizmoModeOf(tool: EditorTool | null): GizmoMode | null {
	switch (tool) {
		case 'move':
			return 'translate';
		case 'rotate':
			return 'rotate';
		case 'scale':
			return 'scale';
		default:
			return null;
	}
}
