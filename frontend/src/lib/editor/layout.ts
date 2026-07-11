// Orchestrateur de disposition de l'éditeur (T097) : onglet supérieur actif de
// la fenêtre à la manière d'OrcaSlicer (Préparer / Aperçu / Appareil / Projet).
// État pur, découplé de Svelte (testable sous bun) ; la page projet enveloppe
// cet état dans une rune `$state` et délègue chaque transition ici.
//
// Distinct de `workspace.ts` (sélection, gizmo, réglages) : `layout.ts` ne gère
// que la navigation entre les grandes vues et le thème par défaut de l'éditeur.

/** Onglet supérieur de l'éditeur, miroir des vues OrcaSlicer. */
export type EditorTab = 'prepare' | 'preview' | 'device' | 'project';

/** Ordre d'affichage des onglets (parité barre supérieure OrcaSlicer). */
export const EDITOR_TABS: readonly EditorTab[] = ['prepare', 'preview', 'device', 'project'];

/** Thème par défaut de l'éditeur : sombre, comme OrcaSlicer (les surfaces 3D et
 *  les panneaux d'outils sont pensés pour un fond sombre). */
export const EDITOR_DEFAULT_THEME = 'dark' as const;

/** État de disposition (immuable : chaque transition renvoie une copie). */
export interface LayoutState {
	tab: EditorTab;
}

/** État initial : vue Préparer. */
export function initialLayout(overrides: Partial<LayoutState> = {}): LayoutState {
	return { tab: 'prepare', ...overrides };
}

/** Bascule vers un onglet explicite. */
export function setTab(state: LayoutState, tab: EditorTab): LayoutState {
	return { ...state, tab };
}

/** La vue courante affiche-t-elle la scène de préparation 3D ? */
export function showsPrepare(state: LayoutState): boolean {
	return state.tab === 'prepare';
}

/** La vue courante affiche-t-elle l'aperçu G-code ? */
export function showsPreview(state: LayoutState): boolean {
	return state.tab === 'preview';
}
