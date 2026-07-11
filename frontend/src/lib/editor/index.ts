// Module éditeur (T087) : orchestrateur pur du workspace (assemblage scène +
// réglages + aperçu dans la page projet). La logique reste ici ; le composant
// ne fait que la disposition (règle CLAUDE.md : aucune logique métier en .svelte).
export {
	canSlice,
	initialWorkspace,
	isSelected,
	pick,
	setGizmoMode,
	setPanel,
	setSelection,
	setSettingsMode,
	togglePanel,
	type EditorPanel,
	type WorkspaceState
} from './workspace';
export {
	applyJobEvent,
	prepareSession,
	resetSession,
	sliceFailed,
	startSlicing,
	type SlicePhase,
	type SliceSession
} from './session';
export { buildWindowGeometry, rangesFromMeta, sliceRequestFor } from './preview';
