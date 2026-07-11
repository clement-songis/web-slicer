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
export { copyObjects, pastedPosition, PASTE_OFFSET } from './clipboard';
export { resolveShortcut, chordString, isToolAction, type KeyChord } from './shortcuts';
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
export {
	EDITOR_DEFAULT_THEME,
	EDITOR_TABS,
	initialLayout,
	setTab,
	showsPrepare,
	showsPreview,
	type EditorTab,
	type LayoutState
} from './layout';
export {
	emptyActivePresets,
	parseActivePresets,
	primaryFilament,
	serializeActivePresets,
	setFilament,
	setPrinter,
	setProcess,
	type ActivePresets,
	type PresetKind
} from './presets';
export {
	gizmoModeOf,
	initialTools,
	isTransformTool,
	paintChannelOf,
	setTool,
	TOOL_ORDER,
	TRANSFORM_TOOLS,
	type EditorTool,
	type ToolsState
} from './tools';
export {
	findByModel,
	importExt,
	isAccepted,
	isPreviewable,
	markConverted,
	markFailed,
	markUploaded,
	startImport,
	type ImportItem,
	type ImportStatus
} from './imports';
