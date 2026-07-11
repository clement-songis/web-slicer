// Orchestrateur du workspace éditeur (T087) : état de disposition pur —
// panneau actif (Préparer / Aperçu), mode de gizmo, mode d'affichage des
// réglages et pont de sélection scène↔liste. Découplé de tout composant Svelte
// pour rester testable sous bun ; le composant `projects/[id]/+page.svelte`
// enveloppe cet état dans des runes `$state` et délègue chaque transition ici.
import type { GizmoMode } from '../scene/gizmos/types';
import { applyPick } from '../scene/selection';
import type { DisplayMode } from '../settings/filter';

/** Panneau actif de l'éditeur : préparation de scène ou aperçu G-code. */
export type EditorPanel = 'prepare' | 'preview';

/** État de disposition du workspace (immuable : chaque transition en renvoie une copie). */
export interface WorkspaceState {
	panel: EditorPanel;
	gizmoMode: GizmoMode;
	settingsMode: DisplayMode;
	/** Objets sélectionnés — source unique partagée par la scène 3D et la liste. */
	selection: Set<string>;
}

/** État initial (préparation, gizmo déplacer, mode simple, rien de sélectionné). */
export function initialWorkspace(overrides: Partial<WorkspaceState> = {}): WorkspaceState {
	return {
		panel: 'prepare',
		gizmoMode: 'translate',
		settingsMode: 'simple',
		selection: new Set<string>(),
		...overrides
	};
}

/** Bascule vers un panneau explicite. */
export function setPanel(state: WorkspaceState, panel: EditorPanel): WorkspaceState {
	return { ...state, panel };
}

/** Alterne Préparer ↔ Aperçu. */
export function togglePanel(state: WorkspaceState): WorkspaceState {
	return { ...state, panel: state.panel === 'prepare' ? 'preview' : 'prepare' };
}

/** Change le mode du gizmo de transformation. */
export function setGizmoMode(state: WorkspaceState, gizmoMode: GizmoMode): WorkspaceState {
	return { ...state, gizmoMode };
}

/** Change le mode d'affichage des réglages (simple / advanced / expert). */
export function setSettingsMode(state: WorkspaceState, settingsMode: DisplayMode): WorkspaceState {
	return { ...state, settingsMode };
}

/**
 * Applique un clic de sélection (raycast 3D ou clic dans la liste). La scène et
 * la liste passent toutes deux par ici : l'ensemble sélectionné reste cohérent
 * entre les deux vues. Réutilise la logique pure de `scene/selection`.
 */
export function pick(
	state: WorkspaceState,
	hitId: string | null,
	additive: boolean
): WorkspaceState {
	return { ...state, selection: applyPick(state.selection, hitId, additive) };
}

/** Remplace l'ensemble sélectionné (ex. synchronisation depuis la scène liée). */
export function setSelection(
	state: WorkspaceState,
	selection: ReadonlySet<string>
): WorkspaceState {
	return { ...state, selection: new Set(selection) };
}

/** L'objet est-il sélectionné ? */
export function isSelected(state: WorkspaceState, id: string): boolean {
	return state.selection.has(id);
}

/** Déclencheur de tranchage : au moins un objet posé sur la scène. */
export function canSlice(objectCount: number): boolean {
	return objectCount > 0;
}
