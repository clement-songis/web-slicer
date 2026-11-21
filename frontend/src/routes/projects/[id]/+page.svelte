<script lang="ts">
	import { onMount, tick, untrack } from 'svelte';
	// Perf : la scène 3D (three/threlte) et la vue d'aperçu sont chargées à la
	// demande, hors du chunk initial de la page (≈1 Mo). La scène démarre son
	// téléchargement dès l'évaluation de la page ; l'aperçu, à l'ouverture.
	const sceneModule = import('$lib/scene/Scene.svelte');
	let previewModule: Promise<typeof import('$lib/preview/PreviewScene.svelte')> | null = null;
	const loadPreviewScene = () => (previewModule ??= import('$lib/preview/PreviewScene.svelte'));
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';
	import { resolve } from '$app/paths';
	import { goto } from '$app/navigation';
	import MenuBar from '$lib/menus/MenuBar.svelte';
	import ShortcutsDialog from '$lib/menus/ShortcutsDialog.svelte';
	import TipOfTheDayDialog from '$lib/menus/TipOfTheDayDialog.svelte';
	import PreferencesDialog from '$lib/settings/PreferencesDialog.svelte';
	import {
		DEFAULT_PREFERENCES,
		appPreferences,
		currentPreferences,
		loadPreferences,
		savePreferences,
		type AppPreferences,
		type ArrangePreferences
	} from '$lib/settings/preferences';
	// Phase 22 (audit Orca §10) : modèles pratiques, unités, À propos, purge auto.
	import AboutDialog from '$lib/menus/AboutDialog.svelte';
	import UnitConversionDialog from '$lib/editor/UnitConversionDialog.svelte';
	import HandySuggestionDialog from '$lib/editor/HandySuggestionDialog.svelte';
	import {
		addHandyModel,
		handyKeyOf,
		listHandyModels,
		suggestionApplies,
		withHandyMenu,
		type HandyModelAdded,
		type HandyModelInfo,
		type HandyModelSuggestion
	} from '$lib/editor/handy-models';
	import {
		convertScale,
		detectUnits,
		meshVolume,
		scaledVolume,
		unitActionOf,
		withUnitItems,
		type UnitSource
	} from '$lib/editor/units';
	import { autoFlushResult, changedSlots, type FlushSnapshot } from '$lib/editor/auto-flush';
	import { detachPreset } from '$lib/api/presets';
	// Lot 21 : brouillon au tranchage, peinture conservée, températures, buses.
	import { buildSliceDraft } from '$lib/editor/slice-draft';
	import { preferredFilaments } from '$lib/printers/wizard-filaments';
	import {
		applyBatchEvent,
		batchFinished,
		batchLabel,
		makeBatch,
		resultFor,
		type PlateResults,
		type SliceBatch
	} from '$lib/editor/plate-results';
	import { checkFilamentCompatibility, compatibilityText } from '$lib/editor/filament-compat';
	import { paintedChannels, remapPaint } from '$lib/editor/keep-paint';
	import {
		effectiveMapMode,
		extruderCount,
		filamentMapChoice,
		isBambuMachine,
		listedFilaments,
		readFilamentMap,
		shouldPromptFilamentMap,
		type FilamentMapMode
	} from '$lib/editor/filament-map';
	import FilamentMapDialog from '$lib/editor/FilamentMapDialog.svelte';
	import { buildZip } from '$lib/export/zip';
	import { modelsFromZip, presetsFromFiles } from '$lib/editor/menu-files';
	import { tabTogglesView } from '$lib/editor/tab-focus';
	import { isDefaultProjectName, projectNameFromFile } from '$lib/editor/project-name';
	import GcodeWindow from '$lib/preview/GcodeWindow.svelte';
	import { nextNotice, type Notice } from '$lib/editor/notice';
	import ToolPositionCard from '$lib/preview/ToolPositionCard.svelte';
	import PreviewPlates from '$lib/preview/PreviewPlates.svelte';
	import { toolPositionAt, type ToolPosition } from '$lib/preview/tool-position';
	import { parseGcodeFooterStats } from '$lib/preview/footer-stats';
	import { platesTotals } from '$lib/preview/plates-summary';
	import { jumpToLayer } from '$lib/preview/sliders';
	import { toolpathsToObj } from '$lib/preview/toolpaths-obj';
	import { supportNormalZ } from '$lib/scene/overhang';
	import {
		isByObject,
		mergeInRank,
		orderFromPlacements,
		permuteInSlots,
		sequentialArrangeFields
	} from '$lib/scene/print-order';
	import ValidationPanel from '$lib/editor/ValidationPanel.svelte';
	import {
		buildNotices,
		conversionCause,
		issuesByObject,
		loadOrcaCatalog,
		optionPage,
		plateBlocked,
		validateProject,
		supportsInlineScene,
		type OrcaCatalog,
		type ValidateResponse,
		type ValidationNotice
	} from '$lib/editor/validation';
	import { safeFileName } from '$lib/export/stl';
	import { getJob, listJobs } from '$lib/api/jobs';
	import { jobAsEvent, planResume, resumedProgress } from '$lib/editor/resume-job';
	import { connection } from '$lib/api/connectivity';
	import type { JobResponse } from '$lib/api/types';
	import type { Projection } from '$lib/scene/camera';
	import { cameraSpaceNudge, clampZoom } from '$lib/scene/camera';
	// Phase 34 : Page préc./suiv. (rotation ±45°), « + »/« − » (instances, ou
	// changement de couleur en Aperçu).
	import { rotationFromAction } from '$lib/editor/shortcuts';
	import { rotateSelectionZ } from '$lib/scene/rotate-selection';
	import {
		codeIndexAtZ,
		editColorChangeAt,
		pendingTicks,
		withAuthoredTypes
	} from '$lib/preview/color-changes';
	// Phase 37 : menu du curseur de couches (IMSlider), tableau des paramètres,
	// fusion de filaments, navigateur 3D, débit de buse, coupe, erreurs de maillage.
	import {
		applyLayerCode,
		codeAt,
		layerCodeMenu,
		remapCodesAfterFilamentRemoval,
		type LayerCodeMenuItem
	} from '$lib/preview/layer-code-menu';
	import ObjectTable from '$lib/scene/ObjectTable.svelte';
	import type { TableObjectSource, TableRow } from '$lib/scene/object-table';
	import ViewCube from '$lib/scene/ViewCube.svelte';
	import {
		cutLinkedIds,
		invalidatedIds,
		parseCutId,
		syncAfterCut,
		uniformCutScale
	} from '$lib/scene/cut-info';
	import { meshWarnings as toMeshWarnings, type MeshWarning } from '$lib/scene/mesh-errors';
	import { invalidateCut, projectMeshErrors } from '$lib/api/scene';
	import {
		NOZZLE_FLOW_LABELS,
		nozzleFlowChoices,
		setNozzleFlow,
		type NozzleFlow
	} from '$lib/settings/nozzle-flow';
	import DevicePanel from '$lib/printers/DevicePanel.svelte';
	import { MAIN_MENUS } from '$lib/menus/menus';
	import { draftStore, type DraftRecord } from '$lib/stores/draft';
	import {
		ObjectList,
		PlateBar,
		PlateToolbar,
		ContextMenu,
		OBJECT_CONTEXT_ITEMS,
		SCENE_ADD_ITEMS,
		SCENE_CONTEXT_ITEMS,
		PLATE_CONTEXT_ITEMS,
		FILAMENT_CONTEXT_ITEMS,
		PART_CONTEXT_ITEMS,
		type PrimitiveKind,
		arrangeItems,
		applyPlacements,
		objectFootprint,
		SaveControls,
		ToolRail,
		TransformPanel,
		CutTool,
		BooleanTool,
		SimplifyTool,
		AssemblyView,
		MeasureTool,
		EmbossTool,
		BrimEarsTool,
		LayerHeight,
		uniformProfile,
		serializeLayerProfile,
		deserializeLayerProfile,
		type LayerBand,
		PaintToolbar,
		TrianglePainting,
		trianglesInRadius,
		measurePoints,
		type MeshPick,
		type PaintChannel,
		ObjectTree,
		PlateSet,
		bedFromValues,
		parseAreaPoints,
		objectsOnPlate,
		viewportObjects,
		canTransformObject,
		movableObjectsOnPlate,
		serializeScene,
		parseScene,
		deserializeObject,
		saveScene,
		centerMesh,
		positionForImport,
		uploadModel,
		import3mfObjects,
		fetchMesh,
		sceneMeshToStlFile,
		BrimEars,
		explode,
		type SaveOutcome,
		type SceneObject,
		type SceneObjectState,
		type SceneObjectDoc,
		type SceneMesh,
		type Transform,
		type NamedView,
		boundsOf
	} from '$lib/scene';
	import {
		dropPlateIndex,
		plateOffsets,
		showDropHint,
		type Footprint,
		type PlateView
	} from '$lib/scene/plate-layout';
	import { planSceneRestore } from '$lib/scene/document';
	import {
		VOLUME_ACTIONS,
		defaultVolume,
		volumeTypeName,
		volumeTransform,
		applyVolumeTransform,
		changeVolumeType,
		parseCutInfo,
		partTypeChoices,
		type SceneVolume,
		type VolumeType
	} from '$lib/scene/volumes';
	import ObjectSettingsPanel from '$lib/scene/ObjectSettingsPanel.svelte';
	import LayerRangesPanel from '$lib/scene/LayerRangesPanel.svelte';
	import { addRootRange, type LayerRange as HeightRange } from '$lib/scene/layer-ranges';
	import {
		alignFaces,
		alignPoints,
		worldFaceCenter,
		worldTriangleNormal
	} from '$lib/scene/tools/assembly-align';
	import { restingZ } from '$lib/scene/mesh';
	import { fitScaleFactor, mirrorScale, scaleBy, worldSize } from '$lib/scene/transform-ops';
	import { sectionNormal, sectionPlane, stepSectionRatio } from '$lib/scene/screen-select';
	import type { PaintDocument } from '$lib/scene/gizmos/painting/painting';
	import type { BrimEarsDocument } from '$lib/scene/gizmos/brim-ears';
	import { PresetSettingsDialog } from '$lib/settings';
	import PlateSettingsDialog from '$lib/settings/PlateSettingsDialog.svelte';
	import {
		changedPresetValues,
		clonePresetValues,
		hasPresetChanges,
		presetValuesForSave,
		withOrcaDefaults
	} from '$lib/settings/editing';
	import { swatchStyle } from '$lib/ui';
	import SendToPrinter from '$lib/printers/SendToPrinter.svelte';
	import { SceneHistory, takeSnapshot, type SceneSnapshot } from '$lib/scene/scene-history';
	import type { SceneNode } from '$lib/scene/objects.svelte';
	import CalibrationDialog from '$lib/calibration/CalibrationDialog.svelte';
	import {
		embossEditLabel,
		embossPartOffset,
		paramsFromEmboss,
		type EmbossParams
	} from '$lib/scene/gizmos/emboss';
	import EngineCalibrationDialog from '$lib/calibration/EngineCalibrationDialog.svelte';
	import FlowRateDialog from '$lib/calibration/FlowRateDialog.svelte';
	import PresetBundleDialog, { type BundlePresetRef } from '$lib/presets/PresetBundleDialog.svelte';
	import type { FlowPattern } from '$lib/calibration/flowrate';
	import { type EngineCalibKind, type RangeParams } from '$lib/calibration/engine-calib';
	import {
		type CalibrationKind,
		type PressureAdvanceParams,
		type TemperatureParams
	} from '$lib/calibration/calibration';
	import { createLatestLoader } from '$lib/settings/latest-load';
	import { PresetSelect } from '$lib/presets';
	import OwnedPrinterPicker from '$lib/printers/OwnedPrinterPicker.svelte';
	import PrinterCatalog from '$lib/printers/PrinterCatalog.svelte';
	import {
		LazyPreviewLoader,
		buildPreviewStats,
		makeLayerRange,
		visibleRange,
		makeMoveCursor,
		moveCursorBy,
		moveCursorToStart,
		moveCursorToEnd,
		loadFilamentColors,
		moveThumb,
		setActiveThumb,
		toggleOneLayerMode,
		resolvePreviewKey,
		defaultFilamentColor,
		parseOrcaColor,
		rgbToHex,
		FAST_STEP,
		type LayerRange,
		type Coloration,
		type Rgb,
		type GcodeStats
	} from '$lib/preview';
	import type { PreviewGeometry } from '$lib/preview/geometry';
	import {
		buildLegend,
		buildOverlay,
		computeFlowRange,
		PREVIEW_OPTIONS,
		type PreviewOption
	} from '$lib/preview/geometry';
	import { parsePreviewTrack, trackMatches, type PreviewTrack } from '$lib/preview/track';
	import {
		applyEngine,
		engineLayersFor,
		EngineMoveCache,
		withEngine,
		type EngineMeta,
		type EngineMoves
	} from '$lib/preview/engine-track';
	import {
		estimatedTimeAtTool,
		layerEndTime,
		layerTicks,
		statsFromProcessed,
		withColorPrint,
		withEngineSeams,
		type EngineSeam,
		type EstimatedTime,
		type PlatesStatsResponse
	} from '$lib/preview/engine-data';
	import EngineSummary from '$lib/preview/EngineSummary.svelte';
	import { formatDuration } from '$lib/preview/stats';
	import {
		effectiveRenderMode,
		detectSoftwareRenderer,
		VOLUME_SEGMENT_LIMIT,
		type RenderMode
	} from '$lib/preview/extrusions';
	import PreviewLegend from '$lib/preview/PreviewLegend.svelte';
	import {
		sliceProject,
		listProjectModels,
		createProject,
		duplicateProject,
		renameProject,
		importProject,
		listProjects
	} from '$lib/api/projects';
	import {
		isNameConflict,
		nameSuggestion,
		nameTakenMessage,
		uniqueProjectName
	} from '$lib/library/names';
	import {
		arrangeScene,
		getArrangeOptions,
		booleanModels,
		cutModel,
		orientModel,
		repairModel,
		simplifyModel,
		splitModel,
		smoothModel,
		embossText,
		embossSvg,
		createPrimitive,
		layerProfileModel,
		listEmbossFonts,
		type MeshOpResponse
	} from '$lib/api/scene';
	import { meshOpErrorMessage, placeSourcePart, worldMatrix } from '$lib/scene/tools/mesh-ops';
	import { EnginePaintClient, paintWsUrl } from '$lib/scene/gizmos/painting/engine-paint';
	import { BLOCKER, ENFORCER, remapChannelStates } from '$lib/scene/gizmos/painting/facets';
	import {
		PAINT_PARAM_DEFAULTS,
		coerceTool,
		hasHoverPreview,
		paintsOnClick,
		remapBaseExtruder,
		remapMessageMap,
		strokeFields,
		wheelParams,
		type HoverPreview,
		type PaintAssist,
		type PaintTool,
		type PointerConstraint,
		type StrokeToolFields
	} from '$lib/scene/gizmos/painting/paint-tools';
	import { effectiveRealistic } from '$lib/scene/realistic';
	import { overlappingCount } from '$lib/scene/footprint-overlap';
	import {
		PAINT_BLOCKER_COLOUR,
		PAINT_ENFORCER_COLOUR,
		paintOverlayGeometry,
		type PaintOverlay
	} from '$lib/scene/gizmos/painting/paint-overlay';
	import { StrokeRecorder } from '$lib/scene/gizmos/painting/stroke';
	import { CutSession } from '$lib/scene/tools/cut-session.svelte';
	import { cutPartLabel } from '$lib/scene/tools/cut-plan';
	import { nearestFreeCell } from '$lib/scene/arrange';
	import { effectiveOptions, optionsBody } from '$lib/scene/arrange-options';
	import { placedTransform, plateMoves, unarrangedIds } from '$lib/scene/arrange-gui';
	import ArrangeOptionsPanel from '$lib/scene/ArrangeOptionsPanel.svelte';
	import ProjectMetadataPanel from '$lib/scene/ProjectMetadataPanel.svelte';
	import OpenProjectDialog from '$lib/projects/OpenProjectDialog.svelte';
	import ImportReportDialog from '$lib/projects/ImportReportDialog.svelte';
	import {
		behaviourFromChoice,
		decideLoadMode,
		isProjectArchive,
		stashImportReport,
		takeImportReport
	} from '$lib/projects/open-3mf';
	import type {
		ArrangeOptionsInfo,
		ArrangeResponse,
		ImportLoadMode,
		ImportReport
	} from '$lib/api/types';
	import { measureGuides } from '$lib/scene/guides';
	import { capturePlatePng, putPlateThumbnail } from '$lib/scene/plate-thumbnail';
	import type { BooleanOp } from '$lib/scene/tools/BooleanTool.svelte';
	import {
		instancesOfModel,
		repairSummary,
		repairTarget,
		replaceMesh,
		simplifyRequest,
		simplifySummary
	} from '$lib/scene/repair';
	import type { SimplifyToolParam } from '$lib/scene/tools/SimplifyTool.svelte';
	import {
		fetchPreviewLayers,
		getPreviewMeta,
		gcodeEngineSource,
		openedEngineSource,
		processLocalGcode,
		getPlatesStats,
		localGcodeLayers,
		localGcodeMeta,
		type EngineSource
	} from '$lib/api/preview';
	import {
		listPresets,
		createPreset,
		updatePreset,
		getPreset,
		getResolvedPreset,
		importPreset,
		deletePreset,
		blockingProjects
	} from '$lib/api/presets';
	import { listPrinters, getPrinterCatalog, createPrinter, uploadSliced } from '$lib/api/printers';
	import { friendlySliceError } from '$lib/editor/session';
	import ComparePresetsDialog from '$lib/settings/ComparePresetsDialog.svelte';
	import UndoHistory from '$lib/scene/UndoHistory.svelte';
	import CreateFilamentDialog from '$lib/presets/CreateFilamentDialog.svelte';
	import PaHistoryDialog from '$lib/calibration/PaHistoryDialog.svelte';
	import CalibrationResultCard from '$lib/calibration/CalibrationResultCard.svelte';
	import { calibrationReading, type CalibrationReading } from '$lib/calibration/calibration-result';
	import type { PaMethod } from '$lib/calibration/pa-history';
	import {
		createServerCalibration,
		engineRequest,
		flowRequest,
		pressureAdvanceRequest,
		temperatureRequest
	} from '$lib/calibration/server';
	import UnsavedPresetDialog from '$lib/settings/UnsavedPresetDialog.svelte';
	import { diffPresets, transferChanges, type SettingDiff } from '$lib/settings/preset-tools';
	import {
		clampTowerPosition,
		effectiveBrim,
		estimateWipeTowerSize,
		showWipeTower
	} from '$lib/scene/wipe-tower';
	import FlushVolumesDialog from '$lib/editor/FlushVolumesDialog.svelte';
	import ObjColorDialog from '$lib/editor/ObjColorDialog.svelte';
	import StepMeshDialog from '$lib/editor/StepMeshDialog.svelte';
	import {
		isStep,
		loadStepPrefs,
		remeshStep,
		sameStepOptions,
		STEP_DEFAULTS
	} from '$lib/editor/step-mesh';
	import AuxiliariesPanel from '$lib/editor/AuxiliariesPanel.svelte';
	import { objColors } from '$lib/api/auxiliaries';
	import {
		isMtl,
		mapExtruder,
		pickMtl,
		remapMmu,
		type ObjColorPlan,
		type ObjColorsResponse
	} from '$lib/editor/obj-colors';
	import { computeFlushVolumes, flushVolumesAvailable } from '$lib/api/flush';
	import {
		afterSlotAdded,
		nextFilamentColour,
		afterSlotRemoved,
		compactProjectConfig,
		effectiveColours,
		parseProjectConfig,
		sameProjectConfig,
		setSlotColour,
		setTowerPosition,
		towerPosition,
		DEFAULT_FLUSH_MULTIPLIER,
		type ProjectFilamentConfig
	} from '$lib/editor/multi-filament';
	import { dialogFocus } from '$lib/ui/dialog-focus';
	import { subscribeEvents, type EventSubscription } from '$lib/queue/events';
	import type {
		PreviewMeta,
		PresetSummary,
		ServerEvent,
		PrinterResponse,
		PrinterCatalogVendor,
		PrinterCatalogModel
	} from '$lib/api/types';
	import { ApiError } from '$lib/api/client';
	import { locale, t } from '$lib/i18n';
	import {
		initialWorkspace,
		pick,
		setSelection,
		copyObjects,
		pastedPosition,
		initialTools,
		setTool,
		gizmoModeOf,
		prepareSession,
		startSlicing,
		sliceFailed,
		applyJobEvent,
		buildWindowGeometry,
		rangesFromMeta,
		sliceRequestFor,
		startImport,
		markUploaded,
		markConverted,
		markFailed,
		importLabel,
		isPending,
		isAccepted,
		isTransformTool,
		paintChannelOf,
		resolveShortcut,
		resolveNudge,
		extruderFromAction,
		isToolAction,
		type EditorTool,
		initialLayout,
		setTab,
		emptyActivePresets,
		parseActivePresets,
		serializeActivePresets,
		setFilament,
		type EditorTab,
		type ImportItem
	} from '$lib/editor';
	import type { DisplayMode } from '$lib/settings/filter';
	import {
		addFilamentSlot,
		importedFilamentSlots,
		canRemoveFilamentSlot,
		clampExtruder,
		clampFilamentSlot,
		filamentAt,
		filamentSlotCount,
		remapExtruderAfterMerge,
		remapExtruderAfterRemoval,
		removeFilamentSlot,
		slotAfterRemoval,
		SUPPORT_FILAMENT_KEYS,
		supportFilamentAfterRemoval
	} from '$lib/editor/presets';
	import Dialog from '$lib/ui/Dialog.svelte';
	import { uniqueCopyName } from '$lib/editor/presets';
	import type { PageData } from './$types';
	import IconPencil from '~icons/lucide/pencil';
	import IconPlus from '~icons/lucide/plus';
	import IconX from '~icons/lucide/x';
	import IconPlateSettings from '~icons/lucide/settings-2';
	import IconFileUp from '~icons/lucide/file-up';

	let { data }: { data: PageData } = $props();

	// Brouillon local plus récent que la version serveur → proposer la
	// restauration après une fermeture accidentelle (l'éditeur le consommera).
	let pendingDraft = $state<DraftRecord | null>(null);

	// État de disposition supérieur (onglets Préparer/Aperçu/Appareil/Projet).
	let layout = $state(initialLayout());
	// État de sélection partagé scène↔liste (orchestrateur pur `lib/editor`).
	let ws = $state(initialWorkspace());
	// Outil actif du rail (T103) ; les outils de transformation pilotent le gizmo.
	let tools = $state(initialTools());
	const gizmoMode = $derived(gizmoModeOf(tools.active));
	// Peinture au triangle (T105) : canal piloté par l'outil de peinture actif ;
	// outil (pinceau/sphère/remplissage), rayon et état peint réglés par la barre.
	const paintChannel = $derived(paintChannelOf(tools.active));
	let paintTool = $state<PaintTool>('brush');
	let paintRadius = $state(2);
	// Lot 38 — paramètres des autres outils (hauteur de plage, comblement, pot
	// MMU sans arêtes) et contrainte « Verticale / Horizontale » de la souris.
	let paintHeight = $state(PAINT_PARAM_DEFAULTS.height);
	let paintGapArea = $state(PAINT_PARAM_DEFAULTS.gapArea);
	let paintEdgeDetection = $state(PAINT_PARAM_DEFAULTS.edgeDetection);
	let paintConstraint = $state<PointerConstraint>(null);
	let paintBusy = $state(false);
	// Plan de coupe des gizmos de peinture (Alt+molette, ratio 0..1, 0 = inactif).
	let sectionRatio = $state(0);
	// Normale du plan (direction de vue au premier cran, conservée ensuite).
	let sectionNormalDir = $state<[number, number, number] | null>(null);
	// Direction de vue courante remontée par la scène (non réactive).
	let cameraForward: [number, number, number] = [0, 0, -1];
	let paintState = $state(1);
	// Document de peinture par objet (indexé par id) : créé paresseusement pour
	// l'objet sélectionné, sérialisable au format 3MF Orca via `serialize`.
	let paintings = $state<Record<string, TrianglePainting>>({});
	let brimEarsByObject = $state<Record<string, BrimEars>>({});
	// Plages de hauteur à réglages propres (Orca `layer_config_ranges`), par objet.
	let layerRangesByObject = $state<Record<string, HeightRange[]>>({});
	// Maillages recentrés des volumes `mesh` (« Split to parts »), par modèle.
	let volumeMeshes = $state<Record<string, SceneMesh>>({});
	// eslint-disable-next-line svelte/prefer-svelte-reactivity
	const volumeMeshRequests = new Set<string>();
	let measuredPoints = $state<[number, number, number][]>([]);
	// Repères 3D des outils (phase 26) : plan de l'outil Coupe, points de Mesure.
	// Phase 28 : état de l'outil Coupe complet (plan incliné, connecteurs, queue
	// d'aronde) partagé entre le panneau, la vue 3D et la requête serveur.
	const cutSession = new CutSession();
	$effect(() => {
		const o = tools.active === 'cut' ? activeObject : null;
		cutSession.bind(
			o
				? {
						id: o.id,
						positions: o.mesh.positions,
						indices: o.mesh.indices,
						xf: {
							position: [...(o.position ?? [0, 0, 0])] as [number, number, number],
							rotation: [...(o.rotation ?? [0, 0, 0])] as [number, number, number],
							scale: [...(o.scale ?? [1, 1, 1])] as [number, number, number]
						}
					}
				: null
		);
	});
	// Pose des connecteurs : partie supérieure masquée (face de coupe visible, comme Orca).
	const cutClip = $derived.by(() => {
		if (tools.active !== 'cut' || !cutSession.editingConnectors || cutSession.mode !== 'planar')
			return null;
		const g = cutSession.guide();
		if (!g) return null;
		const n = g.normal;
		return { normal: n, offset: n[0] * g.center[0] + n[1] * g.center[1] + n[2] * g.center[2] };
	});
	const sceneGuides = $derived.by(() => {
		if (tools.active === 'cut' && cutSession.target) {
			const plane = cutSession.guide();
			return plane ? { plane } : null;
		}
		if (tools.active === 'measure' && measuredPoints.length > 0)
			return measureGuides(measuredPoints);
		return null;
	});
	const measurement = $derived(
		measuredPoints.length === 2 ? measurePoints(measuredPoints[0], measuredPoints[1]) : null
	);
	// Vue d'assemblage : déplacement purement visuel, jamais persisté.
	let assemblyExploded = $state(false);
	let assemblyFactor = $state(0.5);
	let assemblyMode = $state<'explode' | 'face' | 'point'>('explode');
	let assemblyPick = $state<{
		objectId: string;
		point: [number, number, number];
		normal: [number, number, number] | null;
	} | null>(null);
	// Volumes ajoutés par objet (pièce, négatif, modificateur, bloqueur/forceur).
	let volumesByObject = $state<Record<string, SceneVolume[]>>({});
	// Panneau des réglages par objet / par volume (« Ajouter des réglages »).
	let objectSettingsAdding = $state(false);
	let selectedVolumeId = $state<string | null>(null);

	// Barre d'outils de plateau (T106) : édition de hauteur de couche variable et
	// profil par objet. La vue d'assemblage réutilise l'outil « assembly » du rail.
	let showLayerHeight = $state(false);
	let layerProfiles = $state<Record<string, LayerBand[]>>({});

	// Presse-papier de scène (menu Édition, T108) : objets copiés/coupés.
	let clipboard = $state<ObjectSnapshot[]>([]);

	// Vue caméra + affichage (menu Vue, T109).
	let cameraView = $state<NamedView>('default');
	let cameraZoom = $state(1);
	// Cadrage sur la sélection (outils de peinture, bouton « Cadrer l'objet ») ;
	// toute vue nommée du menu Vue revient au cadrage du plateau.
	let cameraFocus = $state<{ min: [number, number, number]; max: [number, number, number] } | null>(
		null
	);
	let printDialogRequest = $state(0);
	let showGrid = $state(true);

	// Dialogue des raccourcis clavier (menu Aide, T111).
	let showShortcuts = $state(false);
	let showTipOfTheDay = $state(false);
	let showAbout = $state(false);

	// Onglet Projet (T117) : nom éditable (renommage via l'API projets).
	let projectName = $state('');
	let savedName = $state('');
	/** `fromFile` : nommage automatique d'un projet « Sans titre » (Orca) — le
	 *  message le dit, au lieu d'un « Projet renommé » que l'utilisateur n'a pas demandé. */
	async function renameCurrentProject(fromFile?: string) {
		const name = projectName.trim();
		if (!name || name === savedName) return;
		try {
			await renameProject(data.project.id, name);
			savedName = name;
			saveMessage = fromFile ? `Projet nommé « ${name} » d'après ${fromFile}.` : 'Projet renommé.';
		} catch (e) {
			if (isNameConflict(e) && fromFile) {
				// Phase 32 : nom de fichier déjà pris par un autre projet → « cube20 (2) ».
				try {
					// Lot 32 : nom libre proposé par le serveur ; sinon calculé ici.
					const free =
						nameSuggestion(e) ??
						uniqueProjectName(name, [
							...(await listProjects()).filter((p) => p.id !== data.project.id).map((p) => p.name),
							name
						]);
					await renameProject(data.project.id, free);
					projectName = savedName = free;
					saveMessage = `Projet nommé « ${free} » d'après ${fromFile}.`;
					return;
				} catch {
					// repli : message ci-dessous
				}
			}
			saveMessage = isNameConflict(e)
				? nameTakenMessage(name, nameSuggestion(e))
				: e instanceof ApiError
					? e.message
					: 'renommage impossible';
		}
	}

	// Menu contextuel objet (T112) : cible + position au curseur.
	let contextMenu = $state<{
		open: boolean;
		x: number;
		y: number;
		targetId: string | null;
		/** Cible = pièce d'un objet multi-pièces (menu `create_part_menu`). */
		part?: boolean;
	}>({
		open: false,
		x: 0,
		y: 0,
		targetId: null
	});
	// Menu d'ajout (T113) : Load + primitives, au curseur ou sous le bouton `add`.
	// Menu contextuel d'un onglet de plateau (PlateBar).
	let plateMenu = $state<{ open: boolean; x: number; y: number; plateId: string | null }>({
		open: false,
		x: 0,
		y: 0,
		plateId: null
	});
	// `scene` : ouvert par clic droit sur le plateau vide (ajoute « Show Labels »).
	let addMenu = $state<{ open: boolean; x: number; y: number; scene?: boolean }>({
		open: false,
		x: 0,
		y: 0
	});
	let filamentMenu = $state<{ open: boolean; x: number; y: number; slot: number }>({
		open: false,
		x: 0,
		y: 0,
		slot: 0
	});
	// Phase 37 — menu du curseur de couches (IMSlider) : couche visée au clic.
	let layerMenu = $state<{ open: boolean; x: number; y: number; layer: number }>({
		open: false,
		x: 0,
		y: 0,
		layer: 0
	});
	// Fenêtre « G-code personnalisé » (IMSlider::render_input_custom_gcode).
	let customGcodeEdit = $state<{ layer: number; z: number; text: string; edit: boolean } | null>(
		null
	);
	// Tableau des paramètres d'objets (« Edit in Parameter Table »).
	let objectTableOpen = $state(false);
	let objectTableFocus = $state<string | null>(null);
	// Erreurs de maillage par objet (GET …/mesh-errors, après enregistrement).
	let meshWarningsByObject = $state<Record<string, MeshWarning>>({});
	// Direction de vue (réactive) pour le navigateur 3D.
	let navForward = $state<[number, number, number]>([0, 1, -0.6]);
	// Phase 38 — navigateur 3D de l'Aperçu : vue choisie au cube, direction caméra.
	let previewNamedView = $state<NamedView | null>(null);
	let previewViewKey = $state(0);
	let previewForward = $state<[number, number, number]>([0.6, 0.8, -0.7]);

	// Modèle de scène (mutations en place → proxysées par `$state`, réactives).
	let tree = $state(new ObjectTree());
	let plates = $state(new PlateSet());
	// Maillages rendus dans la scène 3D : peuplés à l'ouverture depuis les modèles
	// du projet (T092) puis par les imports (T089).
	let sceneObjects = $state<SceneObject[]>([]);
	const activePlate = $derived(plates.activeId ? (plates.get(plates.activeId) ?? null) : null);
	const activeSceneObjects = $derived(objectsOnPlate(sceneObjects, activePlate));
	// Badges de la liste d'objets : réglages surchargés (objet + pièces), plages
	// de hauteur et pièces/modificateurs de chaque objet.
	const objectOverrideCounts = $derived.by(() => {
		const out: Record<string, { settings: number; ranges: number; parts: number }> = {};
		for (const o of sceneObjects) {
			const volumes = volumesByObject[o.id] ?? [];
			const ranges = layerRangesByObject[o.id] ?? [];
			const own = Object.keys(tree.get(o.id)?.settings ?? {}).length;
			const inParts = volumes.reduce((n, v) => n + Object.keys(v.settings ?? {}).length, 0);
			const inRanges = ranges.reduce((n, r) => n + Object.keys(r.settings ?? {}).length, 0);
			out[o.id] = {
				settings: own + inParts + inRanges,
				ranges: ranges.length,
				parts: volumes.length
			};
		}
		return out;
	});
	// Objets du plateau actif dont l'empreinte au sol en chevauche une autre de
	// plus de 0,5 mm (phase 38 : enveloppes convexes XY, rotation et échelle
	// comprises — plus de faux positif après l'arrangement d'objets tournés).
	const overlapCount = $derived.by(() =>
		overlappingCount(activeSceneObjects, [bed.center.x, bed.center.y])
	);
	const visibleSceneObjects = $derived.by(() => {
		const visible = viewportObjects(sceneObjects, activePlate, (id) => tree.isHidden(id));
		const displaced = assemblyExploded
			? new Map(
					explode(
						visible.map((o) => ({ id: o.id, center: o.position ?? [0, 0, 0] })),
						assemblyFactor
					).map((p) => [p.id, p.position])
				)
			: null;
		return visible.map((o) => ({
			...o,
			...(displaced?.has(o.id) ? { position: displaced.get(o.id) } : {}),
			printable: tree.isPrintable(o.id),
			...(volumesByObject[o.id]?.length ? { volumes: volumesByObject[o.id], volumeMeshes } : {})
		}));
	});
	const activeObjectIds = $derived(new Set(activePlate?.objectIds ?? []));
	// Phase 30 : tous les plateaux côte à côte dans la vue 3D (PartPlateList
	// d'Orca). Le plateau actif reste à l'origine ; les autres sont décalés et
	// affichent leurs objets (clic = activer le plateau, et l'objet visé).
	const plateViews = $derived.by((): PlateView[] => {
		const list = plates.list();
		if (list.length < 2) return [];
		const activeIndex = Math.max(0, plates.plateIndex(plates.activeId ?? ''));
		const offsets = plateOffsets(list.length, activeIndex, bed);
		return list.flatMap((plate, index) =>
			index === activeIndex
				? []
				: [
						{
							id: plate.id,
							index,
							name: plate.name,
							offset: offsets[index],
							objects: viewportObjects(sceneObjects, plate, (id) => tree.isHidden(id)).map((o) => ({
								...o,
								printable: tree.isPrintable(o.id),
								...(volumesByObject[o.id]?.length
									? { volumes: volumesByObject[o.id], volumeMeshes }
									: {})
							}))
						}
					]
		);
	});
	const activePlateView = $derived(
		activePlate
			? { id: activePlate.id, index: plates.plateIndex(activePlate.id), name: activePlate.name }
			: null
	);
	// Transformations sauvegardées, indexées par id d'objet (reconstruites à
	// l'ouverture depuis `project.scene`, appliquées quand le maillage arrive).
	// Transitoire (pas d'état réactif) : simple table de correspondance de montage.
	const savedTransformByObject: Record<string, Transform> = {};
	const savedDocumentByObject: Record<string, SceneObjectDoc> = {};
	// Maillage décodé + recentré par modèle, partagé par toutes ses instances
	// (une seule requête `/mesh` par modèle, même avec des appels concurrents).
	// eslint-disable-next-line svelte/prefer-svelte-reactivity
	const meshRequests = new Map<
		string,
		Promise<{ mesh: SceneMesh; center: [number, number, number] }>
	>();
	// Modèles serveur connus de cette session (persistés dans `knownModels`) :
	// distingue à la réouverture un orphelin récent d'une suppression sauvegardée.
	const projectModelIds = new SvelteSet<string>();

	// Presets actifs du projet (T098) : imprimante / filament / process. Peuplés
	// depuis le blob `active_presets` au montage (lecture non réactive de `data`),
	// modifiés par les sélecteurs, persistés au save.
	let activePresets = $state(emptyActivePresets());
	let machinePresets = $state<PresetSummary[]>([]);
	let filamentPresets = $state<PresetSummary[]>([]);
	let processPresets = $state<PresetSummary[]>([]);
	// Imprimantes possédées + catalogue complet (Phase 14, refonte) : le sélecteur
	// d'imprimante de l'éditeur se réduit aux imprimantes déclarées ; le catalogue
	// sert au menu déroulant de buse et à l'ajout d'une nouvelle imprimante.
	let ownedPrinters = $state<PrinterResponse[]>([]);
	let printerCatalog = $state<PrinterCatalogVendor[]>([]);
	// Grille d'ajout d'imprimante (façon OrcaSlicer) ouverte depuis le sélecteur.
	let showCatalog = $state(false);
	let addingPrinter = $state(false);
	// Multi-filament : un sélecteur par slot (slot `i` ↔ extrudeur `i + 1` de la
	// liste d'objets). `filamentSlot` est le slot actif (crayon ✎ / dialogue) ;
	// `filamentSel` en dérive, toujours cohérent avec `activePresets.filaments`
	// (le slot 0 est le filament principal).
	let filamentSlot = $state(0);
	const filamentSlots = $derived(filamentSlotCount(activePresets));
	const filamentSel = $derived(
		filamentAt(activePresets, clampFilamentSlot(activePresets, filamentSlot))
	);
	// Couleur CSS de chaque slot (pastilles colonne Filament / liste d'objets).
	// Config projet multi-filament (couleurs par slot, volumes de purge), Orca :
	// config projet, pas preset (lib/editor/multi-filament).
	let projectFilament = $state<ProjectFilamentConfig>({});
	let flushDialogOpen = $state(false);
	let showCreateFilament = $state(false);
	let showPaHistory = $state(false);
	// Projet de calibration : lecture du résultat (carte) et K proposé à l'historique.
	let calibReading = $state<CalibrationReading | null>(null);
	let paSuggestion = $state<{ k: number; method: PaMethod } | null>(null);
	function applyCalibTemperature(t: number) {
		filamentValues.nozzle_temperature = [t];
		editTouched.filament.set('nozzle_temperature', ++editTouchSeq);
		const id = activePresetOf('filament')?.id;
		if (id) editTouchedPreset.filament.set('nozzle_temperature', id);
		configTab = 'settings';
		openSettingsDialog('filament', 'Filament');
		saveMessage = `Température ${t} °C posée dans le filament : enregistrez ses réglages.`;
	}
	// Valeur K de l'historique PA posée dans le filament actif (brouillon,
	// notée comme une saisie pour survivre à un rechargement de la base).
	function applyPressureAdvance(k: number) {
		filamentValues.enable_pressure_advance = [true];
		filamentValues.pressure_advance = [k];
		for (const key of ['enable_pressure_advance', 'pressure_advance']) {
			editTouched.filament.set(key, ++editTouchSeq);
			const id = activePresetOf('filament')?.id;
			if (id) editTouchedPreset.filament.set(key, id);
		}
		showPaHistory = false;
		configTab = 'settings';
		openSettingsDialog('filament', 'Filament');
		saveMessage = `Pressure advance K = ${k} posé dans le filament : enregistrez ses réglages.`;
	}
	// Multiplicateur de purge effectif hors surcharge projet (config résolue ;
	// défaut Orca 0,3) : valeur initiale du dialogue, pas 1.
	const presetFlushMultiplier = $derived.by(() => {
		for (const v of [
			processValues.flush_multiplier,
			machineValues.flush_multiplier,
			filamentValues.flush_multiplier
		]) {
			const n = Number(Array.isArray(v) ? v[0] : v);
			if (v !== undefined && Number.isFinite(n) && n > 0) return n;
		}
		return DEFAULT_FLUSH_MULTIPLIER;
	});
	// Extrudeur effectif de chaque objet (teinte de la scène en multi-filament ;
	// « auto » = 1, comme Orca).
	const sceneObjectExtruders = $derived.by(() => {
		if (filamentSlots < 2) return null;
		const out: Record<string, number> = {};
		for (const o of sceneObjects)
			out[o.id] = clampExtruder(tree.effectiveExtruder(o.id), filamentSlots) || 1;
		return out;
	});
	// Couleur du preset de chaque slot (avant surcharge projet).
	const presetSlotColors = $derived(
		Array.from({ length: filamentSlots }, (_, i) =>
			rgbToHex(filamentColors?.[i] ?? defaultFilamentColor(i))
		)
	);
	const filamentSlotColors = $derived(effectiveColours(presetSlotColors, projectFilament));
	// Couleurs de l'aperçu (coloration « filament », légende) : surcharges comprises.
	const previewFilamentColors = $derived(
		filamentSlotColors.map((c, i) => parseOrcaColor(c) ?? defaultFilamentColor(i))
	);
	// Recharge filament/process compatibles quand l'imprimante change (baseline
	// posée à la première exécution de l'effet, d'où le sentinel `undefined`).
	let lastPrinter: string | null | undefined = undefined;
	$effect(() => {
		const printer = activePresets.printer;
		if (lastPrinter === undefined) {
			lastPrinter = printer;
			return;
		}
		if (printer !== lastPrinter) {
			lastPrinter = printer;
			void loadPresets();
		}
	});
	// Projet 3MF ouvert (Orca load_project) : un slot par filament du fichier,
	// presets lus dans `filament_settings_id` du process importé (lib/editor/presets).
	async function completeImportedFilaments() {
		const count = projectFilament.filament_colour?.length ?? 0;
		if (count <= activePresets.filaments.length || !activePresets.process) return;
		let names: unknown[] = [];
		try {
			const v = (await getResolvedPreset(activePresets.process)).values as Record<string, unknown>;
			if (Array.isArray(v.filament_settings_id)) names = v.filament_settings_id;
		} catch {
			// Process illisible : slots complétés avec le filament courant.
		}
		const next = importedFilamentSlots(
			activePresets,
			count,
			names,
			filamentPresets,
			filamentPresets[0]?.id ?? null
		);
		if (next) activePresets = next;
	}
	// Choix du preset d'un slot (sélecteur du slot) : le slot devient actif.
	function setFilamentSlot(slot: number, id: string | null) {
		if (id === null || id === filamentAt(activePresets, slot)) return;
		activePresets = setFilament(activePresets, id, slot);
		filamentSlot = clampFilamentSlot(activePresets, slot);
	}
	// Ajoute un slot (copie du dernier filament, sinon premier preset compatible)
	// et le sélectionne.
	function addFilament() {
		const next = addFilamentSlot(activePresets, filamentPresets[0]?.id ?? null);
		if (next === activePresets) return;
		activePresets = next;
		filamentSlot = next.filaments.length - 1;
		// Couleur suivante de la palette d'Orca : le nouveau slot se distingue des autres.
		projectFilament = setSlotColour(
			afterSlotAdded(projectFilament, next.filaments.length),
			next.filaments.length - 1,
			nextFilamentColour(next.filaments.length - 1)
		);
	}
	// Retire un slot : les objets qui l'utilisaient repassent en auto, ceux des
	// slots suivants sont renumérotés pour garder leur filament.
	function removeFilament(slot: number) {
		if (!canRemoveFilamentSlot(activePresets, slot)) return;
		const next = removeFilamentSlot(activePresets, slot);
		for (const node of tree.list()) {
			const extruder = remapExtruderAfterRemoval(node.extruder, slot);
			if (extruder !== node.extruder) tree.setExtruder(node.id, extruder);
		}
		// Volumes : même renumérotation que les objets (0 = hérite de l'objet).
		for (const [objectId, volumes] of Object.entries(volumesByObject))
			for (const v of volumes) {
				const ex = remapExtruderAfterRemoval(v.extruder ?? 0, slot);
				if (ex !== (v.extruder ?? 0))
					updateVolume(objectId, v.id, { extruder: ex > 0 ? ex : undefined });
			}
		afterFilamentSlotRemoved(slot, (s) => remapExtruderAfterRemoval(s, slot));
		projectFilament = afterSlotRemoved(projectFilament, slot);
		filamentSlot = slotAfterRemoval(filamentSlot, slot, next.filaments.length);
		activePresets = next;
	}

	// Phase 37 — « Fusionner avec » (Orca create_filament_action_menu →
	// Sidebar::change_filament → delete_filament(from, to)) : objets, volumes et
	// changements de filament du slot retiré passent au filament cible, puis le
	// slot est supprimé (numérotation des suivants décalée d'un cran).
	function mergeFilament(slot: number, target: number) {
		if (slot === target || !canRemoveFilamentSlot(activePresets, slot)) return;
		const next = removeFilamentSlot(activePresets, slot);
		for (const node of tree.list()) {
			const extruder = remapExtruderAfterMerge(node.extruder, slot, target);
			if (extruder !== node.extruder) tree.setExtruder(node.id, extruder);
		}
		for (const [objectId, volumes] of Object.entries(volumesByObject))
			for (const v of volumes) {
				const ex = remapExtruderAfterMerge(v.extruder ?? 0, slot, target);
				if (ex !== (v.extruder ?? 0))
					updateVolume(objectId, v.id, { extruder: ex > 0 ? ex : undefined });
			}
		for (const plate of plates.list())
			if (plate.customGcodes?.length)
				plates.setCustomGcodes(
					plate.id,
					remapCodesAfterFilamentRemoval(plate.customGcodes, slot, target)
				);
		const repainted = afterFilamentSlotRemoved(slot, (s) =>
			remapExtruderAfterMerge(s, slot, target)
		);
		projectFilament = afterSlotRemoved(projectFilament, slot);
		filamentSlot = slotAfterRemoval(filamentSlot, slot, next.filaments.length);
		activePresets = next;
		saveMessage = `Filament ${slot + 1} fusionné avec le filament ${target + 1} : objets, pièces${repainted ? ', peinture' : ''} et filaments de support réaffectés.`;
	}

	/**
	 * Phase 38 — suite d'un retrait de slot (Orca Plater::on_filaments_delete) :
	 * peinture MMU renumérotée (TriangleSelector::deserialize avec
	 * to_delete / replace : feuilles du filament retiré → cible ou non peintes,
	 * suivantes décalées), puis support_filament / support_interface_filament
	 * (process et réglages par objet / pièce) : retiré → « Défaut », suivants
	 * décalés. Renvoie le nombre d'objets dont la peinture a changé.
	 */
	function afterFilamentSlotRemoved(removed: number, remapState: (s: number) => number): number {
		let repainted = 0;
		for (const doc of Object.values(paintings)) {
			const mmu = doc.serialize().mmu;
			if (!mmu || Object.keys(mmu).length === 0) continue;
			const next = remapChannelStates(mmu, remapState);
			if (JSON.stringify(next) === JSON.stringify(mmu)) continue;
			doc.replaceChannel('mmu', next);
			repainted++;
		}
		if (repainted) paintings = { ...paintings };
		const processLoaded = Object.keys(processValues).length > 0;
		for (const key of SUPPORT_FILAMENT_KEYS) {
			const current = processValues[key] ?? processResolvedValues[key];
			const nextValue = supportFilamentAfterRemoval(current, removed);
			if (processLoaded && nextValue !== null && String(nextValue) !== String(current)) {
				processValues = { ...processValues, [key]: nextValue };
				editTouched.process.set(key, ++editTouchSeq);
				const presetId = activePresetOf('process')?.id;
				if (presetId) editTouchedPreset.process.set(key, presetId);
			}
			for (const node of tree.list()) {
				if (!(key in (node.settings ?? {}))) continue;
				const v = supportFilamentAfterRemoval(node.settings[key], removed);
				if (v !== null && String(v) !== String(node.settings[key]))
					tree.setSetting(node.id, key, v);
			}
			for (const [objectId, volumes] of Object.entries(volumesByObject))
				for (const vol of volumes) {
					if (!vol.settings || !(key in vol.settings)) continue;
					const v = supportFilamentAfterRemoval(vol.settings[key], removed);
					if (v !== null && String(v) !== String(vol.settings[key]))
						updateVolume(objectId, vol.id, { settings: { ...vol.settings, [key]: v } });
				}
		}
		return repainted;
	}

	// Phase 37 — « Déplacer le plateau vers l'avant » (PartPlate, icône 7).
	function movePlateToFront(id: string) {
		const name = plates.get(id)?.name;
		if (!plates.moveToFront(id)) return;
		ws = setSelection(ws, new Set());
		saveMessage = `« ${name} » est maintenant le premier plateau.`;
	}

	// Phase 37 — tableau des paramètres d'objets (ObjectGridTable).
	const objectTableObjects = $derived.by((): TableObjectSource[] => {
		if (!objectTableOpen) return [];
		const list = plates.list();
		return tree
			.list()
			.filter((n) => n.kind === 'object')
			.map((n) => ({
				id: n.id,
				name: n.name,
				plate: list.findIndex((p) => p.objectIds.includes(n.id)) + 1,
				printable: n.printable,
				extruder: n.extruder,
				settings: n.settings,
				volumes: volumesByObject[n.id] ?? []
			}));
	});
	function openObjectTable(focus: string | null = null) {
		objectTableFocus = focus;
		objectTableOpen = true;
	}
	function onTableSetting(row: TableRow, key: string, value: unknown) {
		if (row.volumeId) {
			const v = (volumesByObject[row.objectId] ?? []).find((x) => x.id === row.volumeId);
			if (!v) return;
			const settings = { ...v.settings };
			if (value === undefined) delete settings[key];
			else settings[key] = value;
			updateVolume(row.objectId, v.id, { settings });
		} else if (key === 'extruder') tree.setExtruder(row.objectId, 0);
		else tree.setSetting(row.objectId, key, value);
	}
	function onTableExtruder(row: TableRow, extruder: number) {
		if (row.volumeId)
			updateVolume(row.objectId, row.volumeId, { extruder: extruder > 0 ? extruder : undefined });
		else tree.setExtruder(row.objectId, extruder);
	}
	function onTableRename(row: TableRow, name: string) {
		if (row.volumeId) updateVolume(row.objectId, row.volumeId, { name });
		else tree.rename(row.objectId, name);
	}

	// Phase 37 — « Changer le type » d'une pièce (ObjectList::change_part_type).
	function setVolumeType(objectId: string, volumeId: string, type: VolumeType) {
		const v = (volumesByObject[objectId] ?? []).find((x) => x.id === volumeId);
		const next = v ? changeVolumeType(v, type) : null;
		if (!v || !next) return;
		replaceVolume(objectId, next);
		saveMessage = `« ${v.name} » : ${volumeTypeName(v.type)} → ${volumeTypeName(type)}.`;
	}
	function openVolumeContext(objectId: string, volumeId: string, x: number, y: number) {
		ws = setSelection(ws, new Set([objectId]));
		selectedVolumeId = volumeId;
		contextMenu = { open: true, x, y, targetId: objectId, part: false };
	}
	// Entrées ajoutées en tête du menu objet (phase 37).
	const objectMenuExtras = $derived.by(() => {
		const id = contextMenu.targetId;
		if (!contextMenu.open || !id || contextMenu.part) return [];
		const volume = (volumesByObject[id] ?? []).find((v) => v.id === selectedVolumeId);
		const node = tree.get(id);
		const extras: (
			| {
					label: string;
					action: string;
					section?: true;
					items?: { label: string; action: string }[];
			  }
			| 'separator'
		)[] = [];
		if (volume)
			extras.push(
				{
					label: `Changer le type de « ${volume.name} »`,
					action: 'section.volumeType',
					section: true,
					items: partTypeChoices(volume)
						.filter((c) => c.type !== volume.type)
						.map((c) => ({ label: c.label, action: `volume.type:${c.type}` }))
				},
				'separator'
			);
		extras.push({
			label: 'Modifier dans la Table des Paramètres',
			action: 'object.parameterTable'
		});
		if (node?.cutId)
			extras.push(
				{ label: 'Sélectionner les pièces liées', action: 'object.selectCutParts' },
				{ label: 'Invalider les informations de découpe', action: 'object.invalidateCut' }
			);
		extras.push('separator');
		return extras;
	});

	// Phase 37 — informations de coupe : pièces liées (même cutId.id).
	const cutLinks = $derived.by(() => {
		const nodes = tree.list();
		const out: Record<string, number> = {};
		for (const n of nodes) if (n.cutId) out[n.id] = cutLinkedIds(nodes, n.id).length - 1;
		return out;
	});
	function selectCutParts(id: string) {
		const ids = cutLinkedIds(tree.list(), id).filter((x) => activePlate?.objectIds.includes(x));
		ws = setSelection(ws, new Set(ids));
		saveMessage = `${ids.length} pièce(s) de la même coupe sélectionnée(s).`;
	}
	async function invalidateCutInfo(id: string) {
		if (!tree.get(id)?.cutId) return;
		const saved = await save({ quiet: true });
		if (saved.status !== 'saved') {
			saveMessage = `Informations de découpe : ${saved.status === 'error' ? saved.message : 'sauvegarde impossible'}`;
			return;
		}
		try {
			const res = await invalidateCut(data.project.id, id, Number(data.project.version));
			const expected = invalidatedIds(tree.list(), id);
			for (const objectId of new Set([...res.object_ids, ...expected])) {
				const node = tree.get(objectId);
				if (node) delete node.cutId;
				const vols = volumesByObject[objectId];
				if (vols?.some((v) => v.cutInfo))
					volumesByObject = {
						...volumesByObject,
						[objectId]: vols.map((v) => {
							const { cutInfo: _drop, ...rest } = v;
							void _drop;
							return rest;
						})
					};
			}
			data.project.version = BigInt(res.project.version);
			data.project.scene = res.project.scene;
			savedSignature = saveSignature();
			currentSignature = savedSignature;
			saveMessage = `Informations de découpe retirées de ${res.object_ids.length} pièce(s) : elles se manipulent maintenant séparément.`;
		} catch (e) {
			saveMessage = `Informations de découpe : ${e instanceof ApiError ? e.message : 'échec'}`;
		}
	}

	// Phase 37 — erreurs de maillage (icône d'alerte de la liste d'objets).
	let meshErrorsSeq = 0;
	async function refreshMeshErrors() {
		const seq = ++meshErrorsSeq;
		try {
			const list = await projectMeshErrors(data.project.id);
			if (seq !== meshErrorsSeq) return;
			const byPersisted = toMeshWarnings(list);
			const live: Record<string, MeshWarning> = {};
			for (const [id, w] of Object.entries(byPersisted)) live[persistedToLive.get(id) ?? id] = w;
			meshWarningsByObject = live;
		} catch {
			// Erreurs de maillage : indicatif, silencieux en cas d'échec.
		}
	}
	async function repairFromWarning(id: string) {
		await repairObject(id);
		const { [id]: _done, ...rest } = meshWarningsByObject;
		void _done;
		meshWarningsByObject = rest;
	}

	// Phase 37 — navigateur 3D : vue nommée, rejouée même si déjà active.
	async function selectNamedView(view: NamedView) {
		if (cameraView === view) {
			cameraView = view === 'default' ? 'top' : 'default';
			await tick();
		}
		cameraView = view;
		cameraFocus = null;
	}

	// Phase 37 — type de débit de buse par extrudeur (ExtruderGroup « Débit »).
	const nozzleFlows = $derived.by(() => nozzleFlowChoices(machineValues));
	function setExtruderFlow(index: number, flow: NozzleFlow) {
		machineValues = {
			...machineValues,
			nozzle_volume_type: setNozzleFlow(machineValues, index, flow)
		};
		editTouched.machine.set('nozzle_volume_type', ++editTouchSeq);
		const presetId = activePresetOf('machine')?.id;
		if (presetId) editTouchedPreset.machine.set('nozzle_volume_type', presetId);
		saveMessage = `Extrudeur ${index + 1} : débit ${NOZZLE_FLOW_LABELS[flow]} (réglage de l'imprimante modifié).`;
	}

	// Onglet de la colonne de configuration : liste d'objets/plateaux, réglages
	// ou métadonnées du projet (lot 34).
	let configTab = $state<'objects' | 'settings' | 'project'>('settings');
	let settingsMode = $state<DisplayMode>('simple');
	// Portée des réglages affichés (T100/T101/T102) : process / filament / machine,
	// chacun avec son propre jeu de valeurs éditées.
	// Dialogue de réglages ouvert (Phase 14, T138) ; `null` = fermé.
	let settingsDialog = $state<null | 'process' | 'filament' | 'machine'>(null);
	// « Enregistrer » sur un preset système : nom de la copie demandé comme Orca
	// (champ du pied du dialogue, nom proposé, doublons refusés).
	let saveAsName = $state<string | null>(null);
	let saveAsFor: string | null = null;
	const presetsOfKind = (kind: 'process' | 'filament' | 'machine') =>
		kind === 'process' ? processPresets : kind === 'filament' ? filamentPresets : machinePresets;
	$effect(() => {
		const kind = settingsDialog;
		const preset = kind ? activePresetOf(kind) : undefined;
		const key = kind && preset ? `${kind}:${preset.id}` : null;
		if (key === saveAsFor) return;
		saveAsFor = key;
		saveAsName =
			kind && preset && preset.origin !== 'user'
				? uniqueCopyName(
						preset.name,
						untrack(() => presetsOfKind(kind)).map((x) => x.name)
					)
				: null;
	});
	const saveAsError = $derived.by(() => {
		if (saveAsName === null || !settingsDialog) return null;
		const name = saveAsName.trim();
		if (!name) return 'Donnez un nom au preset.';
		if (presetsOfKind(settingsDialog).some((x) => x.name === name))
			return `Un preset « ${name} » existe déjà.`;
		return null;
	});
	// Dialogue « Réglages du plateau » (surcharges par plateau).
	let plateSettingsOpen = $state(false);
	let processValues = $state<Record<string, unknown>>({});
	// Tour d'essuyage active (résumé de la colonne Filament en multi-filament).
	const primeTowerOn = $derived(
		processValues.enable_prime_tower === true ||
			processValues.enable_prime_tower === '1' ||
			processValues.enable_prime_tower === 1
	);
	let filamentValues = $state<Record<string, unknown>>({});
	let machineValues = $state<Record<string, unknown>>({});
	let processResolvedValues = $state<Record<string, unknown>>({});
	let filamentResolvedValues = $state<Record<string, unknown>>({});
	let machineResolvedValues = $state<Record<string, unknown>>({});
	// Plateau par défaut tant que le preset machine n'est pas résolu, puis contour
	// et hauteur effectifs dès que `/resolved` répond (ou que le preset change).
	const bed = $derived(bedFromValues(machineResolvedValues));
	const processModified = $derived(hasPresetChanges(processResolvedValues, processValues));
	const filamentModified = $derived(hasPresetChanges(filamentResolvedValues, filamentValues));
	const machineModified = $derived(hasPresetChanges(machineResolvedValues, machineValues));
	let saveMessage = $state<string | null>(null);
	// Estompage de la barre de messages : 6 s après chaque message ; le texte
	// reste dans le DOM jusqu'au suivant.
	const MESSAGE_FADE_MS = 6000;
	let messageFaded = $state(false);
	// Phase 31 : chaque affectation est consommée (remise à null) et devient une
	// notification numérotée → un message identique au précédent réapparaît.
	let notice = $state<Notice | null>(null);
	$effect(() => {
		const text = saveMessage;
		if (text === null) return;
		notice = nextNotice(
			untrack(() => notice),
			text
		);
		saveMessage = null;
	});
	$effect(() => {
		if (!notice) return;
		void notice.seq;
		messageFaded = false;
		const timer = setTimeout(() => (messageFaded = true), MESSAGE_FADE_MS);
		return () => clearTimeout(timer);
	});

	// Tranchage + aperçu G-code (T088) : machine à états pure `session` pilotée
	// par les événements WebSocket ; le G-code prêt est chargé paresseusement.
	let session = $state(prepareSession());
	// Résultat mémorisé par plateau et lot en cours (« Trancher tout », phase 25) :
	// changer d'onglet de plateau affiche le résultat de ce plateau, comme Orca.
	let plateResults = $state<PlateResults>({});
	let sliceBatch = $state<SliceBatch | null>(null);
	/** Plateau (rang d'onglet) du dernier résultat restauré à l'ouverture. */
	let restoredPlateIndex: number | null = null;
	let previewMeta = $state<PreviewMeta | null>(null);
	let previewGeometry = $state<PreviewGeometry | null>(null);
	let rawStats = $state<unknown>(null);
	let loader: LazyPreviewLoader | null = null;
	// Contrôles d'aperçu (T116) : coloration active, plage de couches (double
	// curseur), types de ligne masqués.
	let coloration = $state<Coloration>('type');
	let filamentColors = $state<Rgb[] | undefined>(undefined);
	let filamentColorsGeneration = 0;
	$effect(() => {
		const ids = [...activePresets.filaments];
		const generation = ++filamentColorsGeneration;
		void loadFilamentColors(ids).then((colors) => {
			if (generation === filamentColorsGeneration) filamentColors = colors;
		});
	});
	let layerRange = $state<LayerRange>(makeLayerRange(1));
	let moveCursor = $state(makeMoveCursor(0));
	let moveCursorLayer = -1;
	let hiddenTypes = $state<number[]>([]);
	const COLORATIONS: { id: Coloration; label: string }[] = [
		{ id: 'type', label: 'Type de ligne' },
		{ id: 'speed', label: 'Vitesse' },
		{ id: 'height', label: 'Hauteur de couche' },
		{ id: 'width', label: 'Largeur de ligne' },
		{ id: 'flow', label: 'Débit' },
		{ id: 'temperature', label: 'Température' },
		{ id: 'fan', label: 'Ventilateur' },
		{ id: 'layer-time', label: 'Temps de couche (linéaire)' },
		{ id: 'layer-time-log', label: 'Temps de couche (logarithmique)' },
		{ id: 'acceleration', label: 'Accélération' },
		{ id: 'jerk', label: 'Jerk' },
		{ id: 'pa', label: 'Avance de pression' },
		{ id: 'actual-speed', label: 'Vitesse réelle' },
		{ id: 'actual-flow', label: 'Débit réel' },
		{ id: 'filament', label: 'Filament' },
		{ id: 'color-print', label: 'Couleur d’impression' }
	];

	// Piste G-code complète (lib/preview/track.ts) : grandeurs absentes du modèle
	// WSPv (ventilateur, température, temps de couche…) et options d'affichage
	// d'Orca (déplacements, rétractions, coutures…). Ignorée si désalignée.
	let previewTrack = $state<PreviewTrack | null>(null);
	// Simulation GCodeProcessor d'Orca (moteur) du G-code tranché affiché :
	// valeurs réelles (vitesse, débit, temps de couche…) et vue Résumé. La piste
	// texte (track.ts) reste le repli (G-code local, échec moteur).
	// Source moteur : G-code tranché ou G-code externe simulé (lot 31).
	let engineSource: EngineSource | null = null;
	let engineMeta = $state<EngineMeta | null>(null);
	// Coutures EMoveType::Seam du moteur (lot 31), chargées une fois par source.
	let engineSeams: EngineSeam[] | null = null;
	/** Lot 31 superposé à la piste moteur : vue « Couleur » et coutures. */
	function decorateEngine(
		track: PreviewTrack,
		meta: EngineMeta,
		chunks: readonly EngineMoves[]
	): PreviewTrack {
		const cp = meta.color_print;
		let out = cp
			? withColorPrint(
					track,
					meta,
					chunks,
					engineSource?.viewer ? (cp.viewer_palette ?? cp.palette) : cp.palette
				)
			: track;
		if (engineSeams) out = withEngineSeams(out, meta, engineSeams);
		return out;
	}
	let engineTimeMode = $state(0);
	// Piste texte de référence (alignée WSPv) et données moteur sur lesquelles
	// la piste affichée est recalculée (fenêtre de couches, mode N/S).
	let engineBase: PreviewTrack | null = null;
	let engineCache: EngineMoveCache | null = null;
	let engineFullMoves: EngineMoves | null = null; // méta v1 : tout chargé
	let engineSync = 0;
	// Colorations qui lisent une grandeur par segment : seules elles exigent les
	// mouvements ; les autres (type, temps de couche…) n'utilisent que le méta.
	const ENGINE_SEGMENT_COLORATIONS = new Set<Coloration>([
		'fan',
		'temperature',
		'acceleration',
		'jerk',
		'pa',
		'actual-speed',
		'actual-flow',
		'color-print'
	]);
	/** Recalcule la piste moteur pour les couches affichées et le mode N/S. */
	async function syncEngineTrack() {
		const base = engineBase;
		const meta = engineMeta;
		if (!base || !meta) return;
		if (previewMeta && !trackMatches(base, previewMeta.layer_count)) return;
		const ticket = ++engineSync;
		let track: PreviewTrack;
		if (engineCache) {
			const [from, to] = engineLayersFor(base, meta, layerRange.low, layerRange.high);
			// Rendu immédiat avec les blocs déjà là, puis complet une fois chargés.
			const loaded = engineCache.loaded();
			previewTrack = decorateEngine(applyEngine(base, meta, loaded, engineTimeMode), meta, loaded);
			if (!ENGINE_SEGMENT_COLORATIONS.has(coloration)) {
				void refreshGeometry();
				return;
			}
			const chunks = await engineCache.ensure(from, to);
			if (ticket !== engineSync || base !== engineBase) return;
			track = decorateEngine(applyEngine(base, meta, chunks, engineTimeMode), meta, chunks);
		} else if (engineFullMoves) {
			track = decorateEngine(withEngine(base, engineFullMoves, engineTimeMode), meta, [
				engineFullMoves
			]);
		} else return;
		previewTrack = track;
		void refreshGeometry();
	}
	$effect(() => {
		// Dépendances : fenêtre de couches et mode d'estimation.
		void layerRange.low;
		void layerRange.high;
		void engineTimeMode;
		void coloration;
		// Le recalcul (et refreshGeometry) ne doit pas devenir une dépendance.
		if (engineMeta) untrack(() => void syncEngineTrack());
	});
	let previewFlowRange = $state<[number, number]>([0, 0]);
	// Cadrage de l'aperçu (Orca : zoom sur la boîte du G-code au premier
	// affichage) ; « Plateau » reste disponible. Boîte figée au 1er rendu.
	let previewFrame = $state<{
		min: [number, number, number];
		max: [number, number, number];
	} | null>(null);
	let previewFit = $state<'objects' | 'bed'>('objects');
	// Panneau de légende flottant (repliable) de l'aperçu.
	let legendOpen = $state(true);
	// Rendu des extrusions (volumes éclairés comme Orca, lignes en repli ;
	// « auto » bascule en lignes au-delà de VOLUME_SEGMENT_LIMIT segments).
	let renderMode = $state<RenderMode>('auto');
	// Rendu WebGL logiciel détecté (phase 25) : « auto » rend en lignes.
	const softwareGl = typeof document !== 'undefined' && detectSoftwareRenderer();
	const effectiveRender = $derived(
		effectiveRenderMode(renderMode, previewGeometry?.visibleCount ?? 0, softwareGl)
	);
	/** Position relative (0–1) d'une couche sur la piste du curseur. */
	function layerPct(layer: number): number {
		const n = (previewMeta?.layer_count ?? 1) - 1;
		return n > 0 ? layer / n : 0;
	}
	let previewFrameKey = $state(0);
	function setPreviewFit(fit: 'objects' | 'bed') {
		previewFit = fit;
		previewFrameKey++;
	}
	let previewTrackLoading = $state(false);
	let previewOptions = $state<PreviewOption[]>([]);
	const previewOptionSet = $derived(new Set(previewOptions));
	const previewOverlay = $derived(
		previewTrack && previewOptions.length
			? buildOverlay(
					previewTrack,
					{ low: visibleRange(layerRange).from, high: visibleRange(layerRange).to },
					previewOptionSet
				)
			: null
	);
	// Nombre d'éléments par option dans les couches visibles (légende).
	const previewOptionCounts = $derived.by(() => {
		if (!previewTrack) return [];
		const { from, to } = visibleRange(layerRange);
		const layers = previewTrack.layers.slice(from, to + 1);
		return PREVIEW_OPTIONS.map((id) => ({
			id,
			count: layers.reduce(
				(n, l) =>
					n +
					(id === 'travel'
						? l.travels.length / 6
						: id === 'wipe'
							? l.wipes.length / 6
							: l.markers[id].length / 3),
				0
			)
		})).filter((o) => o.count > 0 || o.id === 'travel' || o.id === 'retract' || o.id === 'seam');
	});
	function togglePreviewOption(id: PreviewOption) {
		previewOptions = previewOptions.includes(id)
			? previewOptions.filter((o) => o !== id)
			: [...previewOptions, id];
	}
	// Temps nominal de la couche haute visible (curseur vertical, comme Orca).
	const topLayerTime = $derived(previewTrack?.layers[layerRange.high]?.time ?? null);
	// Phase 31 : sommet courant du curseur horizontal (marqueur + fenêtre
	// « position de l'outil » d'Orca) et « Aller à la couche » (Maj+G).
	let previewTool = $state<ToolPosition | null>(null);
	// Phase 33 (lot 31) : temps estimé au sommet courant (get_estimated_time_at
	// d'Orca : layers[].elapsed + temps des mouvements simulés) et repères du
	// curseur de couches (changements de couleur, pauses, G-code perso).
	let previewToolTime = $state<EstimatedTime | null>(null);
	let toolTimeSeq = 0;
	$effect(() => {
		const tool = previewTool;
		const meta = engineMeta;
		const mode = engineTimeMode;
		const atEnd = moveCursor.position >= moveCursor.total;
		untrack(() => void updateToolTime(tool, meta, mode, atEnd));
	});
	async function updateToolTime(
		tool: ToolPosition | null,
		meta: EngineMeta | null,
		mode: number,
		atEnd: boolean
	) {
		const seq = ++toolTimeSeq;
		const base = engineBase;
		if (!tool || !meta || !base) {
			previewToolTime = null;
			return;
		}
		// Fin de couche (position par défaut) : méta seule, aucun mouvement chargé
		// (les colorations par type / temps restent sans téléchargement WSPm).
		if (atEnd && tool.index >= 0) {
			previewToolTime = layerEndTime(base, meta, tool.layer, mode);
			return;
		}
		const loaded = engineFullMoves ? [engineFullMoves] : (engineCache?.loaded() ?? []);
		let time = estimatedTimeAtTool(base, meta, loaded, tool.layer, tool.index, mode);
		if (!time && engineCache && tool.index >= 0) {
			// Mouvements de la couche pas encore chargés : un bloc suffit.
			const [from, to] = engineLayersFor(base, meta, tool.layer, tool.layer);
			try {
				const chunks = await engineCache.ensure(from, to);
				if (seq !== toolTimeSeq) return;
				time = estimatedTimeAtTool(base, meta, chunks, tool.layer, tool.index, mode);
			} catch {
				time = null;
			}
		}
		if (seq === toolTimeSeq) previewToolTime = time;
	}
	const previewLayerTicks = $derived(
		engineMeta && previewTrack
			? withAuthoredTypes(
					layerTicks(previewTrack, engineMeta),
					activePlate?.customGcodes ?? [],
					previewMeta?.layers ?? []
				)
			: []
	);
	// Phase 34 : noms des pièces par objet (recherche de la liste d'objets).
	const objectPartNames = $derived(
		Object.fromEntries(
			Object.entries(volumesByObject).map(([id, list]) => [id, (list ?? []).map((v) => v.name)])
		)
	);
	// Phase 34 : codes du plateau ajoutés depuis le tranchage (« + » en Aperçu),
	// en pointillé sur le curseur jusqu'au prochain tranchage.
	const pendingLayerTicks = $derived(
		previewMeta
			? pendingTicks(activePlate?.customGcodes ?? [], previewMeta.layers, previewLayerTicks)
			: []
	);
	const codeAtHighLayer = $derived.by(() => {
		const z = previewMeta?.layers[layerRange.high]?.z;
		return z !== undefined && codeIndexAtZ(activePlate?.customGcodes ?? [], z) >= 0;
	});
	const TICK_COLORS: Record<string, string> = {
		color_change: '#da948b',
		pause_print: '#52f083',
		custom: '#5ed194',
		template: '#5ed194',
		tool_change: '#e8b04a'
	};
	let goToLayerOpen = $state(false);
	let goToLayerValue = $state('');
	function openGoToLayer() {
		goToLayerValue = String(
			(layerRange.active === 'low' && !layerRange.oneLayerMode ? layerRange.low : layerRange.high) +
				1
		);
		goToLayerOpen = true;
	}
	function submitGoToLayer() {
		const n = Number(goToLayerValue);
		if (!Number.isInteger(n) || !previewMeta || n < 1 || n > previewMeta.layer_count) return;
		layerRange = jumpToLayer(layerRange, n - 1);
		goToLayerOpen = false;
		void refreshGeometry();
	}
	// Plateaux dans l'aperçu : résultat valable par plateau (+ plateau affiché).
	const previewSlicedPlates = $derived.by(() => {
		const ids = Object.keys(plateResults);
		const active = plates.activeId;
		if (session.phase === 'preview' && session.gcodeId && active && !ids.includes(active))
			ids.push(active);
		return ids;
	});
	const previewPlatesTotals = $derived(
		platesTotals(
			previewSlicedPlates,
			(id) =>
				plateResults[id]?.stats ??
				(id === plates.activeId && session.phase === 'preview' ? rawStats : null)
		)
	);

	// Statistiques serveur de tous les plateaux tranchés (lot 31,
	// render_all_plates_stats : filament par usage cumulé, temps, coût).
	let platesStats = $state<PlatesStatsResponse | null>(null);
	let platesStatsKey = '';
	$effect(() => {
		const ids = previewSlicedPlates
			.map(
				(id) =>
					plateResults[id]?.gcodeId ??
					(id === plates.activeId && session.phase === 'preview' ? session.gcodeId : null)
			)
			.filter((g): g is string => typeof g === 'string' && g.length > 0);
		const key = [...new Set(ids)].join(',');
		if (key === platesStatsKey) return;
		platesStatsKey = key;
		if (ids.length < 2) {
			platesStats = null;
			return;
		}
		getPlatesStats([...new Set(ids)])
			.then((r) => {
				if (platesStatsKey === key) platesStats = r;
			})
			.catch(() => {
				if (platesStatsKey === key) platesStats = null;
			});
	});

	const previewStats = $derived(
		rawStats
			? buildPreviewStats(rawStats as GcodeStats, undefined, previewMeta?.types_present ?? [])
			: null
	);
	// Import de modèles (T089) : file d'imports suivie + zone de dépôt.
	let imports = $state<ImportItem[]>([]);
	// Fichiers choisis mais pas encore pris en charge (l'éditeur attend presets et
	// plateau, `editorReady`, jusqu'à ~2 s à l'ouverture) : état de chargement
	// explicite au lieu d'un plateau vide qui invite encore à « glisser un modèle ».
	let queuedImports = $state<string[]>([]);
	let importError = $state<string | null>(null);
	let dragOver = $state(false);
	let fileInput: HTMLInputElement | null = null;
	let replacementInput: HTMLInputElement | null = null;
	let replacementTargetIds: string[] = [];
	let projectFileInput: HTMLInputElement | null = null;
	let zipFileInput: HTMLInputElement | null = null;
	let configFileInput: HTMLInputElement | null = null;
	const ACCEPT = '.stl,.obj,.mtl,.3mf,.oltp,.step,.stp,.amf,.svg,.drc';
	const canSliceNow = $derived(
		activeSceneObjects.length > 0 &&
			!imports.some((item) => isPending(item)) &&
			!!activePresets.process &&
			activePresets.filaments.length > 0
	);
	// « Trancher tout » : au moins un objet sur un plateau (les plateaux vides sont sautés).
	const canSliceAll = $derived(
		sceneObjects.length > 0 &&
			!imports.some((item) => isPending(item)) &&
			!!activePresets.process &&
			activePresets.filaments.length > 0
	);
	// Raison affichée au survol quand le tranchage est impossible.
	const sliceBlocker = $derived(
		activeSceneObjects.length === 0
			? 'Ajoutez un objet à trancher'
			: !activePresets.process
				? 'Choisissez un profil de traitement'
				: activePresets.filaments.length === 0
					? 'Choisissez un filament'
					: 'Import en cours…'
	);

	// Profils par défaut d'un nouveau projet (comme Orca : default_print_profile /
	// default_filament_profile du preset machine, sinon un profil standard), une
	// fois les listes compatibles chargées.
	function pickDefault(
		list: readonly { id: string; name: string }[],
		preferred: unknown,
		fallback: RegExp
	): string | null {
		const names = (Array.isArray(preferred) ? preferred : [preferred])
			.flatMap((v) => (typeof v === 'string' ? v.split(';') : []))
			.map((v) => v.trim().replace(/^"|"$/g, ''))
			.filter(Boolean);
		for (const name of names) {
			const hit = list.find((p) => p.name === name);
			if (hit) return hit.id;
		}
		return (list.find((p) => fallback.test(p.name)) ?? list[0])?.id ?? null;
	}
	$effect(() => {
		if (!activePresets.printer) return;
		if (!activePresets.process && processPresets.length > 0) {
			const id = pickDefault(
				processPresets,
				machineResolvedValues.default_print_profile,
				/0\.20mm Standard/
			);
			if (id) activePresets.process = id;
		}
		if (activePresets.filaments.length === 0 && filamentPresets.length > 0) {
			// Filaments cochés à l'assistant (phase 25) en tête, défaut machine ensuite.
			const id = pickDefault(
				filamentPresets,
				preferredFilaments(
					machineResolvedValues.default_filament_profile,
					untrack(() => currentPreferences().wizardFilaments)
				),
				/Generic PLA/
			);
			if (id) activePresets = setFilament(activePresets, id, 0);
		}
	});

	// Seuls les imports créés dans cette session sont supprimés en cas d'échec.
	// Les modèles déjà rattachés au projet conservent leur badge d'erreur au
	// rechargement, au lieu de faire disparaître un objet potentiellement sauvé.
	const rollbackImportIds = new SvelteSet<string>();
	let sceneGeneration = 0;
	// Le contour machine détermine le placement initial des objets. Imports et
	// tranchage attendent cette initialisation pour ne jamais figer le plateau
	// provisoire de 256 mm dans la scène persistée.
	let editorReady: Promise<void> = Promise.resolve();

	onMount(() => {
		let alive = true;
		// Thème : l'éditeur suit la préférence de l'utilisateur (clair / sombre /
		// système), vue 3D comprise (palette `sceneColors`, lib/scene/scene-colors.ts).

		// Presets actifs du projet (lecture non réactive de `data` au montage, T098).
		activePresets = parseActivePresets(data.project.active_presets);
		filamentSlot = 0;
		projectName = data.project.name;
		savedName = data.project.name;

		draftStore.pendingRestore(data.project.id, data.project.updated_at).then((d) => {
			if (alive) pendingDraft = d;
		});
		// Résout le plateau réel avant de restaurer ou importer des objets.
		editorReady = (async () => {
			await loadPresetsShared();
			await loadEditedValues('machine');
			await loadProjectModels();
			await completeImportedFilaments();
		})();
		// Référence de l'indicateur « non enregistré » : prise après restauration.
		void editorReady.then(() => {
			if (!alive) return;
			// Phase 29 : référence prise ICI, avant qu'un import lancé pendant
			// l'ouverture (il attend `editorReady`) ne modifie la scène — sinon un
			// premier import passait pour « enregistré » (ni pastille, ni
			// sauvegarde avant la validation en direct, qui voyait un plateau vide).
			// Phase 32 : modèles orphelins restaurés (import jamais enregistré, onglet
			// fermé ou rechargé) → la scène affichée n'est PAS celle du serveur.
			// Référence volontairement différente (sans suffixe de presets) : la
			// pastille « non enregistré » s'allume et « Trancher » enregistre d'abord,
			// au lieu de trancher la scène serveur vide (« plateau vide »).
			if (restoredOrphans) savedSignature = '\u0000orphelins-restaurés';
			else if (!imports.some((item) => isPending(item))) savedSignature = saveSignature();
			baselineReady = true;
			// Phase 37 : alertes d'erreurs de maillage de la scène enregistrée.
			if (((data.project.scene as { objects?: unknown[] } | null)?.objects?.length ?? 0) > 0)
				void refreshMeshErrors();
		});
		// Restaure le dernier résultat réussi du projet : quitter l'éditeur pour
		// déclarer Moonraker ne doit pas désactiver l'export/envoi au retour.
		void listJobs()
			.then((jobs) => {
				if (!alive || session.phase !== 'prepare' || session.gcodeId) return;
				// Phase 32 : tranchage encore en cours (page rechargée pendant le
				// tranchage) → reprise du suivi : progression, puis aperçu à la fin.
				const plan = planResume(jobs, data.project.id);
				if (plan?.kind === 'slicing') {
					session = {
						...startSlicing(plan.jobs.map((job) => job.id)),
						progress: resumedProgress(plan.jobs)
					};
					saveMessage = 'Tranchage en cours repris après le rechargement de la page.';
					// Fini entre la liste et l'abonnement : état relu une fois.
					void resyncSessionJobs();
					return;
				}
				const latest = plan?.kind === 'result' ? plan.job : null;
				if (!latest?.gcode_id) return;
				session = {
					...prepareSession(),
					phase: 'preview',
					progress: 1,
					gcodeId: latest.gcode_id
				};
				previewLoadedFor = latest.gcode_id;
				restoredPlateIndex = Number(latest.plate_index);
				layout = setTab(layout, 'preview');
				void loadPreview(latest.gcode_id);
			})
			.catch(() => {
				// L'historique est un confort de restauration : la scène et un nouveau
				// tranchage restent utilisables si ce chargement optionnel échoue.
			});
		// Flux d'événements du compte (T065) : progression et fin des jobs.
		// Phase 32 : après une coupure (flux WS ou serveur), les événements émis
		// entre-temps sont perdus → état des jobs suivis relu au retour.
		const sub: EventSubscription = subscribeEvents({
			onEvent,
			onOpen: (reconnected) => {
				if (reconnected) void resyncSessionJobs();
			}
		});
		let lastRestored = 0;
		const unsubConnection = connection.subscribe((c) => {
			if (c.restored === lastRestored) return;
			lastRestored = c.restored;
			if (c.restored > 0) void resyncSessionJobs();
		});
		return () => {
			alive = false;
			sub.close();
			unsubConnection();
		};
	});

	// Ouvre le projet : charge ses modèles, reconstruit l'arbre d'objets et les
	// maillages **décodés par le moteur** (R7/T126). Un maillage déjà prêt est
	// chargé immédiatement (`fetchMesh`) ; sinon l'objet reste en attente de
	// conversion (badge « conversion en cours », résolu par `model.converted`).
	async function loadProjectModels() {
		let models;
		try {
			models = await listProjectModels(data.project.id);
		} catch (e) {
			importError = e instanceof ApiError ? e.message : 'chargement des modèles impossible';
			return;
		}
		// Recharger remplace le document vivant : conserver les tableaux précédents
		// dupliquait les nœuds, imports et maillages à chaque « Recharger ».
		sceneGeneration += 1;
		tree = new ObjectTree();
		plates = new PlateSet();
		sceneObjects = [];
		imports = [];
		paintings = {};
		layerProfiles = {};
		brimEarsByObject = {};
		layerRangesByObject = {};
		volumesByObject = {};
		rollbackImportIds.clear();
		persistedToLive.clear();
		validation = null;
		ws = setSelection(ws, new Set());
		for (const id of Object.keys(savedTransformByObject)) delete savedTransformByObject[id];
		for (const id of Object.keys(savedDocumentByObject)) delete savedDocumentByObject[id];
		meshRequests.clear();
		projectModelIds.clear();
		// Scène persistée : un objet vivant par entrée de `saved.objects` (plusieurs
		// instances peuvent partager un même modèle). Les ids d'objet du document
		// servent à retrouver le plateau d'origine ; l'arbre régénère les ids vivants.
		const saved = parseScene(data.project.scene);
		sceneCalibration = saved.calibration ?? null;
		calibReading = calibrationReading(sceneCalibration);
		sceneCalibrationOverrides = saved.calibrationOverrides ?? null;
		projectFilament = parseProjectConfig(saved.projectConfig);
		const plateByOldId: Record<string, string> = {};
		for (const p of saved.plates.plates ?? []) {
			for (const oid of p.objectIds ?? []) plateByOldId[oid] = p.id;
		}
		const modelById = new Map(models.map((model) => [model.id, model]));
		for (const model of models) projectModelIds.add(model.id);
		const plan = planSceneRestore(
			saved,
			data.project.id,
			models.map((model) => model.id)
		);
		// Restaure la structure de plateaux (noms, types, plateau actif) mais vide
		// les affectations : elles référencent d'anciens ids, réattribués ci-dessous.
		if ((saved.plates.plates ?? []).length > 0) {
			plates = PlateSet.deserialize({
				activeId: saved.plates.activeId ?? null,
				plates: saved.plates.plates.map((p) => ({ ...p, objectIds: [] }))
			});
		}
		const restoredImports: ImportItem[] = [];
		const track = (objectId: string, modelId: string) => {
			const model = modelById.get(modelId);
			if (!model) return;
			// Suivi de l'import (permet la résolution via `model.converted`).
			restoredImports.push(
				markUploaded(startImport(objectId, model.filename), model.id, model.conversion_pending)
			);
		};
		for (const doc of plan.objects) {
			const model = modelById.get(doc.modelId);
			if (!model) continue;
			const objectId = tree.add(doc.name || model.filename).id;
			persistedToLive.set(doc.id, objectId);
			const savedPlate = plateByOldId[doc.id];
			const plateId = (savedPlate && plates.get(savedPlate) ? savedPlate : null) ?? plates.activeId;
			if (plateId) plates.assign(objectId, plateId);
			const restored = deserializeObject(doc);
			savedTransformByObject[objectId] = restored.transform;
			savedDocumentByObject[objectId] = doc;
			const node = tree.get(objectId);
			if (node) {
				node.extruder = restored.extruder;
				node.printable = restored.printable;
				node.settings = { ...restored.settings };
				if (restored.meshName) node.meshName = restored.meshName;
				if (restored.emboss) node.emboss = restored.emboss;
				if (restored.cutId) node.cutId = restored.cutId;
			}
			paintings[objectId] = restored.painting;
			layerProfiles[objectId] = restored.layerProfile;
			brimEarsByObject[objectId] = restored.brimEars;
			layerRangesByObject[objectId] = restored.layerRanges ?? [];
			volumesByObject[objectId] = restored.volumes ?? [];
			track(objectId, model.id);
		}
		// Modèles orphelins postérieurs à la dernière sauvegarde (ou scène vierge) :
		// nouveaux objets sur le plateau actif. Les suppressions sauvegardées restent
		// absentes (cf. `planSceneRestore`).
		for (const modelId of plan.orphanModelIds) {
			const model = modelById.get(modelId);
			if (!model) continue;
			if (!baselineReady) restoredOrphans = true;
			const objectId = tree.add(model.filename).id;
			if (plates.activeId) plates.assign(objectId, plates.activeId);
			track(objectId, model.id);
		}
		imports = restoredImports;
		// Sonde l'état de conversion **une fois par modèle** : maillage prêt → chargé
		// pour toutes ses instances ; en cours → reste en attente (événement
		// `model.converted`) ; échoué avant la session → badge d'erreur (422).
		for (const modelId of plan.modelIds) {
			await resolveConversion(modelId);
		}
	}

	// Charge les catalogues de presets (T098). Filament **et** process (traitement)
	// sont réduits aux presets **compatibles avec l'imprimante sélectionnée**
	// (FR-021, filtre côté serveur). On résout le nom de l'imprimante active sur le
	// catalogue machine **complet** (une buse sœur du modèle possédé n'est pas dans
	// la liste réduite aux imprimantes possédées) et **avant** de charger
	// filament/process — sinon la première charge partirait sans filtre.
	// Chargement en cours partagé par les simples consultations du catalogue
	// (`loadPresetsShared`) : à l'ouverture, l'éditeur et les trois effets de
	// sélection de preset lançaient chacun le même lot de requêtes (catalogue
	// imprimantes, presets…), allongeant `editorReady` sur machine chargée.
	let presetsInFlight: Promise<void> | null = null;
	function loadPresets(): Promise<void> {
		const pending = loadPresetsNow();
		presetsInFlight = pending;
		void pending.finally(() => {
			if (presetsInFlight === pending) presetsInFlight = null;
		});
		return pending;
	}
	function loadPresetsShared(): Promise<void> {
		return presetsInFlight ?? loadPresets();
	}
	// — Gestion des presets (Phase 18) : valeurs système de référence, renommer /
	//   supprimer un preset utilisateur, comparer, changement de preset avec des
	//   modifications non enregistrées (Orca Tab / DiffPresetDialog /
	//   UnsavedChangesDialog).
	type Kind = 'process' | 'filament' | 'machine';
	let systemValues = $state<Record<Kind, Record<string, unknown> | null>>({
		process: null,
		filament: null,
		machine: null
	});
	$effect(() => {
		const kind = settingsDialog;
		if (!kind) return;
		const preset = activePresetOf(kind);
		const parent =
			preset?.origin === 'user' && preset.inherits
				? presetsOfKind(kind).find((p) => p.name === preset.inherits && p.origin !== 'user')
				: undefined;
		if (!parent) {
			untrack(() => (systemValues[kind] = null));
			return;
		}
		void getResolvedPreset(parent.id)
			.then((r) => {
				if (activePresetOf(kind)?.id === preset?.id) systemValues[kind] = r.values;
			})
			.catch(() => {});
	});
	function replacePresetList(kind: Kind, list: PresetSummary[]) {
		presetListSeq++;
		if (kind === 'process') processPresets = list;
		else if (kind === 'filament') filamentPresets = list;
		else machinePresets = list;
	}
	async function renameActivePreset(kind: Kind, name: string): Promise<string | null> {
		const preset = activePresetOf(kind);
		if (!preset || preset.origin !== 'user') return 'seuls les presets utilisateur se renomment';
		if (presetsOfKind(kind).some((p) => p.id !== preset.id && p.name === name))
			return `un preset « ${name} » existe déjà`;
		try {
			await updatePreset(preset.id, { name });
			replacePresetList(
				kind,
				presetsOfKind(kind).map((p) => (p.id === preset.id ? { ...p, name } : p))
			);
			saveMessage = `Preset renommé en « ${name} ».`;
			return null;
		} catch (e) {
			return e instanceof ApiError ? e.message : 'renommage impossible';
		}
	}
	// Détacher du preset système (Orca Tab::detach_preset) : copie autonome
	// sélectionnée (preset système) ou preset utilisateur détaché sur place.
	async function detachActivePreset(kind: Kind, name: string | null): Promise<string | null> {
		const preset = activePresetOf(kind);
		if (!preset) return 'aucun preset sélectionné';
		if (name && presetsOfKind(kind).some((p) => p.name === name))
			return `un preset « ${name} » existe déjà`;
		try {
			const detached = await detachPreset(preset.id, name);
			const summary = {
				...preset,
				id: detached.id,
				name: detached.name,
				origin: 'user',
				inherits: undefined
			};
			const list = presetsOfKind(kind);
			if (preset.origin === 'user') {
				replacePresetList(
					kind,
					list.map((p) => (p.id === preset.id ? summary : p))
				);
				saveMessage = `« ${preset.name} » est maintenant autonome.`;
			} else {
				replacePresetList(kind, [...list, summary]);
				if (kind === 'process') activePresets.process = summary.id;
				else if (kind === 'filament')
					setFilamentSlot(clampFilamentSlot(activePresets, filamentSlot), summary.id);
				else activePresets.printer = summary.id;
				saveMessage = `Copie autonome « ${summary.name} » créée et sélectionnée.`;
			}
			return null;
		} catch (e) {
			return e instanceof ApiError ? e.message : 'détachement impossible';
		}
	}
	async function deleteActivePreset(kind: Kind) {
		const preset = activePresetOf(kind);
		if (!preset || preset.origin !== 'user') return;
		const list = presetsOfKind(kind);
		const fallback =
			list.find((p) => p.name === preset.inherits && p.id !== preset.id) ??
			list.find((p) => p.id !== preset.id && p.origin !== 'user') ??
			list.find((p) => p.id !== preset.id);
		// Orca bascule d'abord sur le parent puis supprime ; ici le projet
		// référence le preset (active_presets) : bascule + sauvegarde avant la
		// suppression, sinon le backend refuse (« preset référencé par un projet »).
		const select = (id: string) => {
			if (kind === 'process') activePresets.process = id;
			else if (kind === 'filament')
				setFilamentSlot(clampFilamentSlot(activePresets, filamentSlot), id);
			else activePresets.printer = id;
		};
		try {
			if (fallback) {
				select(fallback.id);
				const saved = await save();
				if (saved.status !== 'saved') {
					select(preset.id);
					saveMessage = `Suppression impossible : ${saved.status === 'error' ? saved.message : 'sauvegarde du projet impossible'}`;
					return;
				}
			}
			const result = await deletePreset(preset.id);
			replacePresetList(
				kind,
				list.filter((p) => p.id !== preset.id)
			);
			settingsDialog = null;
			saveMessage = `Preset « ${preset.name} » supprimé${fallback ? `, « ${fallback.name} » sélectionné` : ''}.`;
			// Autres projets qui l'utilisaient : basculés sur le parent par le serveur.
			presetDeleteReport = result?.projects.length
				? {
						title: `« ${preset.name} » supprimé : ${result.projects.length} autre(s) projet(s) passé(s) sur « ${result.parent.name} »`,
						projects: result.projects
					}
				: null;
		} catch (e) {
			const blocking = e instanceof ApiError && e.status === 409 ? blockingProjects(e.details) : [];
			if (blocking.length) {
				// Le projet ouvert reste sur le preset de repli (déjà sauvegardé).
				presetDeleteReport = {
					title: `« ${preset.name} » n'a pas de preset parent et reste utilisé par ${blocking.length} projet(s) : choisissez un autre preset dans ces projets avant de le supprimer`,
					projects: blocking
				};
			}
			saveMessage = e instanceof ApiError ? e.message : 'suppression impossible';
		}
	}
	let presetDeleteReport = $state<{
		title: string;
		projects: { id: string; name: string }[];
	} | null>(null);
	let compare = $state<{ kind: Kind; left: string | null; right: string | null } | null>(null);
	function openCompare(kind: Kind) {
		const preset = activePresetOf(kind);
		const list = presetsOfKind(kind);
		const parent = list.find((p) => p.name === preset?.inherits);
		const other = parent ?? list.find((p) => p.id !== preset?.id);
		compare = { kind, left: preset?.id ?? null, right: other?.id ?? preset?.id ?? null };
	}
	let pendingSwitch = $state<{
		kind: Kind;
		toName: string;
		changes: SettingDiff[];
		apply: () => void;
	} | null>(null);
	const pendingTransfer: Record<Kind, Record<string, unknown> | null> = {
		process: null,
		filament: null,
		machine: null
	};
	// Remonte les sélecteurs après « Garder » (le <select> afficherait sinon le
	// preset refusé).
	let presetSelectSeq = $state(0);
	// Changement de preset en attente du chargement de la base (exposé pour
	// les tests et l'accessibilité : aria-busy sur la barre supérieure).
	let presetSwitchPending = $state<Kind | null>(null);
	// Changements de preset demandés mais pas encore appliqués (attente de la
	// base `/resolved` du preset courant, cf. ci-dessous). Le sélecteur affiche
	// déjà le nouveau preset : le tranchage les attend avant de lire
	// `activePresets`, sinon il partait avec l'ancien filament (scène en ligne,
	// presets jugés enregistrés → G-code au filament_colour du preset quitté).
	// Une confirmation (UnsavedChangesDialog) ne bloque pas : la promesse se
	// résout dès l'ouverture du dialogue.
	let presetSwitches: Promise<unknown> = Promise.resolve();
	function requestPresetSwitch(kind: Kind, id: string | null, apply: () => void) {
		const run = requestPresetSwitchNow(kind, id, apply);
		presetSwitches = Promise.all([presetSwitches, run.catch(() => {})]);
		return run;
	}
	async function requestPresetSwitchNow(kind: Kind, id: string | null, apply: () => void) {
		const current = activePresetOf(kind);
		if (!id || id === current?.id) return;
		// Base `/resolved` du preset courant encore en chargement (ex. juste après
		// « Enregistrer » ou un renommage) : l'attendre, sinon le brouillon saisi
		// pendant ce temps serait jugé « non chargé » et perdu sans confirmation.
		presetSwitchPending = kind;
		try {
			await editLoads.pending(kind);
		} finally {
			presetSwitchPending = null;
		}
		if (activePresetOf(kind)?.id !== current?.id) return;
		const resolved = resolvedForKind(kind);
		const draft = valuesForKind(kind);
		if (editLoadedId[kind] !== current?.id || !hasPresetChanges(resolved, draft)) {
			apply();
			return;
		}
		pendingSwitch = {
			kind,
			toName: presetsOfKind(kind).find((p) => p.id === id)?.name ?? id,
			changes: diffPresets(kind, resolved, draft),
			apply
		};
	}

	// Modifications locales des listes (renommer / supprimer) : une liste
	// demandée avant elles est périmée à son arrivée et ne doit pas les écraser.
	let presetListSeq = 0;
	async function loadPresetsNow(): Promise<void> {
		const seqAtStart = presetListSeq;
		try {
			const [printers, catalog, machines] = await Promise.all([
				listPrinters(),
				getPrinterCatalog(),
				listPresets('machine')
			]);
			if (seqAtStart !== presetListSeq) return loadPresetsNow();
			ownedPrinters = printers;
			printerCatalog = catalog;
			// Restriction (Phase 14) : le sélecteur ne montre que les presets machine
			// des imprimantes **possédées** ; le catalogue alimente le menu de buse.
			const owned = new Set(printers.map((p) => p.machine_preset_id));
			machinePresets = machines.filter((m) => owned.has(m.id) || m.id === activePresets.printer);
			const printerName = machines.find((m) => m.id === activePresets.printer)?.name;
			const [filaments, processes] = await Promise.all([
				listPresets('filament', printerName ?? undefined),
				listPresets('process', printerName ?? undefined)
			]);
			if (seqAtStart !== presetListSeq) return loadPresetsNow();
			filamentPresets = filaments;
			processPresets = processes;
		} catch (e) {
			importError = e instanceof ApiError ? e.message : 'chargement des presets impossible';
		}
	}

	// Valeurs éditées du panneau de réglages correspondant au type d'un preset.
	function valuesForKind(kind: 'process' | 'filament' | 'machine'): Record<string, unknown> {
		if (kind === 'filament') return filamentValues;
		if (kind === 'machine') return machineValues;
		return processValues;
	}

	function resolvedForKind(kind: 'process' | 'filament' | 'machine'): Record<string, unknown> {
		if (kind === 'filament') return filamentResolvedValues;
		if (kind === 'machine') return machineResolvedValues;
		return processResolvedValues;
	}

	function setValuesForKind(
		kind: 'process' | 'filament' | 'machine',
		resolved: Record<string, unknown>
	) {
		// Clés absentes de la chaîne d'héritage : valeur par défaut d'Orca (base et
		// brouillon identiques → pas de « modifié », seules les saisies sont sauvées).
		const complete = Object.keys(resolved).length > 0 ? withOrcaDefaults(kind, resolved) : resolved;
		const baseline = clonePresetValues(complete);
		const draft = clonePresetValues(complete);
		if (kind === 'process') {
			processResolvedValues = baseline;
			processValues = draft;
		} else if (kind === 'filament') {
			filamentResolvedValues = baseline;
			filamentValues = draft;
		} else {
			machineResolvedValues = baseline;
			machineValues = draft;
		}
	}

	function selectPreset(kind: 'process' | 'filament' | 'machine', id: string) {
		if (kind === 'filament') {
			const slot = clampFilamentSlot(activePresets, filamentSlot);
			activePresets = setFilament(activePresets, id, slot);
			filamentSlot = clampFilamentSlot(activePresets, slot);
		} else if (kind === 'machine') activePresets.printer = id;
		else activePresets.process = id;
	}

	// Preset actif d'un type (pour le dialogue de réglages modal, T138).
	function activePresetOf(kind: 'process' | 'filament' | 'machine'): PresetSummary | undefined {
		if (kind === 'process') return processPresets.find((p) => p.id === activePresets.process);
		if (kind === 'filament') return filamentPresets.find((p) => p.id === filamentSel);
		return machinePresets.find((p) => p.id === activePresets.printer);
	}

	// Preset actif résolu pour l'enregistrement : recharge la liste si l'id actif
	// n'y figure pas encore.
	async function presetForSave(
		kind: 'process' | 'filament' | 'machine'
	): Promise<PresetSummary | undefined> {
		const id =
			kind === 'process'
				? activePresets.process
				: kind === 'filament'
					? filamentSel
					: activePresets.printer;
		if (!id) return undefined;
		let preset = activePresetOf(kind);
		if (!preset) {
			await loadPresetsShared();
			preset = activePresetOf(kind);
		}
		return preset?.id === id ? preset : undefined;
	}

	// Enregistre les réglages édités dans le dialogue courant. Parité OrcaSlicer :
	// éditer un preset **système** puis enregistrer **dérive automatiquement** une
	// copie utilisateur (héritée) et la sélectionne — pas de bouton « Dériver »
	// distinct ; un preset utilisateur est mis à jour en place.
	async function saveSettingsDialog() {
		if (!settingsDialog) return;
		const currentKind = settingsDialog;
		const kinds = ['process', 'filament', 'machine'] as const;
		// Base `/resolved` encore en chargement (machine lente) : l'attendre avant de
		// figer les brouillons, sinon la différence est calculée contre une base
		// vide (remplissages des widgets enregistrés, saisie mal isolée).
		await Promise.all(kinds.map((kind) => editLoads.pending(kind)));
		if (settingsDialog !== currentKind) return;
		// Brouillons figés au clic : un chargement `/resolved` qui se termine pendant
		// les appels réseau ci-dessous ne doit pas changer ce qui est enregistré.
		const drafts = Object.fromEntries(
			kinds.map((kind) => [
				kind,
				{
					values: clonePresetValues(valuesForKind(kind)),
					resolved: clonePresetValues(resolvedForKind(kind))
				}
			])
		) as Record<
			(typeof kinds)[number],
			{ values: Record<string, unknown>; resolved: Record<string, unknown> }
		>;
		// Les dépendances peuvent appartenir à un autre preset que celui testé
		// (ex. gcode_flavor machine pour une option process). Persister chaque
		// brouillon modifié, sans écraser les surcharges existantes.
		const toSave = kinds.filter(
			(kind) => kind === currentKind || hasPresetChanges(resolvedForKind(kind), valuesForKind(kind))
		);
		try {
			let derived = false;
			for (const kind of toSave) {
				// La liste des presets peut ne pas être encore chargée (ouverture du
				// dialogue avant la fin de `loadPresets`) : sans ce rechargement, le
				// preset courant était ignoré et « Réglages enregistrés » s'affichait
				// sans dérivation ni mise à jour.
				const preset = await presetForSave(kind);
				if (!preset) {
					if (kind === currentKind) throw new Error('preset actif introuvable');
					continue;
				}
				const values = drafts[kind].values;
				// Orca refuse l'extrusion absolue avec le `G92 E0` fourni par le
				// profil de base. Comme dans l'UI Orca, retirer cette instruction
				// incompatible au moment d'enregistrer le passage en absolu.
				if (
					kind === 'machine' &&
					values.use_relative_e_distances === false &&
					typeof values.before_layer_change_gcode === 'string'
				) {
					values.before_layer_change_gcode = values.before_layer_change_gcode
						.split('\n')
						.filter((line) => line.trim() !== 'G92 E0')
						.join('\n');
				}
				const raw = ((await getPreset(preset.id)).values ?? {}) as Record<string, unknown>;
				const changes = presetValuesForSave(
					drafts[kind].resolved,
					values,
					raw,
					preset.origin !== 'user'
				);
				if (preset.origin !== 'user') {
					const created = await createDerivedPreset(
						preset.kind,
						preset.name,
						changes,
						kind === currentKind ? saveAsName?.trim() || undefined : undefined
					);
					selectPreset(kind, created.id);
					derived = true;
				} else {
					await updatePreset(preset.id, { values: changes });
				}
			}
			// Une seule sauvegarde du projet après avoir sélectionné toutes les
			// copies dérivées (active_presets cohérent pour le prochain slice).
			if (derived) await save();
			saveMessage = 'Réglages enregistrés.';
			settingsDialog = null;
			await loadPresets();
			await Promise.all(toSave.map((kind) => loadEditedValues(kind, true)));
		} catch (e) {
			importError =
				e instanceof ApiError
					? e.message
					: `enregistrement impossible${e instanceof Error ? ` : ${e.message}` : ''}`;
		}
	}

	// Dernier chargement gagnant par portée ; un appel remplacé attend le plus
	// récent (sinon `editorReady` se résoudrait avant le preset machine).
	const editLoads = createLatestLoader<'process' | 'filament' | 'machine'>();
	// Preset dont la base `/resolved` alimente actuellement le brouillon.
	const editLoadedId: Record<'process' | 'filament' | 'machine', string | null> = {
		process: null,
		filament: null,
		machine: null
	};

	// Pré-remplit avec la configuration effective `/resolved`. Les surcharges
	// brutes ne sont relues qu'à l'enregistrement, pour ne pas aplatir l'héritage.
	// Un brouillon modifié du même preset n'est jamais écrasé, sauf `force`
	// (après enregistrement : la base a changé côté serveur).
	// Clés de réglage saisies par l'utilisateur dans les dialogues (séquence
	// croissante), par portée : distingue une saisie d'un remplissage de widget.
	let editTouchSeq = 0;
	const editTouched: Record<'process' | 'filament' | 'machine', Map<string, number>> = {
		process: new Map(),
		filament: new Map(),
		machine: new Map()
	};
	// Preset affiché au moment de chaque saisie (par portée et clé).
	const editTouchedPreset: Record<'process' | 'filament' | 'machine', Map<string, string>> = {
		process: new Map(),
		filament: new Map(),
		machine: new Map()
	};
	function recordSettingEdit(e: Event) {
		const kind = settingsDialog;
		const key = (e.target as Element | null)?.closest?.('[data-param]')?.getAttribute('data-param');
		if (kind && key) {
			editTouched[kind].set(key, ++editTouchSeq);
			const presetId = activePresetOf(kind)?.id;
			if (presetId) editTouchedPreset[kind].set(key, presetId);
			else editTouchedPreset[kind].delete(key);
		}
	}

	function loadEditedValues(kind: 'process' | 'filament' | 'machine', force = false) {
		return editLoads.run(kind, (isCurrent) => loadEditedValuesOnce(kind, force, isCurrent));
	}

	async function loadEditedValuesOnce(
		kind: 'process' | 'filament' | 'machine',
		force: boolean,
		isCurrent: () => boolean
	) {
		let preset = activePresetOf(kind);
		if (!preset) {
			await loadPresetsShared();
			if (!isCurrent()) return;
			preset = activePresetOf(kind);
		}
		if (!preset) {
			if (isCurrent()) {
				editLoadedId[kind] = null;
				setValuesForKind(kind, {});
			}
			return;
		}
		const requestedId = preset.id;
		// Saisies de l'utilisateur (événements des lignes de réglage, cf.
		// `recordSettingEdit`) postérieures au départ de la requête — ou toutes tant
		// qu'aucune base n'a été chargée (ouverture de l'éditeur, requête relancée
		// à l'arrivée du catalogue) : réappliquées sur la base reçue au lieu d'être
		// écrasées par une réponse lente. Les remplissages automatiques des widgets
		// ne sont pas des saisies et ne sont pas conservés.
		const touchSeqAtStart = editTouchSeq;
		try {
			const resolved = await getResolvedPreset(requestedId);
			if (!isCurrent() || activePresetOf(kind)?.id !== requestedId) return;
			if (
				!force &&
				editLoadedId[kind] === requestedId &&
				hasPresetChanges(resolvedForKind(kind), valuesForKind(kind))
			)
				return;
			const draft = valuesForKind(kind);
			// Aussi les saisies faites sur CE preset avant le départ de la requête
			// (une requête plus ancienne, annulée par celle-ci, n'a pas pu les
			// réappliquer : ex. rechargement forcé après « Enregistrer » doublé par
			// le rechargement dû au changement de preset).
			const typed = [...editTouched[kind]]
				.filter(
					([key, seq]) =>
						key in draft &&
						(seq > touchSeqAtStart ||
							editLoadedId[kind] === null ||
							editTouchedPreset[kind].get(key) === requestedId)
				)
				.map(([key]) => key);
			editTouched[kind].clear();
			editTouchedPreset[kind].clear();
			const kept = clonePresetValues(Object.fromEntries(typed.map((key) => [key, draft[key]])));
			editLoadedId[kind] = requestedId;
			setValuesForKind(kind, resolved.values);
			// « Transférer » (changement de preset avec modifications) : les
			// différences du brouillon précédent sont posées sur le nouveau preset.
			const transfer = pendingTransfer[kind];
			if (transfer) {
				pendingTransfer[kind] = null;
				// Brouillon seulement : la base reste le preset choisi (options
				// transférées affichées comme modifiées, à enregistrer ou annuler).
				const next = transferChanges(transfer, valuesForKind(kind));
				if (kind === 'process') processValues = next;
				else if (kind === 'filament') filamentValues = next;
				else machineValues = next;
			}
			if (typed.length) {
				const next = valuesForKind(kind);
				for (const key of typed) next[key] = kept[key];
			}
		} catch (e) {
			importError = e instanceof ApiError ? e.message : 'chargement des réglages impossible';
		}
	}

	// Ouvre immédiatement : le catalogue complet peut encore être en cours de
	// chargement au premier clic. La configuration effective continue d'arriver
	// via `/resolved`, avec les gardes de génération contre les réponses obsolètes.
	// Dérive une copie utilisateur : nom unique « X (copie) », « X (copie 2) »…
	// (comme Orca) ; un 409 (nom pris, liste locale incomplète) passe au suivant.
	async function createDerivedPreset(
		kind: PresetSummary['kind'],
		parentName: string,
		values: Record<string, unknown>,
		chosenName?: string
	) {
		// Nom choisi par l'utilisateur : une seule tentative, doublon signalé.
		if (chosenName) {
			try {
				return await createPreset({ kind, name: chosenName, inherits: parentName, values });
			} catch (e) {
				if (e instanceof ApiError && e.status === 409)
					throw new Error(`un preset « ${chosenName} » existe déjà`, { cause: e });
				throw e;
			}
		}
		const taken = [...processPresets, ...filamentPresets].map((p) => p.name);
		const first = uniqueCopyName(parentName, taken);
		const tried: string[] = [];
		for (let attempt = 0; ; attempt++) {
			const name = attempt === 0 ? first : uniqueCopyName(parentName, [...taken, first, ...tried]);
			try {
				// Orca : `inherits` = nom du parent (le backend résout aussi un id).
				return await createPreset({ kind, name, inherits: parentName, values });
			} catch (e) {
				if (!(e instanceof ApiError && e.status === 409) || attempt >= 50) throw e;
				tried.push(name);
			}
		}
	}

	// Onglet ouvert par le dialogue (raccourci « Tour d'essuyage » → Multimatériau).
	let settingsInitialPage = $state<string | undefined>(undefined);
	// Option mise en évidence à l'ouverture (lien « Régler » de la validation).
	let settingsFocusKey = $state<string | null>(null);
	function openSettingsDialog(
		kind: 'process' | 'filament' | 'machine',
		page?: string,
		focusKey: string | null = null
	) {
		settingsInitialPage = page;
		settingsFocusKey = focusKey;
		settingsDialog = kind;
		// Base déjà chargée pour ce preset : pas de rechargement (il écraserait
		// une saisie en cours si la réponse arrivait après).
		const current = activePresetOf(kind)?.id;
		if (current && editLoadedId[kind] === current) return;
		void loadEditedValues(kind);
	}

	// Chaque changement de sélection recharge la base résolue et efface le marqueur
	// « modifié » du preset précédent.
	$effect(() => {
		if (activePresets.process) void loadEditedValues('process');
	});
	$effect(() => {
		if (filamentSel) void loadEditedValues('filament');
	});
	$effect(() => {
		if (activePresets.printer) void loadEditedValues('machine');
	});

	// Ajoute les imprimantes sélectionnées dans la grille (une par modèle, buse par
	// défaut, sans connexion réseau — configurable ensuite sur la page Imprimantes).
	async function addPrintersFromCatalog(models: PrinterCatalogModel[]) {
		addingPrinter = true;
		try {
			let firstPreset: string | null = null;
			for (const m of models) {
				await createPrinter({ name: m.model, machine_preset_id: m.default_machine_preset_id });
				firstPreset ??= m.default_machine_preset_id;
			}
			await loadPresets();
			// Sélectionne d'emblée la première imprimante ajoutée.
			if (firstPreset && !activePresets.printer) activePresets.printer = firstPreset;
			showCatalog = false;
		} catch (e) {
			importError = e instanceof ApiError ? e.message : 'ajout d’imprimante impossible';
		} finally {
			addingPrinter = false;
		}
	}

	// G-code dont l'aperçu est déjà chargé : évite de recharger à chaque événement.
	let previewLoadedFor: string | null = null;

	/** Relit l'état des jobs de la session en cours et l'applique comme des
	 *  événements (reprise après rechargement, retour de connexion). Les lots
	 *  « Trancher tout » gardent leur propre suivi. */
	async function resyncSessionJobs() {
		if (session.phase !== 'slicing' || sliceBatch) return;
		for (const id of [...session.jobIds]) {
			try {
				const job = await getJob(id);
				if (session.phase !== 'slicing' || !session.jobIds.includes(id)) return;
				onEvent(jobAsEvent(job));
			} catch {
				// hors ligne : nouvel essai au retour de la connexion
			}
		}
	}

	function onEvent(event: ServerEvent) {
		const wasPreview = layout.tab === 'preview';
		if (
			sliceBatch &&
			(event.event === 'job.updated' || event.event === 'job.finished') &&
			sliceBatch.jobPlates[event.id]
		) {
			// Lot par plateau : la session suit le plateau actif, les autres
			// résultats sont mémorisés (plate-results.ts).
			const before = session.gcodeId;
			const step = applyBatchEvent(
				{ session, batch: sliceBatch, results: plateResults },
				plates.activeId,
				event
			);
			session = step.session;
			sliceBatch = step.batch;
			plateResults = step.results;
			if (step.showPlate && plates.get(step.showPlate)) {
				plates.activeId = step.showPlate;
				ws = setSelection(ws, new Set());
			}
			if (session.gcodeId && session.gcodeId !== before) {
				const shown = Object.values(plateResults).find((r) => r.gcodeId === session.gcodeId);
				rawStats = shown?.stats ?? null;
			}
			const label = batchLabel(sliceBatch);
			if (label) saveMessage = `Trancher tout : ${label}${batchFinished(sliceBatch) ? '.' : '…'}`;
		} else {
			session = applyJobEvent(session, event);
			if (event.event === 'job.finished' && session.jobIds.includes(event.id)) {
				rawStats = event.stats;
			}
		}
		// Au premier G-code de la session : charge l'aperçu et bascule sur l'onglet
		// (sauf si l'utilisateur l'a déjà quitté). Le chargement doit se faire même
		// quand `sliceActive` a déjà ouvert l'onglet aperçu (d'où le suivi par
		// `previewLoadedFor` plutôt que la seule garde `!wasPreview`, qui empêchait
		// la préviz de se charger).
		if (session.phase === 'preview' && session.gcodeId && session.gcodeId !== previewLoadedFor) {
			previewLoadedFor = session.gcodeId;
			if (!wasPreview) layout = setTab(layout, 'preview');
			void loadPreview(session.gcodeId);
		}
		// Fin de conversion moteur (STEP…) : récupère le maillage et l'affiche.
		if (event.event === 'model.converted') {
			void resolveConversion(event.model_id);
		}
		// Échec de conversion (lot 27) : `/mesh` répond 422 avec la cause, traitée
		// comme à l'ouverture (badge + cause dans la liste, notification).
		if (event.event === 'model.conversion_failed') {
			void resolveConversion(event.model_id);
		}
	}

	function patchImport(objectId: string, fn: (i: ImportItem) => ImportItem) {
		imports = imports.map((i) => (i.objectId === objectId ? fn(i) : i));
	}

	// — OBJ coloré (Orca : ObjColorDialog, lot 16) : après l'upload d'un OBJ, le
	// serveur détecte ses couleurs ; si l'OBJ en a, le dialogue propose de les
	// répartir sur des filaments (nouveaux ou existants) et la peinture MMU
	// renvoyée est renumérotée vers ces filaments. Un dialogue à la fois.
	type ObjColorRequest = { objectId: string; filename: string; res: ObjColorsResponse };
	const objMtlFiles = new WeakMap<File, File>();
	let objColorPending = $state<ObjColorRequest | null>(null);
	const objColorQueue: ObjColorRequest[] = [];
	async function detectObjColors(
		objectId: string,
		modelId: string,
		file: File,
		generation: number
	) {
		let res: ObjColorsResponse;
		try {
			res = await objColors(modelId, objMtlFiles.get(file) ?? null);
		} catch {
			return; // détection indisponible : l'objet reste sur un seul filament
		}
		if (generation !== sceneGeneration || !tree.get(objectId)) return;
		if (!res.colored || !res.palette?.length) return;
		const request = { objectId, filename: file.name, res };
		if (objColorPending) objColorQueue.push(request);
		else objColorPending = request;
	}
	// — Conversion d'unités (Orca : « Object too small », Plater.cpp:6693) : un
	// objet de moins de 8 mm³ (0,008 en mètres) propose ×25,4 ou ×1000.
	type UnitRequest = {
		objectId: string;
		filename: string;
		size: [number, number, number];
		suggested: UnitSource;
	};
	let unitPending = $state<UnitRequest | null>(null);
	async function offerUnitConversion(objectId: string, filename: string, generation: number) {
		if (!currentPreferences().unitConversionPrompt) return;
		for (let i = 0; i < 360; i++) {
			if (generation !== sceneGeneration || !tree.get(objectId)) return;
			const item = imports.find((x) => x.objectId === objectId);
			if (!item || item.status === 'failed') return;
			const obj = sceneObjects.find((o) => o.id === objectId);
			if (item.status === 'ready' && obj) {
				const unit = detectUnits(scaledVolume(meshVolume(obj.mesh), obj.scale));
				if (!unit || unitPending) return;
				const [sx, sy, sz] = worldSize(obj.mesh, obj.rotation, obj.scale);
				unitPending = { objectId, filename, size: [sx, sy, sz], suggested: unit };
				return;
			}
			await new Promise((r) => setTimeout(r, 500));
		}
	}
	function convertObjectUnits(id: string, unit: UnitSource, restore: boolean) {
		const obj = sceneObjects.find((o) => o.id === id);
		if (!obj) return;
		const scale = convertScale(obj.scale, unit, restore);
		const [x, y] = obj.position ?? [bed.center.x, bed.center.y, 0];
		onTransform(id, {
			...objectTransform(obj),
			position: [x, y, restingZ(obj.mesh, obj.rotation, scale)],
			scale
		});
	}

	// — Modèles pratiques (Orca « Add Handy models ») : catalogue du backend,
	// un objet par fichier, arrangement si demandé, suggestion String Hell.
	let handyModels = $state<HandyModelInfo[]>([]);
	let handySuggestion = $state<HandyModelSuggestion | null>(null);
	onMount(() => {
		listHandyModels()
			.then((list) => (handyModels = list))
			.catch(() => {});
	});
	async function importsReady(ids: string[], generation: number): Promise<boolean> {
		for (let i = 0; i < 360; i++) {
			if (generation !== sceneGeneration) return false;
			const items = ids.map((id) => imports.find((x) => x.objectId === id));
			if (items.some((x) => !x || x.status === 'failed')) return false;
			if (
				items.every((x) => x?.status === 'ready') &&
				ids.every((id) => sceneObjects.some((o) => o.id === id))
			)
				return true;
			await new Promise((r) => setTimeout(r, 500));
		}
		return false;
	}
	async function addHandyModelToScene(key: string) {
		await editorReady;
		const generation = sceneGeneration;
		importError = null;
		let res: HandyModelAdded;
		try {
			res = await addHandyModel(data.project.id, key);
		} catch (e) {
			importError = e instanceof ApiError ? e.message : 'ajout du modèle pratique impossible';
			return;
		}
		if (generation !== sceneGeneration) return;
		const before = activeSceneObjects.length;
		const ids: string[] = [];
		for (const model of res.models) {
			const objectId = tree.add(model.filename).id;
			ids.push(objectId);
			rollbackImportIds.add(objectId);
			if (plates.activeId) plates.assign(objectId, plates.activeId);
			imports = [...imports, startImport(objectId, model.filename)];
			projectModelIds.add(model.id);
			patchImport(objectId, (i) => markUploaded(i, model.id, model.conversion_pending));
			if (!model.conversion_pending) await resolveConversion(model.id);
			else void watchConversion(model.id, generation);
		}
		ws = setSelection(ws, new Set(ids));
		// Orca : arrangement après l'ajout pour les combos ; ici aussi dès que le
		// plateau contenait déjà des objets (préférence « Arranger automatiquement »).
		const arrange =
			res.handy_model.arrange_after_import || (currentPreferences().autoArrange && before > 0);
		if (arrange && (await importsReady(ids, generation))) await arrangeActive();
		const suggestion = res.handy_model.suggestion;
		if (suggestion && suggestionApplies(suggestion, processValues)) handySuggestion = suggestion;
	}

	// — Purge automatique (Orca auto_calculate_flush) : après un changement de
	// couleur (ou de filament), lignes et colonnes du slot recalculées.
	let flushSnapshot: FlushSnapshot | null = null;
	$effect(() => {
		const next: FlushSnapshot = {
			colours: [...filamentSlotColors],
			filaments: [...activePresets.filaments]
		};
		const mode = $appPreferences.autoFlush;
		if (!baselineReady) {
			flushSnapshot = next;
			return;
		}
		const slots = changedSlots(flushSnapshot, next, mode);
		flushSnapshot = next;
		if (slots.length) void autoRecalcFlush(slots, next.colours.length);
	});
	async function autoRecalcFlush(slots: number[], n: number) {
		// État au déclenchement : une saisie (dialogue appliqué) reçue avant la
		// réponse l'emporte ; le multiplicateur n'est jamais modifié ici.
		const started = {
			matrix: projectFilament.flush_volumes_matrix,
			multiplier: projectFilament.flush_multiplier
		};
		if (!(await flushVolumesAvailable().catch(() => false))) return;
		try {
			// Matrice brute (sans multiplicateur, appliqué au tranchage), comme Orca.
			const { matrix } = await computeFlushVolumes([...filamentSlotColors], 1, data.project.id);
			if (filamentSlotColors.length !== n) return;
			const merged = autoFlushResult(
				started,
				{
					matrix: projectFilament.flush_volumes_matrix,
					multiplier: projectFilament.flush_multiplier
				},
				matrix,
				n,
				slots
			);
			if (!merged) return;
			projectFilament = { ...projectFilament, flush_volumes_matrix: merged };
			saveMessage = `Volumes de purge recalculés (filament ${slots.map((s) => s + 1).join(', ')}).`;
		} catch {
			// Calcul indisponible : les volumes saisis restent en place.
		}
	}

	// — Maillage STEP (Orca : StepMeshDialog, lot 17) : proposé une fois le STEP
	// converti ; chaque remaillage remplace le maillage affiché des instances.
	type StepMeshRequest = {
		objectId: string;
		modelId: string;
		filename: string;
		triangles: number | null;
	};
	let stepMeshPending = $state<StepMeshRequest | null>(null);
	async function offerStepMesh(
		objectId: string,
		modelId: string,
		filename: string,
		generation: number
	) {
		for (let i = 0; i < 360; i++) {
			if (generation !== sceneGeneration || !tree.get(objectId)) return;
			const item = imports.find((x) => x.objectId === objectId);
			if (!item || item.status === 'failed') return;
			const obj = sceneObjects.find((o) => o.id === objectId);
			if (item.status === 'ready' && obj) {
				// « Ne plus afficher » (Orca enable_step_mesh_setting) : les réglages
				// mémorisés s'appliquent sans dialogue s'ils diffèrent des défauts.
				const prefs = loadStepPrefs();
				if (!prefs.ask) {
					if (!sameStepOptions(prefs.options, STEP_DEFAULTS)) {
						await remeshStep(modelId, prefs.options).catch(() => null);
						await reloadStepMesh(modelId).catch(() => {});
					}
					return;
				}
				if (stepMeshPending) return; // un dialogue à la fois
				stepMeshPending = { objectId, modelId, filename, triangles: obj.mesh.indices.length / 3 };
				return;
			}
			await new Promise((r) => setTimeout(r, 500));
		}
	}
	async function reloadStepMesh(modelId: string) {
		meshRequests.delete(modelId);
		const { mesh } = await loadCenteredMesh(modelId);
		const ids = new Set(imports.filter((i) => i.modelId === modelId).map((i) => i.objectId));
		// Nouveau maillage : la peinture par triangle de l'ancien n'a plus de sens.
		for (const id of ids) delete paintings[id];
		paintings = { ...paintings };
		sceneObjects = sceneObjects.map((o) => (ids.has(o.id) ? { ...o, mesh } : o));
	}
	function nextObjColor() {
		objColorPending = objColorQueue.shift() ?? null;
	}
	function applyObjColors(plan: ObjColorPlan) {
		const pending = objColorPending;
		if (!pending) return;
		const { objectId, res } = pending;
		const mesh = sceneObjects.find((o) => o.id === objectId)?.mesh;
		const meshTriangles = mesh ? mesh.indices.length / 3 : null;
		if (
			meshTriangles !== null &&
			res.triangles &&
			![meshTriangles, meshTriangles * 3].includes(res.triangles)
		) {
			saveMessage = `Couleurs de « ${pending.filename} » non appliquées : maillage différent (${res.triangles} / ${meshTriangles} triangles).`;
			nextObjColor();
			return;
		}
		let added = 0;
		for (const colour of plan.newColours) {
			const before = activePresets.filaments.length;
			addFilament();
			if (activePresets.filaments.length === before) break;
			projectFilament = setSlotColour(projectFilament, activePresets.filaments.length - 1, colour);
			added++;
		}
		const doc = paintingFor(objectId);
		paintUndo.push({ id: objectId, channel: 'mmu', facets: doc.serialize().mmu ?? {} });
		paintRedo = [];
		doc.replaceChannel('mmu', remapMmu(res.painting?.mmu ?? {}, plan));
		paintings = { ...paintings };
		tree.setExtruder(objectId, mapExtruder(plan, res.extruder || 1));
		saveMessage = `Couleurs de « ${pending.filename} » appliquées${added ? ` : ${added} filament${added > 1 ? 's' : ''} ajouté${added > 1 ? 's' : ''}` : ''}.`;
		nextObjColor();
	}

	// Importe une liste de fichiers : aperçu immédiat + upload en tâche de fond.
	async function importFiles(files: File[]) {
		importError = null;
		let firstLoaded: string | null = null;
		// OBJ coloré : un .mtl déposé avec l'OBJ l'accompagne (couleurs Kd).
		const mtls = files.filter((f) => isMtl(f.name));
		const models = files.filter((f) => !isMtl(f.name));
		if (mtls.length && !models.length) importError = 'Un fichier .mtl s’importe avec son .obj.';
		queuedImports = [
			...queuedImports,
			...models.filter((f) => isAccepted(f.name)).map((f) => f.name)
		];
		for (const file of models) {
			if (/\.obj$/i.test(file.name) && mtls.length) {
				const text = await file
					.slice(0, 64 * 1024)
					.text()
					.catch(() => null);
				const name = pickMtl(
					file.name,
					text,
					mtls.map((m) => m.name)
				);
				const mtl = mtls.find((m) => m.name === name);
				if (mtl) objMtlFiles.set(file, mtl);
			}
			if (!isAccepted(file.name)) {
				importError = `Format non supporté : ${file.name}`;
				continue;
			}
			if ((await importOne(file)) && firstLoaded === null) firstLoaded = file.name;
		}
		// Orca : un projet encore « Untitled » prend le nom du premier fichier chargé.
		if (firstLoaded && isDefaultProjectName(savedName || projectName)) {
			const name = projectNameFromFile(firstLoaded);
			if (name) {
				projectName = name;
				await renameCurrentProject(firstLoaded);
			}
		}
	}

	/** Importe un fichier ; `true` si l'upload a abouti (modèle chargé côté serveur). */
	async function importOne(file: File): Promise<boolean> {
		try {
			await editorReady;
		} finally {
			const at = queuedImports.indexOf(file.name);
			if (at >= 0) queuedImports = queuedImports.filter((_, i) => i !== at);
		}
		const generation = sceneGeneration;
		const node = tree.add(file.name);
		const objectId = node.id;
		rollbackImportIds.add(objectId);
		if (plates.activeId) plates.assign(objectId, plates.activeId);
		imports = [...imports, startImport(objectId, file.name)];

		// Upload → décodage moteur (R7/T126) : plus d'aperçu client. Le maillage
		// arrive via `model.converted` → `resolveConversion` (fetchMesh) ; s'il est
		// déjà prêt à l'upload, on le charge immédiatement.
		try {
			if (/\.3mf$/i.test(file.name)) {
				try {
					return await importThreeMf(file, objectId, generation);
				} catch (e) {
					// Backend sans la route (déploiement antérieur) : import monobloc.
					if (!(e instanceof ApiError && (e.status === 404 || e.status === 405))) throw e;
				}
			}
			const model = await uploadModel(data.project.id, file);
			if (generation !== sceneGeneration) return false;
			projectModelIds.add(model.id);
			patchImport(objectId, (i) => markUploaded(i, model.id, model.conversion_pending));
			if (!model.conversion_pending) {
				await resolveConversion(model.id);
				// Fichier illisible : objet retiré, et le projet ne prend pas son nom.
				if (!sceneObjects.some((o) => o.id === objectId)) return false;
			} else void watchConversion(model.id, generation);
			if (/\.obj$/i.test(file.name)) void detectObjColors(objectId, model.id, file, generation);
			if (isStep(file.name)) void offerStepMesh(objectId, model.id, file.name, generation);
			if (!/\.3mf$/i.test(file.name)) void offerUnitConversion(objectId, file.name, generation);
			return true;
		} catch (e) {
			if (generation !== sceneGeneration) return false;
			failImport(objectId, e instanceof ApiError ? e.message : 'échec de l’upload');
			return false;
		}
	}

	/**
	 * 3MF : un objet de scène par objet du 3MF, comme Orca (nom, transformation,
	 * volumes, réglages par objet, peinture). Un 3MF à objet unique garde le
	 * comportement d'un modèle importé (nom du fichier, posé au centre).
	 */
	async function importThreeMf(file: File, objectId: string, generation: number) {
		const res = await import3mfObjects(data.project.id, file);
		if (generation !== sceneGeneration) return false;
		for (const model of res.models) projectModelIds.add(model.id);
		const modelById = new Map(res.models.map((m) => [m.id, m]));
		const watch = (model: { id: string; conversion_pending: boolean }) =>
			model.conversion_pending
				? void watchConversion(model.id, generation)
				: resolveConversion(model.id);
		if (res.objects.length === 1) {
			const model = modelById.get(res.objects[0].modelId)!;
			patchImport(objectId, (i) => markUploaded(i, model.id, model.conversion_pending));
			await watch(model);
			return true;
		}
		// Plusieurs objets : le nœud provisoire est remplacé par un nœud par objet.
		const plateId = plates.activeId;
		plates.unassign(objectId);
		tree.remove(objectId);
		imports = imports.filter((i) => i.objectId !== objectId);
		rollbackImportIds.delete(objectId);
		const added: ImportItem[] = [];
		for (const doc of res.objects) {
			const model = modelById.get(doc.modelId);
			if (!model) continue;
			const id = tree.add(doc.name || model.filename).id;
			if (plateId) plates.assign(id, plateId);
			const restored = deserializeObject(doc);
			savedTransformByObject[id] = restored.transform;
			savedDocumentByObject[id] = doc;
			const node = tree.get(id);
			if (node) {
				node.extruder = restored.extruder;
				node.printable = restored.printable;
				node.settings = { ...restored.settings };
				if (restored.meshName) node.meshName = restored.meshName;
				if (restored.emboss) node.emboss = restored.emboss;
				if (restored.cutId) node.cutId = restored.cutId;
			}
			paintings[id] = restored.painting;
			layerProfiles[id] = restored.layerProfile;
			brimEarsByObject[id] = restored.brimEars;
			layerRangesByObject[id] = restored.layerRanges ?? [];
			volumesByObject[id] = restored.volumes ?? [];
			added.push(markUploaded(startImport(id, model.filename), model.id, model.conversion_pending));
		}
		imports = [...imports, ...added];
		for (const model of res.models) await watch(model);
		return true;
	}

	// Récupère le maillage moteur d'un modèle et l'ajoute à la scène. Distingue
	// les états servis par `/mesh` (T125) : 200 → prêt ; 409 → conversion encore
	// en cours (on laisse l'objet en attente, `model.converted` le résoudra) ;
	// 422 (ou autre) → échec → badge d'erreur. Sert au rendu à l'ouverture (sonde
	// l'état réel des modèles, dont ceux échoués avant la session) comme à la
	// résolution en direct sur l'événement.
	// Un seul fetch + décodage + recentrage par modèle, partagé par toutes ses
	// instances (et par les appels concurrents : ouverture + `model.converted`).
	function loadCenteredMesh(modelId: string) {
		let request = meshRequests.get(modelId);
		if (!request) {
			request = fetchMesh(modelId).then((raw) => centerMesh(raw));
			meshRequests.set(modelId, request);
			// Un échec (409 en cours de conversion…) ne doit pas rester en cache.
			request.catch(() => {
				if (meshRequests.get(modelId) === request) meshRequests.delete(modelId);
			});
		}
		return request;
	}

	/**
	 * Conversion en attente après l'upload : `model.converted` peut arriver
	 * avant la réponse d'upload (l'import n'a pas encore son `modelId`, l'événement
	 * est ignoré) ou être perdu (flux WS reconnecté) — l'objet restait alors
	 * « conversion en cours » indéfiniment. On sonde `/mesh` (409 = en cours)
	 * avec un intervalle croissant tant que l'import est en attente.
	 */
	async function watchConversion(modelId: string, generation: number) {
		for (const delay of [0, 1000, 2000, 4000, 8000, 15000, 30000, 60000]) {
			if (delay) await new Promise((r) => setTimeout(r, delay));
			if (generation !== sceneGeneration) return;
			const pending = imports.some((i) => i.modelId === modelId && isPending(i));
			if (!pending) return;
			await resolveConversion(modelId);
		}
	}

	async function resolveConversion(modelId: string) {
		const generation = sceneGeneration;
		if (!imports.some((i) => i.modelId === modelId)) return;
		try {
			const { mesh, center } = await loadCenteredMesh(modelId);
			if (generation !== sceneGeneration) return;
			// Instances encore présentes après la requête réseau (suppressions,
			// duplications ou collages pendant l'attente pris en compte).
			const items = imports.filter((i) => i.modelId === modelId);
			const added: SceneObject[] = [];
			for (const item of items) {
				if (sceneObjects.some((object) => object.id === item.objectId)) continue;
				// Transformation sauvegardée prioritaire sur le recentrage par défaut.
				const t = savedTransformByObject[item.objectId];
				let position = positionForImport(center, [bed.center.x, bed.center.y], t?.position);
				if (!t?.position) {
					// Phase 28 : posé à côté des objets du plateau (Orca, cellule libre la
					// plus proche du centre) au lieu d'être empilé au centre.
					const taken = [...activeSceneObjects, ...added].map((o) =>
						objectFootprint(o, [bed.center.x, bed.center.y])
					);
					const fp = objectFootprint({ id: item.objectId, mesh }, [0, 0]);
					const [x, y] = nearestFreeCell(fp, taken, {
						x: bed.origin.x,
						y: bed.origin.y,
						width: bed.width,
						depth: bed.depth
					});
					position = [x, y, position[2]];
				}
				added.push({
					id: item.objectId,
					mesh,
					position,
					rotation: t?.rotation,
					scale: t?.scale
				});
			}
			if (added.length > 0) {
				// Objets restaurés : ordre du document conservé (ordre d'impression
				// séquentielle) même si les maillages arrivent dans le désordre.
				const order = tree.list().map((n) => n.id);
				sceneObjects = mergeInRank(sceneObjects, added, (o) =>
					savedDocumentByObject[o.id] ? order.indexOf(o.id) : undefined
				);
			}
			for (const item of items) {
				patchImport(item.objectId, markConverted);
				rollbackImportIds.delete(item.objectId);
			}
		} catch (e) {
			if (generation !== sceneGeneration) return;
			// 409 = pas encore prêt : rester en « conversion en cours ».
			if (e instanceof ApiError && e.status === 409) return;
			// Cause du moteur (lot 27 : details.cause, traduite par le catalogue d'Orca).
			const failed = e instanceof ApiError && e.code === 'conversion_failed';
			if (failed) orcaCatalog ??= await loadOrcaCatalog().catch(() => null);
			if (generation !== sceneGeneration) return;
			const cause = failed
				? conversionCause(
						(e.details as { cause?: string } | undefined)?.cause ??
							e.technical.replace(/^.*?moteur\s*:\s*/, ''),
						orcaCatalog
					)
				: null;
			const message = cause
				? 'conversion impossible — ' + cause
				: e instanceof ApiError
					? e.message
					: 'maillage converti indisponible';
			const kept: string[] = [];
			for (const item of imports.filter((i) => i.modelId === modelId)) {
				if (item.status === 'failed') continue;
				if (rollbackImportIds.has(item.objectId)) failImport(item.objectId, message);
				else {
					patchImport(item.objectId, (i) => markFailed(i, cause ?? message));
					kept.push(item.filename);
				}
			}
			// Objets restaurés (projet rouvert) : restent listés avec leur cause ;
			// notification comme pour un import raté.
			if (kept.length)
				importError =
					'Conversion impossible de ' +
					kept.map((f) => '« ' + f + ' »').join(', ') +
					' : ' +
					(cause ?? message);
		}
	}

	function failImport(objectId: string, message: string) {
		// Message préfixé du fichier concerné (« casse.stl : fichier illisible… »).
		const file = imports.find((i) => i.objectId === objectId)?.filename;
		importError = file ? `${file} : ${message}` : message;
		rollbackImportIds.delete(objectId);
		removeObject(objectId);
	}

	function onFilePicked(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		void importFiles([...(input.files ?? [])]);
		input.value = '';
	}

	function onDrop(e: DragEvent) {
		e.preventDefault();
		dragOver = false;
		void importFiles([...(e.dataTransfer?.files ?? [])]);
	}

	async function dismissDraft() {
		await draftStore.clear(data.project.id);
		pendingDraft = null;
	}

	function selectObject(id: string, additive: boolean) {
		if (!activePlate?.objectIds.includes(id)) return;
		ws = pick(ws, id, additive);
	}

	function selectPlate(id: string) {
		if (!plates.get(id)) return;
		plates.activeId = id;
		ws = setSelection(ws, new Set());
		closeContext();
	}

	// Changement d'onglet de plateau (clic, ajout, duplication, lot « Trancher
	// tout ») : l'aperçu suit le résultat mémorisé du plateau.
	let shownPlateId: string | null = null;
	$effect(() => {
		const id = plates.activeId;
		untrack(() => {
			const previous = shownPlateId;
			shownPlateId = id;
			if (previous === null || id === null || previous === id) return;
			rememberShownResult(previous);
			showPlateResult(id);
		});
	});

	// — Résultat par plateau (phase 25, Orca PartPlate) ————————————————————
	/** Empreinte de ce qui est tranché : scène (hors onglet actif), presets, brouillon. */
	function sliceSignature(): string {
		const scene = buildSceneForSave() as unknown as Record<string, unknown>;
		const saved = scene.plates;
		const stable =
			saved && typeof saved === 'object' && !Array.isArray(saved)
				? { ...scene, plates: { ...(saved as Record<string, unknown>), activeId: null } }
				: scene;
		return (
			JSON.stringify(stable) +
			JSON.stringify(serializeActivePresets(activePresets)) +
			JSON.stringify(currentDraft() ?? null)
		);
	}
	/** Résultat affiché (restauré à l'ouverture) rattaché à son plateau avant de changer d'onglet. */
	function rememberShownResult(previous: string) {
		if (session.phase !== 'preview' || !session.gcodeId) return;
		if (Object.values(plateResults).some((r) => r.gcodeId === session.gcodeId)) return;
		const owner = restoredPlateIndex !== null ? plates.list()[restoredPlateIndex]?.id : previous;
		if (!owner) return;
		plateResults = {
			...plateResults,
			[owner]: { gcodeId: session.gcodeId, jobId: '', stats: rawStats, signature: null }
		};
	}
	/** Affiche le résultat mémorisé du plateau (ou « non tranché »). */
	function showPlateResult(id: string) {
		if (session.phase === 'slicing') return; // le lot en cours décide
		const result = resultFor(plateResults, id, sliceSignature());
		if (!result && plateResults[id]) {
			// Résultat périmé (scène ou réglages modifiés depuis) : oublié.
			const rest = { ...plateResults };
			delete rest[id];
			plateResults = rest;
		}
		if (result) {
			if (session.gcodeId === result.gcodeId && session.phase === 'preview') return;
			session = { ...prepareSession(), phase: 'preview', progress: 1, gcodeId: result.gcodeId };
			rawStats = result.stats;
			previewLoadedFor = result.gcodeId;
			void loadPreview(result.gcodeId);
		} else if (session.gcodeId || session.phase === 'error') {
			// Le résultat affiché appartient à un autre plateau : celui-ci n'est pas tranché.
			session = prepareSession();
			rawStats = null;
			previewLoadedFor = null;
		}
	}

	// Clic dans la vue 3D sur un autre plateau (surface, étiquette ou objet).
	function activatePlateFromScene(plateId: string, objectId: string | null) {
		selectPlate(plateId);
		if (objectId) selectObject(objectId, false);
	}
	// Fin d'un glisser de gizmo hors du plateau actif : l'objet rejoint le
	// plateau qu'il touche (PartPlateList::notify_instance_update), position
	// ramenée dans le repère de ce plateau ; le plateau actif ne change pas.
	function dropObjectOnPlate(objectId: string, footprint: Footprint) {
		const list = plates.list();
		const activeIndex = plates.plateIndex(plates.activeId ?? '');
		if (activeIndex < 0 || !tree.get(objectId)) return;
		const index = dropPlateIndex(footprint, bed, list.length, activeIndex);
		if (index === null) return;
		const [dx, dy] = plateOffsets(list.length, activeIndex, bed)[index];
		sceneObjects = sceneObjects.map((o) => {
			if (o.id !== objectId) return o;
			const [x, y, z] = o.position ?? [0, 0, 0];
			return { ...o, position: [x - dx, y - dy, z] as [number, number, number] };
		});
		assignObjectToPlate(objectId, list[index].id);
		saveMessage = `« ${tree.get(objectId)?.name ?? 'Objet'} » déplacé sur ${list[index].name}.`;
	}

	function assignObjectToPlate(id: string, plateId: string) {
		if (!plates.get(plateId) || !tree.get(id)) return;
		plates.assign(id, plateId);
		if (plateId !== plates.activeId && ws.selection.has(id)) {
			const selection = new SvelteSet(ws.selection);
			selection.delete(id);
			ws = setSelection(ws, selection);
		}
		closeContext();
	}

	// Menu contextuel du plateau : chaque entrée active d'abord le plateau visé,
	// puis réutilise l'action existante (sélection, suppression, arrangement…).
	async function onPlateMenuAction(action: string) {
		const id = plateMenu.plateId;
		if (!id || !plates.get(id)) return;
		if (action === 'plate.delete') {
			removePlate(id);
			return;
		}
		if (plates.activeId !== id) selectPlate(id);
		switch (action) {
			case 'plate.selectAll':
				selectAllObjects();
				break;
			case 'plate.selectAllPlates':
				ws = setSelection(ws, new Set(sceneObjects.map((object) => object.id)));
				break;
			case 'plate.addModels':
				fileInput?.click();
				break;
			case 'plate.deleteAll':
				deleteAllObjects('plate');
				break;
			case 'plate.arrange':
				await arrangeActive();
				break;
			case 'plate.autoRotate':
				await orientAllOnPlate();
				break;
			case 'plate.reloadAll':
				await loadProjectModels();
				saveMessage = 'Modèles rechargés depuis le disque.';
				break;
		}
	}

	function removePlate(id: string) {
		const wasActive = plates.activeId === id;
		if (plates.removePlate(id) && wasActive) {
			ws = setSelection(ws, new Set());
			closeContext();
		}
	}

	function selectTool(tool: EditorTool) {
		tools = setTool(tools, tool);
		if (tools.active === 'measure') measuredPoints = [];
		// Peinture : un objet de 20 mm sur un plateau de 350 mm est minuscule ;
		// cadrer la sélection rend le pinceau utilisable dès l'ouverture.
		if (tools.active && PAINT_TOOLS.has(tools.active)) frameSelection();
	}
	const PAINT_TOOLS = new Set<EditorTool>([
		'support-paint',
		'seam-paint',
		'fuzzy-paint',
		'mm-paint'
	]);
	/** Cadre la caméra sur les objets sélectionnés (ou tous les objets du plateau). */
	function frameSelection() {
		const chosen = sceneObjects.filter((o) => ws.selection.has(o.id));
		const list = chosen.length ? chosen : objectsOnPlate(sceneObjects, activePlate);
		if (!list.length) return;
		const min: [number, number, number] = [Infinity, Infinity, Infinity];
		const max: [number, number, number] = [-Infinity, -Infinity, -Infinity];
		for (const obj of list) {
			const f = objectFootprint(obj, [bed.center.x, bed.center.y]);
			const pz = obj.mesh.positions;
			let lo = Infinity;
			let hi = -Infinity;
			for (let i = 2; i < pz.length; i += 3) {
				if (pz[i] < lo) lo = pz[i];
				if (pz[i] > hi) hi = pz[i];
			}
			const sz = Math.abs(obj.scale?.[2] ?? 1);
			const cz = obj.position?.[2] ?? 0;
			min[0] = Math.min(min[0], f.x - f.width / 2);
			max[0] = Math.max(max[0], f.x + f.width / 2);
			min[1] = Math.min(min[1], f.y - f.depth / 2);
			max[1] = Math.max(max[1], f.y + f.depth / 2);
			min[2] = Math.min(min[2], cz + lo * sz);
			max[2] = Math.max(max[2], cz + hi * sz);
		}
		if (min.every(Number.isFinite) && max.every(Number.isFinite)) {
			cameraZoom = 1;
			cameraFocus = { min, max };
		}
	}

	// Objet inerte : verrouillé dans la liste d'objets (Orca ObjectList) OU posé
	// sur un plateau verrouillé (lot 34) — Orca bloque aussi le déplacement des
	// objets d'un plateau verrouillé.
	function plateLockOf(id: string): boolean {
		const plateId = plates.plateOf(id);
		return plateId !== null && plates.get(plateId)?.locked === true;
	}
	function objectLocked(id: string): boolean {
		return tree.isLocked(id) || plateLockOf(id);
	}

	// Applique la transformation d'un objet manipulé par le gizmo (T103) : persiste
	// position/rotation/échelle dans l'état de scène (survit au re-rendu).
	function onTransform(id: string, transform: Transform) {
		if (!canTransformObject(id, activePlate, objectLocked)) return;
		// Phase 37 : pièce coupée → échelle uniforme seulement (comme Orca).
		if (tree.get(id)?.cutId) {
			const prev = sceneObjects.find((o) => o.id === id)?.scale ?? [1, 1, 1];
			if (transform.scale.some((s, a) => Math.abs(s - prev[a]) > 1e-9))
				transform = { ...transform, scale: uniformCutScale(prev, transform.scale) };
		}
		sceneObjects = sceneObjects.map((o) =>
			o.id === id
				? {
						...o,
						position: transform.position,
						rotation: transform.rotation,
						scale: transform.scale
					}
				: o
		);
	}

	// Contexte des panneaux d'outils (T104) : objet unique sélectionné + dérivés.
	const selId = $derived(ws.selection.size === 1 ? [...ws.selection][0] : null);
	const activeObject = $derived(selId ? (sceneObjects.find((o) => o.id === selId) ?? null) : null);
	const activeGizmoMode = $derived(
		selId && canTransformObject(selId, activePlate, objectLocked) ? gizmoMode : null
	);
	const activeTransform = $derived<Transform>({
		position: activeObject?.position ?? [0, 0, 0],
		rotation: activeObject?.rotation ?? [0, 0, 0],
		scale: activeObject?.scale ?? [1, 1, 1]
	});
	const activeBounds = $derived(
		activeObject
			? meshBounds(activeObject.mesh)
			: {
					min: [-100, -100, -100] as [number, number, number],
					max: [100, 100, 100] as [number, number, number]
				}
	);
	// Gizmo Flatten : face candidate cliquée → rotation (normale → −Z), puis pose.
	function flattenOnFace(id: string, rotation: [number, number, number]) {
		const object = sceneObjects.find((o) => o.id === id);
		if (!object) return;
		onTransform(id, {
			...objectTransform(object),
			rotation,
			position: [
				object.position?.[0] ?? 0,
				object.position?.[1] ?? 0,
				restingZ(object.mesh, rotation, object.scale)
			]
		});
	}

	// Raccourci « Poser à plat » : rotation remise à zéro puis posée sur le plateau.
	function layActiveFlat() {
		if (!activeObject) return;
		const rotation: [number, number, number] = [0, 0, 0];
		onTransform(activeObject.id, {
			...objectTransform(activeObject),
			rotation,
			position: [
				activeObject.position?.[0] ?? 0,
				activeObject.position?.[1] ?? 0,
				restingZ(activeObject.mesh, rotation, activeObject.scale)
			]
		});
	}

	// Emprise (mm) d'un maillage, pour borner les curseurs (ex. plan de coupe).
	function meshBounds(mesh: SceneMesh): {
		min: [number, number, number];
		max: [number, number, number];
	} {
		const p = mesh.positions;
		const min: [number, number, number] = [Infinity, Infinity, Infinity];
		const max: [number, number, number] = [-Infinity, -Infinity, -Infinity];
		for (let i = 0; i < p.length; i += 3) {
			for (let a = 0; a < 3; a++) {
				const v = p[i + a];
				if (v < min[a]) min[a] = v;
				if (v > max[a]) max[a] = v;
			}
		}
		if (!Number.isFinite(min[0])) return { min: [-100, -100, -100], max: [100, 100, 100] };
		return { min, max };
	}

	// Outils dont l'exécution passe par le moteur FFI (endpoints backlog v1) :
	// on informe l'utilisateur au lieu d'échouer silencieusement.
	// Menu Calibration (Plater::calib_temp d'Orca) : projet dédié prêt à trancher,
	// dérivé des presets actifs (imprimante, filament, process).
	// — Historique undo/redo (Ctrl+Z / Ctrl+Y / Ctrl+Maj+Z, parité Orca) : un
	//   instantané léger (arbre, plateaux, transformations, volumes) est empilé
	//   400 ms après la dernière modification — un glissement de gizmo donne
	//   donc une seule entrée. Les maillages ne sont jamais copiés : cache par
	//   objet, ou rechargement via le modèle serveur pour un objet rétabli.
	const sceneHistory = new SceneHistory();
	// Cache non réactif (lecture en différé uniquement).
	// eslint-disable-next-line svelte/prefer-svelte-reactivity
	const meshCache = new Map<string, SceneObject['mesh']>();
	let historyRestores = $state(0);
	let historyTimer: ReturnType<typeof setTimeout> | undefined;
	// Geste de gizmo en cours (objet ou volume) : l'historique attend sa fin
	// pour n'empiler qu'une étape, même si le glisser marque des pauses.
	let gizmoDragging = $state(false);
	let renamingObjectId = $state<string | null>(null);

	function currentSnapshot(): SceneSnapshot {
		const models: Record<string, string> = {};
		for (const item of imports) if (item.modelId) models[item.objectId] = item.modelId;
		return takeSnapshot({
			nodes: $state.snapshot(tree.list()) as unknown as SceneNode[],
			plates: $state.snapshot(plates.serialize()),
			objects: sceneObjects,
			volumes: $state.snapshot(volumesByObject) as Record<string, SceneVolume[]>,
			models
		});
	}

	function onGizmoDragChange(dragging: boolean) {
		clearTimeout(historyTimer);
		// Le geste peut commencer avant le debounce qui suit une création ou une
		// modification : enregistrer son état de départ immédiatement, sinon undo
		// reviendrait avant cette modification au lieu d'annuler seulement le geste.
		if (dragging) sceneHistory.record(currentSnapshot());
		gizmoDragging = dragging;
	}

	$effect(() => {
		const snap = currentSnapshot();
		const busy = historyRestores > 0 || gizmoDragging || imports.some((item) => isPending(item));
		clearTimeout(historyTimer);
		if (busy) return;
		historyTimer = setTimeout(() => {
			for (const o of sceneObjects) meshCache.set(o.id, o.mesh);
			if (sceneHistory.record(snap)) historyVersion++;
		}, 400);
	});

	function applySnapshot(snap: SceneSnapshot) {
		for (const o of sceneObjects) meshCache.set(o.id, o.mesh);
		const byId = new Map(sceneObjects.map((o) => [o.id, o]));
		const missing: string[] = [];
		sceneHistory.restoring(() => {
			tree.restore(snap.nodes);
			plates.restore(snap.plates);
			const next: SceneObject[] = [];
			for (const [id, t] of Object.entries(snap.transforms)) {
				const mesh = byId.get(id)?.mesh ?? meshCache.get(id);
				if (!mesh) {
					missing.push(id);
					continue;
				}
				next.push({
					...(byId.get(id) ?? { id, mesh }),
					position: [...t.position],
					rotation: [...t.rotation],
					scale: [...t.scale]
				});
			}
			sceneObjects = next;
			volumesByObject = structuredClone(snap.volumes);
			const kept = imports.filter((item) => item.objectId in snap.transforms);
			for (const [id, modelId] of Object.entries(snap.models)) {
				if (kept.some((item) => item.objectId === id)) continue;
				const name = tree.get(id)?.name ?? 'objet';
				kept.push(markConverted(markUploaded(startImport(id, name), modelId, false)));
			}
			imports = kept;
			if ([...ws.selection].some((id) => !(id in snap.transforms))) {
				ws = setSelection(
					ws,
					new SvelteSet([...ws.selection].filter((id) => id in snap.transforms))
				);
			}
		}, snap);
		// Objet rétabli dont le maillage n'est plus en cache : rechargé du serveur.
		for (const id of missing) {
			const modelId = snap.models[id];
			const t = snap.transforms[id];
			if (!modelId) continue;
			historyRestores++;
			loadCenteredMesh(modelId)
				.then(({ mesh }) => {
					meshCache.set(id, mesh);
					if (!sceneObjects.some((o) => o.id === id) && tree.get(id))
						sceneObjects = [
							...sceneObjects,
							{
								id,
								mesh,
								position: [...t.position],
								rotation: [...t.rotation],
								scale: [...t.scale]
							}
						];
				})
				.catch(() => (saveMessage = 'Rétablissement : maillage introuvable.'))
				.finally(() => historyRestores--);
		}
	}

	function undoScene() {
		// Outil de peinture actif : Ctrl+Z annule le dernier trait (une entrée
		// par appui, comme l'instantané de GLGizmoPainterBase).
		if (paintChannel && restorePaint(paintUndo, paintRedo)) return;
		undoSteps(1);
	}

	function redoScene() {
		if (paintChannel && restorePaint(paintRedo, paintUndo)) return;
		redoSteps(1);
	}

	// Liste d'historique (Orca) : plusieurs étapes d'un coup, un seul rendu.
	let historyVersion = $state(0);
	function undoSteps(steps: number) {
		clearTimeout(historyTimer);
		if (historyRestores === 0) sceneHistory.record(currentSnapshot());
		let snap: SceneSnapshot | undefined;
		for (let i = 0; i < steps; i++) snap = sceneHistory.undo() ?? snap;
		if (snap) applySnapshot(snap);
		historyVersion++;
	}
	function redoSteps(steps: number) {
		clearTimeout(historyTimer);
		let snap: SceneSnapshot | undefined;
		for (let i = 0; i < steps; i++) snap = sceneHistory.redo() ?? snap;
		if (snap) applySnapshot(snap);
		historyVersion++;
	}
	// Étapes affichées : l'état non encore enregistré (debounce de 400 ms)
	// compte comme une étape, comme au moment d'un Ctrl+Z.
	function flushHistory() {
		if (historyRestores > 0 || gizmoDragging) return;
		clearTimeout(historyTimer);
		if (sceneHistory.record(currentSnapshot())) historyVersion++;
	}

	let showCalibration = $state(false);
	let calibKind = $state<CalibrationKind>('temperature');
	let calibBusy = $state(false);
	let calibError = $state<string | null>(null);

	async function startCalibration(params: TemperatureParams | PressureAdvanceParams) {
		calibBusy = true;
		calibError = null;
		try {
			// Listes de presets éventuellement pas encore chargées : repli sur l'API.
			const ref = async (kind: 'machine' | 'process' | 'filament', id: string | null) => {
				const local = activePresetOf(kind);
				if (local) return { id: local.id, name: local.name };
				if (!id) return null;
				const p = await getPreset(id);
				return { id: p.id, name: p.name };
			};
			const printer = await ref('machine', activePresets.printer);
			const process = await ref('process', activePresets.process);
			const filament = await ref('filament', filamentSel ?? activePresets.filaments[0] ?? null);
			if (!printer || !process || !filament) {
				calibError = 'Sélectionnez d’abord une imprimante, un filament et un traitement.';
				return;
			}
			// Construite côté serveur (lot 14, portage des Plater::calib_* d'Orca).
			const request =
				calibKind === 'temperature'
					? temperatureRequest(params as TemperatureParams)
					: pressureAdvanceRequest(
							params as PressureAdvanceParams,
							(params as PressureAdvanceParams).method ?? 'tower'
						);
			const id = await createServerCalibration(
				request,
				serializeActivePresets(activePresets),
				calibKind === 'temperature' ? 'Calibration température' : 'Calibration PA'
			);
			showCalibration = false;
			// Navigation complète : même route projet, SvelteKit réutiliserait ce
			// composant (onMount non rejoué) et garderait la scène du projet source.
			window.location.assign(resolve('/projects/[id]', { id }));
		} catch (e) {
			calibError =
				e instanceof ApiError ? e.message : e instanceof Error ? e.message : 'création impossible';
		} finally {
			calibBusy = false;
		}
	}

	// Calibrations « mode moteur » (rétraction, VFA, débit max) : même flux que
	// température/PA, plus `scene.calibration` transmis au moteur au tranchage.
	let engineCalibOpen = $state(false);
	let engineCalibKind = $state<EngineCalibKind>('retraction');
	let engineCalibBusy = $state(false);
	let engineCalibError = $state<string | null>(null);
	// `scene.calibration` du projet ouvert, conservé à la sauvegarde.
	let sceneCalibration: Record<string, unknown> | null = null;
	// `scene.calibrationOverrides` (surcharges posées par le serveur), conservé aussi.
	let sceneCalibrationOverrides: Record<string, unknown> | null = null;

	// Calibration du débit (FlowRateCalibrationDialog, Plater::calib_flowrate) :
	// choix du type et du motif, puis création du projet dérivé.
	let flowCalibOpen = $state(false);
	let flowCalibBusy = $state(false);
	let flowCalibError = $state<string | null>(null);
	async function startFlowCalibration(
		pass: 1 | 2,
		linear = false,
		pattern: FlowPattern = 'monotonic'
	) {
		flowCalibBusy = true;
		flowCalibError = null;
		saveMessage = `Calibration du débit : création du projet…`;
		try {
			const ref = async (kind: 'machine' | 'process' | 'filament', id: string | null) => {
				const local = activePresetOf(kind);
				if (local) return { id: local.id, name: local.name };
				if (!id) return null;
				const preset = await getPreset(id);
				return { id: preset.id, name: preset.name };
			};
			const printer = await ref('machine', activePresets.printer);
			const process = await ref('process', activePresets.process);
			const filament = await ref('filament', filamentSel ?? activePresets.filaments[0] ?? null);
			if (!printer || !process || !filament) {
				flowCalibError = 'Sélectionnez une imprimante, un filament et un traitement.';
				return;
			}
			const id = await createServerCalibration(
				flowRequest(pass, linear, pattern),
				serializeActivePresets(activePresets),
				`Calibration du débit (passe ${pass})`
			);
			flowCalibOpen = false;
			window.location.assign(resolve('/projects/[id]', { id }));
		} catch (e) {
			flowCalibError = e instanceof ApiError ? e.message : e instanceof Error ? e.message : 'échec';
		} finally {
			flowCalibBusy = false;
		}
	}

	async function startEngineCalibration(params: RangeParams) {
		engineCalibBusy = true;
		engineCalibError = null;
		try {
			const ref = async (kind: 'machine' | 'process' | 'filament', id: string | null) => {
				const local = activePresetOf(kind);
				if (local) return { id: local.id, name: local.name };
				if (!id) return null;
				const preset = await getPreset(id);
				return { id: preset.id, name: preset.name };
			};
			const printer = await ref('machine', activePresets.printer);
			const process = await ref('process', activePresets.process);
			const filament = await ref('filament', filamentSel ?? activePresets.filaments[0] ?? null);
			if (!printer || !process || !filament) {
				engineCalibError = 'Sélectionnez d’abord une imprimante, un filament et un traitement.';
				return;
			}
			// Les 7 calibrations « mode moteur » (rétraction, VFA, débit max, motif
			// PA, input shaping, virages) sont construites par le serveur (lot 14).
			const id = await createServerCalibration(
				engineRequest(engineCalibKind, params),
				serializeActivePresets(activePresets),
				`Calibration ${engineCalibKind}`
			);
			engineCalibOpen = false;
			window.location.assign(resolve('/projects/[id]', { id }));
		} catch (e) {
			engineCalibError =
				e instanceof ApiError ? e.message : e instanceof Error ? e.message : 'création impossible';
		} finally {
			engineCalibBusy = false;
		}
	}

	// Outils Emboss (texte en relief, moteur) et Forme SVG (import extrudé).
	let embossFonts = $state<{ id: string; label: string }[]>([]);
	let embossBusy = $state(false);
	$effect(() => {
		if ((tools.active === 'emboss' || tools.active === 'svg') && embossFonts.length === 0)
			listEmbossFonts()
				.then((f) => (embossFonts = f))
				.catch(() => {});
	});

	async function createEmboss(params: EmbossParams) {
		const target =
			!params.embossed && selId ? (sceneObjects.find((o) => o.id === selId) ?? null) : null;
		const targetModel = target ? objectModelId(target.id) : null;
		if (!params.embossed && (!target || !targetModel)) {
			saveMessage = 'Gravure en creux : sélectionnez un objet.';
			return;
		}
		const label = params.source === 'svg' ? 'Forme SVG' : 'Texte en relief';
		embossBusy = true;
		try {
			// En creux : outil d'épaisseur double centré sur la face supérieure de
			// l'objet, puis différence booléenne → gravure de la profondeur demandée.
			const depth = params.embossed ? params.depth : params.depth * 2;
			const res =
				params.source === 'svg'
					? await embossSvg(data.project.id, {
							svg: params.svg,
							width_mm: params.svgWidth,
							depth_mm: depth
						})
					: await embossText(data.project.id, {
							text: params.text,
							font: params.font,
							size_mm: params.fontSize,
							depth_mm: depth,
							boldness_mm: params.boldness
						});
			if (target && targetModel) {
				const tool = res.parts[0];
				// Modèle outil connu du projet (jamais restauré comme orphelin).
				projectModelIds.add(tool.model.id);
				const toolMatrix = worldMatrix({
					position: [target.position?.[0] ?? 0, target.position?.[1] ?? 0, objectTopZ(target)],
					rotation: [0, 0, 0],
					scale: [1, 1, 1]
				});
				await runMeshOp(
					label + ' (creux)',
					[targetModel],
					() =>
						booleanModels(data.project.id, {
							op: 'difference',
							a: { model_id: targetModel, matrix: worldMatrix(objectTransform(target)) },
							b: { model_id: tool.model.id, matrix: toolMatrix }
						}),
					null,
					[target.id]
				);
				return;
			}
			// En relief sur l'objet sélectionné (gizmo Emboss d'Orca) : le relief devient
			// une pièce de l'objet, posée sur sa face supérieure, au lieu d'un objet
			// séparé superposé au centre du plateau.
			const host =
				params.embossed && params.onObject !== false && selId
					? (sceneObjects.find((o) => o.id === selId) ?? null)
					: null;
			if (host && res.parts[0]) {
				const part = res.parts[0];
				const { mesh } = await loadCenteredMesh(part.model.id);
				projectModelIds.add(part.model.id);
				volumeMeshRequests.add(part.model.id);
				volumeMeshes = { ...volumeMeshes, [part.model.id]: mesh };
				const pb = meshBounds(mesh);
				const size = [0, 1, 2].map((a) => Math.round((pb.max[a] - pb.min[a]) * 1000) / 1000) as [
					number,
					number,
					number
				];
				const name =
					params.source === 'svg' ? 'Forme SVG' : `Texte « ${params.text.trim().slice(0, 24)} »`;
				const volume: SceneVolume = {
					id: crypto.randomUUID(),
					type: 'part',
					name,
					shape: 'mesh',
					modelId: part.model.id,
					size,
					offset: embossPartOffset(meshBounds(host.mesh).max[2], size),
					settings: {},
					...(part.emboss ? { emboss: part.emboss } : {})
				};
				volumesByObject = {
					...volumesByObject,
					[host.id]: [...(volumesByObject[host.id] ?? []), volume]
				};
				ws = setSelection(ws, new Set([host.id]));
				saveMessage = `${label} : pièce « ${name} » ajoutée sur « ${tree.get(host.id)?.name ?? 'objet'} ».`;
				return;
			}
			const ids = await applyMeshOpParts(res, null, []);
			// Phase 28 : paramètres ré-éditables portés par l'objet (objects[].emboss).
			ids.forEach((id, i) => {
				const node = tree.get(id);
				const e = res.parts[i]?.emboss;
				if (node && e) node.emboss = e;
			});
			// Posé au centre du plateau (le maillage part de l'origine).
			sceneObjects = sceneObjects.map((o) =>
				ids.includes(o.id)
					? { ...o, position: [bed.center.x, bed.center.y, o.position?.[2] ?? 0] }
					: o
			);
			saveMessage = label + ' : objet créé.';
		} catch (e) {
			saveMessage = label + ' : ' + (e instanceof ApiError ? e.message : 'échec.');
		} finally {
			embossBusy = false;
		}
	}

	// — Phase 28 : ré-édition d'un texte / SVG en relief (Orca : double-clic sur
	//   le volume texte ou « Edit text ») — l'outil se rouvre avec les
	//   paramètres emboss, l'appliquer régénère le maillage à la même place.
	let embossEdit = $state<{
		objectId: string;
		volumeId: string | null;
		params: EmbossParams;
	} | null>(null);
	/** Cible ré-éditable d'un objet : volume sélectionné, objet, sinon 1er volume en relief. */
	function embossTarget(
		objectId: string | null
	): { volumeId: string | null; emboss: unknown } | null {
		if (!objectId) return null;
		const vols = volumesByObject[objectId] ?? [];
		const sel = vols.find((v) => v.id === selectedVolumeId && v.emboss);
		if (sel) return { volumeId: sel.id, emboss: sel.emboss };
		const own = tree.get(objectId)?.emboss;
		if (own) return { volumeId: null, emboss: own };
		const first = vols.find((v) => v.emboss);
		return first ? { volumeId: first.id, emboss: first.emboss } : null;
	}
	function openEmbossEdit(objectId: string | null): boolean {
		const target = embossTarget(objectId);
		const params = target ? paramsFromEmboss(target.emboss) : null;
		if (!objectId || !target || !params) return false;
		embossEdit = { objectId, volumeId: target.volumeId, params };
		if (tools.active !== 'emboss') tools = setTool(tools, 'emboss');
		return true;
	}
	async function applyEmbossEdit(params: EmbossParams) {
		const edit = embossEdit;
		if (!edit) return;
		const object = sceneObjects.find((o) => o.id === edit.objectId);
		if (!object) return;
		const label = params.source === 'svg' ? 'Forme SVG' : 'Texte en relief';
		embossBusy = true;
		try {
			const res =
				params.source === 'svg'
					? await embossSvg(data.project.id, {
							svg: params.svg,
							width_mm: params.svgWidth,
							depth_mm: params.depth,
							...(params.svgFileName ? { file_name: params.svgFileName } : {})
						})
					: await embossText(data.project.id, {
							text: params.text,
							font: params.font,
							size_mm: params.fontSize,
							depth_mm: params.depth,
							boldness_mm: params.boldness
						});
			const part = res.parts[0];
			if (!part) throw new Error('aucune pièce');
			const { mesh } = await loadCenteredMesh(part.model.id);
			projectModelIds.add(part.model.id);
			const b = meshBounds(mesh);
			const size = [0, 1, 2].map((a) => Math.round((b.max[a] - b.min[a]) * 1000) / 1000) as [
				number,
				number,
				number
			];
			if (edit.volumeId) {
				volumeMeshRequests.add(part.model.id);
				volumeMeshes = { ...volumeMeshes, [part.model.id]: mesh };
				const vols = volumesByObject[edit.objectId] ?? [];
				const old = vols.find((v) => v.id === edit.volumeId);
				// Posé sur le dessus : la base du relief reste à la même hauteur.
				const base = old ? old.offset[2] - old.size[2] / 2 : 0;
				volumesByObject = {
					...volumesByObject,
					[edit.objectId]: vols.map((v) =>
						v.id === edit.volumeId
							? {
									...v,
									modelId: part.model.id,
									size,
									offset: [
										v.offset[0],
										v.offset[1],
										Math.round((base + size[2] / 2) * 10000) / 10000
									],
									name:
										params.source === 'svg'
											? v.name
											: 'Texte « ' + params.text.trim().slice(0, 24) + ' »',
									...(part.emboss ? { emboss: part.emboss } : {})
								}
							: v
					)
				};
			} else {
				// Objet texte : nouveau modèle, même position (posé sur le plateau).
				imports = imports.map((i) =>
					i.objectId === edit.objectId ? { ...i, modelId: part.model.id } : i
				);
				sceneObjects = sceneObjects.map((o) =>
					o.id === edit.objectId
						? {
								...o,
								mesh,
								position: [
									o.position?.[0] ?? 0,
									o.position?.[1] ?? 0,
									restingZ(mesh, o.rotation, o.scale)
								]
							}
						: o
				);
				const node = tree.get(edit.objectId);
				if (node && part.emboss) node.emboss = part.emboss;
			}
			saveMessage = label + ' : relief mis à jour.';
			embossEdit = null;
			closeTool();
		} catch (e) {
			saveMessage =
				label + ' : ' + (e instanceof ApiError ? e.message : 'échec de la mise à jour.');
		} finally {
			embossBusy = false;
		}
	}
	$effect(() => {
		if (tools.active !== 'emboss' && tools.active !== 'svg' && embossEdit) embossEdit = null;
	});

	/** Z monde de la face supérieure (rotation ignorée : objets posés). */
	function objectTopZ(o: SceneObject): number {
		let max = -Infinity;
		const p = o.mesh.positions;
		for (let i = 2; i < p.length; i += 3) if (p[i] > max) max = p[i];
		return (o.position?.[2] ?? 0) + (Number.isFinite(max) ? max : 0) * (o.scale?.[2] ?? 1);
	}

	function closeTool() {
		if (tools.active) tools = setTool(tools, tools.active);
	}

	// Document de peinture de l'objet sélectionné (créé paresseusement).
	function paintingFor(id: string): TrianglePainting {
		let doc = paintings[id];
		if (!doc) {
			doc = new TrianglePainting();
			paintings[id] = doc;
		}
		return doc;
	}
	// Nombre de triangles peints dans le canal actif de l'objet sélectionné.
	function paintedCount(id: string | null, channel: PaintChannel | null): number {
		if (!id || !channel) return 0;
		const doc = paintings[id];
		if (!doc) return 0;
		const serialized = doc.serialize()[channel];
		return serialized ? Object.keys(serialized).length : 0;
	}
	const activePaintedCount = $derived(paintedCount(selId, paintChannel));
	// Peinture visible dans la vue 3D (Orca TriangleSelectorGUI) : canal de l'outil
	// ouvert sur l'objet sélectionné ; segmentation MMU de tous les objets en
	// multi-filament (couleur du filament peint).
	const scenePaintOverlays = $derived.by(() => {
		const out: Record<string, PaintOverlay | null> = {};
		let any = false;
		for (const o of sceneObjects) {
			const doc = paintings[o.id]?.serialize();
			if (!doc) continue;
			const channel: PaintChannel | null =
				paintChannel && o.id === selId ? paintChannel : filamentSlots > 1 && doc.mmu ? 'mmu' : null;
			if (!channel || !doc[channel]) continue;
			const overlay = paintOverlayGeometry(o.mesh, doc[channel], (s) =>
				channel === 'mmu'
					? (filamentSlotColors[s - 1] ?? null)
					: s === ENFORCER
						? PAINT_ENFORCER_COLOUR
						: s === BLOCKER
							? PAINT_BLOCKER_COLOUR
							: null
			);
			if (overlay) {
				out[o.id] = overlay;
				any = true;
			}
		}
		return any ? out : null;
	});

	// Canal moteur (worker persistant, WebSocket) : peinture TriangleSelector.
	let enginePaint: EnginePaintClient | null = null;
	const paintQueues = new SvelteMap<string, Promise<void>>();
	function enginePaintClient(): EnginePaintClient {
		return (enginePaint ??= new EnginePaintClient(paintWsUrl(data.project.id)));
	}
	$effect(() => () => enginePaint?.close());
	// Trait glissé en cours (appui → relâchement) ; caméra figée pendant le trait.
	let activeStroke: {
		id: string;
		channel: PaintChannel;
		before: Record<string, string>;
		recorder: StrokeRecorder;
	} | null = null;
	let paintDragging = $state(false);
	// Angle d'arrêt du pot de peinture (défaut Orca `m_smart_fill_angle` = 30°).
	let paintFillAngle = $state(30);
	// Annulation propre à la peinture : une entrée par trait (état d'avant).
	type PaintHistoryEntry = { id: string; channel: PaintChannel; facets: Record<string, string> };
	let paintUndo: PaintHistoryEntry[] = [];
	let paintRedo: PaintHistoryEntry[] = [];
	function restorePaint(from: PaintHistoryEntry[], to: PaintHistoryEntry[]): boolean {
		const entry = from.pop();
		if (!entry) return false;
		const doc = paintingFor(entry.id);
		to.push({ ...entry, facets: doc.serialize()[entry.channel] ?? {} });
		doc.replaceChannel(entry.channel, entry.facets);
		paintings = { ...paintings };
		return true;
	}

	/**
	 * Commit moteur d'un trait (un seul par appui, une entrée d'annulation) : lots
	 * successifs (≤ 4096 points, chevauchants) rejoués par le TriangleSelector
	 * d'Orca à partir de l'état d'avant le trait ; le résultat remplace l'aperçu
	 * TS et est seul persisté. Traits sérialisés par objet/canal.
	 */
	function commitEngineStroke(
		id: string,
		object: SceneObject,
		channel: PaintChannel,
		before: Record<string, string>,
		recorder: StrokeRecorder,
		fields: StrokeToolFields
	) {
		const modelId = objectModelId(id);
		if (!modelId || recorder.length === 0) return;
		const key = `${id}/${channel}`;
		const camera = recorder.camera;
		const common = {
			camera_pos: camera ?? recorder.points[0],
			radius: paintRadius,
			state: paintState,
			trafo: worldMatrix(objectTransform(object)),
			// Outil d'Orca (cercle / sphère / triangle / plage de hauteur / remplissages).
			...fields,
			...(fields.cursor === 'circle' && !camera ? { cursor: 'sphere' as const } : {})
		};
		const batches = recorder.batches();
		const previous = paintQueues.get(key);
		const run = async () => {
			// Base : état d'avant le trait, ou résultat moteur du trait précédent
			// (file séquentielle par objet/canal).
			let facets = previous ? (paintings[id]?.serialize()[channel] ?? before) : before;
			for (const batch of batches) {
				facets = await enginePaintClient().stroke(id, modelId, channel, facets, {
					...common,
					...batch
				});
			}
			const doc = paintings[id];
			if (!doc) return;
			doc.replaceChannel(channel, facets);
			paintings = { ...paintings };
		};
		const tail = (previous ?? Promise.resolve()).then(run).catch((e) => {
			// Échec moteur : retour à l'état d'avant le trait (l'aperçu TS n'est
			// jamais persisté à la place du résultat d'Orca).
			paintings[id]?.replaceChannel(channel, before);
			paintings = { ...paintings };
			saveMessage = 'Peinture (moteur) : ' + (e instanceof Error ? e.message : String(e));
		});
		paintQueues.set(key, tail);
		void tail.finally(() => {
			if (paintQueues.get(key) === tail) paintQueues.delete(key);
		});
		paintUndo.push({ id, channel, facets: before });
		paintRedo = [];
	}

	/** Applique l'outil actif à la facette réellement pickée dans le viewport. */
	function onMeshPick(id: string, pick: MeshPick) {
		const object = sceneObjects.find((o) => o.id === id);
		if (!object) return;
		if (paintChannel) {
			const doc = paintingFor(id);
			// Comblement : rien au clic (bouton « Combler » de la barre).
			if (!paintsOnClick(paintTool)) return;
			if (paintTool === 'fill') {
				// Remplissages d'Orca (smart fill des supports / fuzzy / couture, pot
				// du MMU avec ou sans arêtes) : calcul moteur seul, l'aperçu au
				// survol a déjà montré la zone.
				if (pick.phase === 'move' || pick.phase === 'up' || pick.triangle === null) return;
				const recorder = new StrokeRecorder(0);
				recorder.add(pick);
				const before = doc.serialize()[paintChannel] ?? {};
				commitEngineStroke(id, object, paintChannel, before, recorder, currentStrokeFields());
				clearHoverPreview();
				return;
			}
			if (pick.phase === 'up') {
				paintDragging = false;
				const stroke = activeStroke;
				activeStroke = null;
				const target = stroke ? sceneObjects.find((o) => o.id === stroke.id) : undefined;
				if (stroke && target) {
					const { channel, before, recorder } = stroke;
					commitEngineStroke(stroke.id, target, channel, before, recorder, currentStrokeFields());
				}
				return;
			}
			if (pick.phase === 'down' || !pick.phase) {
				activeStroke = {
					id,
					channel: paintChannel,
					before: doc.serialize()[paintChannel] ?? {},
					recorder: new StrokeRecorder(paintRadius * 0.1)
				};
				paintDragging = pick.phase === 'down';
			}
			if (!activeStroke || activeStroke.id !== id || !activeStroke.recorder.add(pick)) return;
			// Aperçu instantané (facettes entières, TS) pendant le glisser : disque /
			// sphère, ou le seul triangle touché (outil Triangle). La plage de
			// hauteur attend la réponse du moteur.
			const preview =
				paintTool === 'pointer'
					? pick.triangle !== null
						? [pick.triangle]
						: []
					: paintTool === 'height'
						? []
						: trianglesInRadius(object.mesh, pick.localPoint, paintRadius);
			if (preview.length) {
				doc.paint(paintChannel, preview, paintState);
				paintings = { ...paintings };
			}
			// Clic simple (sans phases) : trait d'un point, commit immédiat.
			if (!pick.phase) onMeshPick(id, { ...pick, phase: 'up' });
			return;
		}
		if (tools.active === 'measure') {
			measuredPoints = [...measuredPoints, pick.point].slice(-2);
			return;
		}
		if (tools.active === 'assembly' && assemblyMode !== 'explode') {
			const normal =
				assemblyMode === 'face' && pick.triangle !== null
					? worldTriangleNormal(
							object.mesh,
							pick.triangle,
							object.rotation ?? [0, 0, 0],
							object.scale ?? [1, 1, 1]
						)
					: null;
			// Face à face : point de la face = son centre (Orca `Measure`, cog du plan ;
			// `set_to_center_coincidence`), pas le point cliqué.
			const point =
				normal && pick.triangle !== null
					? worldFaceCenter(
							object.mesh,
							pick.triangle,
							object.position ?? [0, 0, 0],
							object.rotation ?? [0, 0, 0],
							object.scale ?? [1, 1, 1]
						)
					: pick.point;
			const first = assemblyPick;
			if (!first || first.objectId === id) {
				assemblyPick = { objectId: id, point, normal };
				return;
			}
			// 2e entité : son objet est amené contre la 1re (GLGizmoAssembly).
			const placement = objectTransform(object);
			if (assemblyMode === 'face' && first.normal && normal) {
				const aligned = alignFaces(
					{ position: placement.position, rotation: placement.rotation },
					point,
					normal,
					first.point,
					first.normal
				);
				onTransform(id, { ...placement, ...aligned });
			} else {
				onTransform(id, {
					...placement,
					position: alignPoints(placement.position, pick.point, first.point)
				});
			}
			assemblyPick = null;
			saveMessage = 'Assemblage appliqué.';
		}
		if (tools.active === 'brim-ears') {
			const ears = brimEarsByObject[id] ?? new BrimEars();
			ears.add(pick.point[0], pick.point[1]);
			brimEarsByObject = { ...brimEarsByObject, [id]: ears };
		}
	}

	function removeBrimEar(index: number) {
		if (!selId) return;
		const ears = brimEarsByObject[selId];
		if (!ears) return;
		ears.removeAt(index);
		brimEarsByObject = { ...brimEarsByObject };
	}
	// Efface le canal actif de l'objet sélectionné (réaffecte l'index → réactif).
	function clearActivePaint() {
		if (!selId || !paintChannel) return;
		const doc = paintingFor(selId);
		paintUndo.push({
			id: selId,
			channel: paintChannel,
			facets: doc.serialize()[paintChannel] ?? {}
		});
		paintRedo = [];
		doc.clear(paintChannel);
		paintings = { ...paintings };
	}

	// — Lot 38 : outils des gizmos de peinture d'Orca ———————————————————
	const paintParams = $derived({
		radius: paintRadius,
		height: paintHeight,
		fillAngle: paintFillAngle,
		edgeDetection: paintEdgeDetection,
		gapArea: paintGapArea
	});
	function currentStrokeFields(): StrokeToolFields {
		return strokeFields(paintChannel ?? 'supports', paintTool, paintParams);
	}
	// Outil absent du canal ouvert (ex. Plage de hauteur hors MMU) → pinceau.
	$effect(() => {
		if (paintChannel) {
			const tool = coerceTool(paintChannel, paintTool);
			if (tool !== paintTool) paintTool = tool;
		}
	});

	/**
	 * Opération moteur sur tout un canal (comblement, remappage) : sérialisée
	 * derrière les traits en cours du même objet / canal, une entrée d'annulation.
	 */
	function runPaintOp(
		id: string,
		channel: PaintChannel,
		op: (modelId: string, facets: Record<string, string>) => Promise<Record<string, string>>,
		done?: (before: Record<string, string>, after: Record<string, string>) => void
	): boolean {
		const modelId = objectModelId(id);
		if (!modelId) {
			saveMessage = 'Peinture : l’objet n’est pas encore enregistré sur le serveur.';
			return false;
		}
		const key = `${id}/${channel}`;
		const before = paintingFor(id).serialize()[channel] ?? {};
		const previous = paintQueues.get(key);
		paintBusy = true;
		const tail = (previous ?? Promise.resolve())
			.then(async () => {
				const base = previous ? (paintings[id]?.serialize()[channel] ?? before) : before;
				const after = await op(modelId, base);
				paintings[id]?.replaceChannel(channel, after);
				paintings = { ...paintings };
				done?.(base, after);
			})
			.catch((e) => {
				saveMessage = 'Peinture (moteur) : ' + (e instanceof Error ? e.message : String(e));
			})
			.finally(() => {
				paintBusy = false;
			});
		paintQueues.set(key, tail);
		void tail.finally(() => {
			if (paintQueues.get(key) === tail) paintQueues.delete(key);
		});
		paintUndo.push({ id, channel, facets: before });
		paintRedo = [];
		return true;
	}

	/** « Combler » (Orca Gap fill › Perform) sur l'objet sélectionné. */
	function performGapFill() {
		const id = selId;
		const channel = paintChannel;
		if (!id || !channel || paintGapArea <= 0) return;
		const area = paintGapArea;
		runPaintOp(
			id,
			channel,
			(modelId, facets) => enginePaintClient().gapFill(id, modelId, channel, facets, area),
			(before, after) => {
				const changed = JSON.stringify(before) !== JSON.stringify(after);
				saveMessage = changed
					? `Comblement : zones de moins de ${area.toLocaleString('fr-FR')} mm² absorbées.`
					: 'Comblement : aucune zone assez petite.';
			}
		);
	}

	/**
	 * « Remapper les filaments » (Orca remap_filament_assignments) : peinture
	 * MMU de l'objet par le moteur (remap_triangle_state), puis extrudeur de
	 * base avec la même table — celui de chaque pièce qui a le sien, sinon celui
	 * de l'objet.
	 */
	function remapPaintFilaments(table: number[]) {
		const id = selId;
		const map = remapMessageMap(table);
		if (!id || !map) return;
		const mmu = paintings[id]?.serialize().mmu ?? {};
		if (Object.keys(mmu).length > 0)
			runPaintOp(id, 'mmu', (modelId, facets) =>
				enginePaintClient().remap(id, modelId, facets, map)
			);
		const node = tree.get(id);
		let baseChanged = false;
		const volumes = volumesByObject[id] ?? [];
		for (const v of volumes) {
			if (v.type !== 'part' || !v.extruder) continue;
			const dst = remapBaseExtruder(v.extruder, table);
			if (dst !== null) {
				updateVolume(id, v.id, { extruder: dst });
				baseChanged = true;
			}
		}
		if (node) {
			const dst = remapBaseExtruder(node.extruder, table);
			if (dst !== null) {
				tree.setExtruder(id, dst);
				baseChanged = true;
			}
		}
		saveMessage = `Filaments remappés${Object.keys(mmu).length ? ' (peinture' + (baseChanged ? ' et filament de base)' : ')') : baseChanged ? ' (filament de base)' : ''}.`;
	}

	// Aperçu au survol (Orca : surbrillance de la zone avant le clic) — envoyé
	// au seul changement de facette / d'outil ; réponses dépassées ignorées.
	let hoverPreview = $state<{ id: string; preview: HoverPreview } | null>(null);
	let hoverKey = '';
	function clearHoverPreview() {
		hoverKey = '';
		enginePaint?.cancelPreviews();
		if (hoverPreview) hoverPreview = null;
	}
	// Couleur de l'aperçu : celle de l'état peint, éclaircie (Orca surligne la
	// sélection du remplissage) pour rester visible sur un objet de même couleur.
	function paintStateColour(channel: PaintChannel, state: number): string {
		const base =
			channel === 'mmu'
				? (filamentSlotColors[state - 1] ?? '#f59e0b')
				: state === BLOCKER
					? PAINT_BLOCKER_COLOUR
					: PAINT_ENFORCER_COLOUR;
		const m = /^#?([0-9a-f]{6})$/i.exec(base);
		if (!m) return base;
		const n = parseInt(m[1], 16);
		const mix = (v: number) => Math.round(v + (255 - v) * 0.55);
		const rgb = [mix(n >> 16), mix((n >> 8) & 255), mix(n & 255)];
		return '#' + rgb.map((v) => v.toString(16).padStart(2, '0')).join('');
	}
	function onPaintHover(pick: MeshPick | null) {
		const id = selId;
		const channel = paintChannel;
		if (!pick || pick.triangle === null || !id || !channel || !hasHoverPreview(paintTool)) {
			clearHoverPreview();
			return;
		}
		const object = sceneObjects.find((o) => o.id === id);
		const modelId = objectModelId(id);
		if (!object || !modelId) return;
		const fields = currentStrokeFields();
		const key = `${id}/${channel}/${pick.triangle}/${fields.tool}/${fields.fill_angle ?? ''}/${paintState}`;
		if (key === hoverKey) return;
		hoverKey = key;
		const facets = paintings[id]?.serialize()[channel] ?? {};
		const colour = paintStateColour(channel, paintState);
		void enginePaintClient()
			.preview(id, modelId, channel, facets, {
				points: [pick.localPoint],
				facets: [pick.triangle],
				camera_pos: pick.localCamera ?? pick.localPoint,
				radius: paintRadius,
				state: paintState,
				trafo: worldMatrix(objectTransform(object)),
				...fields
			})
			.then((res) => {
				if (res.stale || hoverKey !== key) return;
				hoverPreview = {
					id,
					preview: { positions: new Float32Array(res.vertices ?? []), color: colour }
				};
			})
			.catch(() => {
				// Aperçu facultatif : une erreur ne doit pas gêner la peinture.
			});
	}
	// Outil, canal ou réglages changés : l'aperçu courant n'est plus valable.
	$effect(() => {
		void paintTool;
		void paintChannel;
		void paintFillAngle;
		void paintEdgeDetection;
		void paintState;
		untrack(clearHoverPreview);
	});
	const sceneHoverPreviews = $derived(
		hoverPreview && paintChannel ? { [hoverPreview.id]: hoverPreview.preview } : null
	);
	const scenePaintAssist = $derived<PaintAssist | null>(
		paintChannel ? { constraint: paintConstraint, onhover: onPaintHover } : null
	);
	// Vue réaliste (préférence + menu Vue), suspendue pendant les gizmos
	// Supports / Couture / Fuzzy comme dans Orca.
	const realisticView = $derived(effectiveRealistic($appPreferences.graphics, paintChannel));

	// Items de la barre de plateau en état actif (bascules visuelles).
	const plateToolActive = $derived(
		new Set([
			...(showLayerHeight ? ['layersediting'] : []),
			...(tools.active === 'assembly' ? ['assembly_view'] : [])
		])
	);
	// Étendue Z (mm) de l'objet sélectionné, pour l'éditeur de hauteur de couche.
	const activeZMax = $derived(activeBounds.max[2] - activeBounds.min[2] || 10);
	// Profil de couche courant (par objet, uniforme par défaut).
	const activeLayerProfile = $derived(
		selId
			? (layerProfiles[selId] ?? uniformProfile(activeZMax, 0.2))
			: uniformProfile(activeZMax, 0.2)
	);
	// Persiste le profil de couche édité pour l'objet sélectionné.
	function setLayerProfile(profile: LayerBand[]) {
		if (!selId) return;
		layerProfiles = { ...layerProfiles, [selId]: profile };
	}
	// Pinceau de hauteur de couche : `adjust_layer_height_profile` (Slicing.cpp)
	// via le worker persistant (canal WebSocket de peinture).
	async function engineLayerBrush(
		z: number,
		action: 'increase' | 'decrease' | 'reduce' | 'smooth',
		delta: number,
		bandWidth: number
	) {
		const id = selId;
		const object = id ? sceneObjects.find((o) => o.id === id) : undefined;
		const modelId = id ? objectModelId(id) : null;
		if (!id || !object || !modelId) {
			saveMessage = 'Hauteur de couche : sélectionnez un objet dont le maillage est prêt.';
			return;
		}
		try {
			const profile = await enginePaintClient().layerAdjust({
				model_id: modelId,
				values: { ...machineValues, ...processValues },
				matrix: worldMatrix(objectTransform(object)),
				profile: layerProfiles[id] ? serializeLayerProfile(layerProfiles[id]) : [],
				z,
				delta,
				band_width: bandWidth,
				action
			});
			layerProfiles = { ...layerProfiles, [id]: deserializeLayerProfile(profile) };
		} catch (e) {
			saveMessage = 'Hauteur de couche : ' + (e instanceof Error ? e.message : 'erreur moteur');
		}
	}
	// Adaptatif / lissage : calculés par libslic3r (Slicing.cpp) dans le moteur,
	// pas en TS — le profil persisté est exactement celui d'Orca.
	async function engineLayerProfile(
		op:
			{ kind: 'adaptive'; quality: number } | { kind: 'smooth'; radius: number; keep_min: boolean }
	) {
		const id = selId;
		const object = id ? sceneObjects.find((o) => o.id === id) : undefined;
		const modelId = id ? objectModelId(id) : null;
		if (!id || !object || !modelId) {
			saveMessage = 'Hauteur de couche : sélectionnez un objet dont le maillage est prêt.';
			return;
		}
		try {
			const res = await layerProfileModel(modelId, {
				op:
					op.kind === 'adaptive'
						? op
						: { ...op, profile: serializeLayerProfile(activeLayerProfile) },
				values: { ...machineValues, ...processValues },
				matrix: worldMatrix(objectTransform(object))
			});
			layerProfiles = { ...layerProfiles, [id]: deserializeLayerProfile(res.profile) };
		} catch (e) {
			saveMessage =
				'Hauteur de couche : ' + (e instanceof ApiError ? e.message : 'erreur moteur inconnue');
		}
	}

	// Orchestration de la barre d'outils de plateau (T106) : chaque item déclenche
	// une action réelle (locale, ou moteur via `lib/api/scene` avec repli message).
	async function onPlateAction(id: string) {
		switch (id) {
			case 'add':
				// Menu d'ajout (Load + primitives) ancré sous la barre de plateau (T113).
				addMenu = { open: true, x: Math.max(12, window.innerWidth / 2 - 380), y: 96 };
				break;
			case 'addplate':
				plates.addPlate();
				break;
			case 'arrange':
				// Orca : le bouton Arrange ouvre le menu d'options (toggable),
				// l'arrangement part des boutons « Tout arranger » / « Ce plateau ».
				await toggleArrangePanel();
				break;
			case 'orient':
				await orientActive();
				break;
			case 'more':
				duplicateSelected();
				break;
			case 'fewer':
				removeSelected();
				break;
			case 'splitobjects':
				await splitObject(selId);
				break;
			case 'splitvolumes':
				await splitObjects(selectedIds, 'parts');
				break;
			case 'layersediting':
				showLayerHeight = !showLayerHeight;
				break;
			case 'assembly_view':
				selectTool('assembly');
				break;
		}
	}

	// Arrangement sans collision : empreintes → endpoint moteur → placements appliqués.
	// Arrangement en cours : la sauvegarde l'attend (sinon, cliquée pendant que
	// la réponse moteur est décodée, elle persistait les positions d'avant).
	let arrangeInFlight: Promise<void> | null = null;
	function arrangeActive(): Promise<void> {
		const run = arrangeActiveNow();
		arrangeInFlight = run;
		void run.finally(() => {
			if (arrangeInFlight === run) arrangeInFlight = null;
		});
		return run;
	}
	async function arrangeActiveNow() {
		const candidates = movableObjectsOnPlate(sceneObjects, activePlate, objectLocked);
		if (candidates.length === 0) return;
		try {
			const res = await arrangeScene(data.project.id, {
				bed_width: bed.width,
				bed_depth: bed.depth,
				// En objet par objet, le moteur porte lui-même l'écart au rayon de
				// dégagement de la tête (Orca update_selected_items_inflation).
				spacing: 6,
				items: arrangeItems(candidates),
				bed_shape: bed.polygon.map((point) => [point.x, point.y]),
				// Zone d'exclusion du preset ; « 0x0 » (Orca : aucune zone) n'est pas un polygone.
				excluded: ((pts) =>
					pts.length >= 3 ? [pts.map((p) => [p.x, p.y] as [number, number])] : [])(
					parseAreaPoints(machineValues.bed_exclude_area)
				),
				max_height: bed.height,
				// Arrangement séquentiel d'Orca (ArrangeJob init_arrange_params) :
				// séquence effective du plateau et dégagements de la tête (valeurs
				// machine, brouillon compris). Explicite aussi en couche par couche,
				// pour qu'un plateau « par couche » prime sur le preset.
				...(sequentialPrint
					? sequentialArrangeFields(machineValues, processValues)
					: { is_seq_print: false })
			});
			// Ne jamais réinjecter un placement pour un objet verrouillé ou d'un
			// autre plateau, même si une réponse moteur contient des ids en trop.
			const arranged = applyPlacements(candidates, res.placements);
			const byId = new Map(arranged.map((object) => [object.id, object]));
			sceneObjects = sceneObjects.map((object) => byId.get(object.id) ?? object);
			// Objet par objet : Orca fixe l'ordre d'impression à l'issue de
			// l'arrangement (arrange_order → print_order 1..N) ; la liste d'objets,
			// donc les étiquettes « Séquence #N » et le G-code, le suivent.
			const order = sequentialPrint
				? orderFromPlacements(res.placements.filter((p) => byId.has(p.id)))
				: null;
			if (order) {
				tree.setOrder(order);
				sceneObjects = permuteInSlots(sceneObjects, order, (o) => o.id);
			}
			saveMessage =
				`Arrangement : ${res.placements.length} objet(s) repositionné(s)` +
				(order ? ', ordre d’impression mis à jour.' : '.');
		} catch (e) {
			saveMessage = e instanceof ApiError ? e.message : 'arrangement indisponible';
		}
	}

	// — Options d'arrangement GUI (lot 34, menu « Options d'arrangement » d'Orca) —
	// Panneau ancré au bouton Arrange de la barre de plateau : options mémorisées
	// dans les préférences du compte (section arrange d'Orca), bornes et valeurs
	// de Réinitialiser fournies par GET /api/projects/{id}/arrange/options.
	let arrangePanel = $state(false);
	let arrangeInfo = $state<ArrangeOptionsInfo | null>(null);
	let arrangeBusy = $state(false);
	let arrangeSaved = $state<ArrangePreferences>(currentPreferences().arrange);
	async function toggleArrangePanel() {
		// Le bouton ouvre toujours le panneau (fermeture par ✕ ou Échap) : Orca
		// rouvre son menu d'arrangement au bouton même s'il est déjà affiché.
		arrangePanel = true;
		if (arrangePanel && !arrangeInfo) {
			try {
				const info = await getArrangeOptions(data.project.id);
				arrangeInfo = info;
				arrangeSaved = effectiveOptions(arrangeSaved, info);
			} catch {
				// Serveur sans la route (déploiement antérieur) : bornes par défaut.
				arrangeInfo = null;
			}
		}
	}
	function setArrangeOptions(next: ArrangePreferences) {
		arrangeSaved = next;
		savePreferences({ ...currentPreferences(), arrange: next });
	}

	// Arrangement GUI (ArrangeJob d'Orca) : enregistre la scène (l'arrangeur lit
	// plateaux, verrouillages et objets côté serveur), POST /arrange avec options
	// puis réapplique placements (matrice complète), réaffectations de plateau et
	// ordre séquentiel. `scope` = all (bouton « Arrange ») | plate (menu plateau).
	async function arrangeGui(scope: 'all' | 'plate') {
		if (arrangeBusy) return;
		arrangeBusy = true;
		try {
			// L'arrangeur GUI lit la scène persistée (plateaux, verrous, objets) :
			// la sauvegarde doit réussir avant l'appel, sinon on arrête là.
			const outcome = await save();
			if (outcome.status === 'error') {
				saveMessage = outcome.message;
				return;
			}
			if (outcome.status === 'conflict') {
				saveMessage =
					'Conflit d’enregistrement (projet ouvert ailleurs ?) : rechargez avant d’arranger.';
				return;
			}
			const list = plates.list();
			const activeIndex = Math.max(
				0,
				list.findIndex((p) => p.id === plates.activeId)
			);
			const res = await arrangeScene(data.project.id, {
				bed_width: bed.width,
				bed_depth: bed.depth,
				spacing: arrangeSaved.spacing,
				items: [],
				bed_shape: bed.polygon.map((point) => [point.x, point.y]),
				excluded: ((pts) =>
					pts.length >= 3 ? [pts.map((p) => [p.x, p.y] as [number, number])] : [])(
					parseAreaPoints(machineValues.bed_exclude_area)
				),
				max_height: bed.height,
				options: optionsBody(arrangeSaved),
				scope,
				plate_index: activeIndex
			});
			applyGuiArrangeResult(res);
		} catch (e) {
			saveMessage = e instanceof ApiError ? e.message : 'arrangement indisponible';
		} finally {
			arrangeBusy = false;
		}
	}

	function applyGuiArrangeResult(res: ArrangeResponse) {
		const byId = new Map(res.placements.map((pl) => [pl.id, pl]));
		sceneObjects = sceneObjects.map((o) => {
			const pl = byId.get(o.id);
			if (!pl) return o;
			const t = placedTransform(
				objectTransform(o),
				pl,
				arrangeSaved.enableRotation || arrangeSaved.alignToYAxis
			);
			return { ...o, position: t.position, rotation: t.rotation, scale: t.scale };
		});
		// Plateaux créés par l'arrangeur puis réaffectation des objets déplacés.
		const count = res.plate_count ?? plates.list().length;
		while (plates.list().length < count) plates.addPlate();
		for (const move of plateMoves(plates.list(), res.placements)) {
			const target = plates.list()[move.plateIndex];
			if (target) plates.assign(move.objectId, target.id);
		}
		// Ordre d'impression séquentiel (print_order 1..N d'Orca).
		if (res.placements.some((p) => p.print_order !== undefined && p.print_order !== null)) {
			const order = orderFromPlacements(res.placements.filter((p) => byId.has(p.id)));
			if (order) {
				tree.setOrder(order);
				sceneObjects = permuteInSlots(sceneObjects, order, (o) => o.id);
			}
		}
		const ignored = unarrangedIds(res.placements);
		const placed = res.placements.length - ignored.length;
		saveMessage =
			`Arrangement : ${placed} objet(s) sur ${count} plateau${count > 1 ? 'x' : ''}.` +
			(ignored.length > 0
				? ` ${ignored.length} objet(s) trop grand(s) pour un plateau, laissé(s) en l’état.`
				: '');
	}

	// Maj+Q : orientation auto de tous les objets mobiles du plateau actif.
	async function orientAllOnPlate() {
		const candidates = movableObjectsOnPlate(sceneObjects, activePlate, objectLocked);
		let oriented = 0;
		for (const object of candidates) {
			const modelId = imports.find((i) => i.objectId === object.id)?.modelId;
			if (!modelId) continue;
			try {
				const res = await orientModel(data.project.id, modelId);
				const current = sceneObjects.find((o) => o.id === object.id);
				if (!current) continue;
				const [rx, ry, rz] = res.rotation;
				onTransform(object.id, { ...objectTransform(current), rotation: [rx, ry, rz] });
				oriented++;
			} catch (e) {
				saveMessage = e instanceof ApiError ? e.message : 'orientation indisponible';
				return;
			}
		}
		saveMessage = `Orientation auto : ${oriented} objet(s) du plateau.`;
	}

	// Orientation auto de l'objet sélectionné : rotation Euler renvoyée par le moteur.
	async function orientActive() {
		if (!selId) return;
		if (!canTransformObject(selId, activePlate, objectLocked)) {
			saveMessage = 'Orientation : objet verrouillé ou hors du plateau actif.';
			return;
		}
		const modelId = imports.find((i) => i.objectId === selId)?.modelId;
		if (!modelId) {
			saveMessage = 'Orientation : modèle non résolu (conversion en cours).';
			return;
		}
		try {
			const res = await orientModel(data.project.id, modelId);
			const [rx, ry, rz] = res.rotation;
			onTransform(selId, { ...activeTransform, rotation: [rx, ry, rz] });
			saveMessage = 'Orientation auto appliquée.';
		} catch (e) {
			saveMessage = e instanceof ApiError ? e.message : 'orientation indisponible';
		}
	}

	// Instantané autonome d'un objet (presse-papier, duplication) : objet de
	// scène, suivi d'import (`modelId` → persistance), nœud d'arbre et documents
	// (peinture, profil de couche, oreilles), copiés par valeur pour survivre à la
	// suppression de la source (couper/coller).
	interface ObjectSnapshot {
		sourceId: string;
		object: SceneObject;
		importItem: ImportItem | null;
		name: string;
		extruder: number;
		settings: Record<string, unknown>;
		painting: PaintDocument | null;
		layerProfile: LayerBand[] | null;
		brimEars: BrimEarsDocument | null;
		volumes?: SceneVolume[];
	}

	function snapshotObject(id: string): ObjectSnapshot | null {
		const object = sceneObjects.find((o) => o.id === id);
		if (!object) return null;
		const node = tree.get(id);
		const savedDoc = savedDocumentByObject[id];
		const restored = savedDoc ? deserializeObject(savedDoc) : null;
		const importItem = imports.find((item) => item.objectId === id);
		return {
			sourceId: id,
			object: { ...object },
			importItem: importItem ? { ...importItem } : null,
			name: node?.name ?? 'Objet',
			extruder: node?.extruder ?? 0,
			settings: { ...(node?.settings ?? {}) },
			painting: (paintings[id] ?? restored?.painting)?.serialize() ?? null,
			layerProfile: layerProfiles[id] ? [...layerProfiles[id]] : null,
			brimEars: (brimEarsByObject[id] ?? restored?.brimEars)?.serialize() ?? null,
			volumes: structuredClone(
				$state.snapshot(volumesByObject[id] ?? restored?.volumes ?? []) as SceneVolume[]
			)
		};
	}

	// Crée une nouvelle instance depuis un instantané, sur le plateau actif. Voie
	// commune duplication / collage : le clone partage le modèle serveur
	// (`modelId`, donc sauvegardé et tranché) mais possède ses propres
	// transformations, réglages, peintures et affectations.
	function instantiateSnapshot(
		snap: ObjectSnapshot,
		position: [number, number, number],
		plateId: string | null = plates.activeId
	): string {
		// La source existe encore : clone d'arbre (sous-arbre de pièces inclus).
		// Sinon (coupé puis collé) : nouveau nœud portant nom et réglages copiés.
		const cloned = tree.get(snap.sourceId) ? tree.duplicate(snap.sourceId) : undefined;
		const copyId =
			cloned?.id ??
			tree.add(`${snap.name} (copie)`, {
				extruder: snap.extruder,
				settings: { ...snap.settings }
			}).id;
		if (snap.importItem) imports = [...imports, { ...snap.importItem, objectId: copyId }];
		if (plateId) plates.assign(copyId, plateId);
		if (snap.painting) paintings[copyId] = TrianglePainting.deserialize(snap.painting);
		if (snap.layerProfile) layerProfiles[copyId] = [...snap.layerProfile];
		if (snap.brimEars) brimEarsByObject[copyId] = BrimEars.deserialize(snap.brimEars);
		if (snap.volumes?.length) volumesByObject[copyId] = structuredClone(snap.volumes);
		const copy: SceneObject = { ...snap.object, id: copyId, position };
		if (snap.object.rotation) copy.rotation = [...snap.object.rotation];
		if (snap.object.scale) copy.scale = [...snap.object.scale];
		sceneObjects = [...sceneObjects, copy];
		return copyId;
	}

	// Copie un objet vers un plateau donné, à la même position. Objet affiché :
	// voie instantané (modèle, transformations, réglages, peinture, profils).
	// Objet sans maillage affiché (conversion en cours ou en échec) : copie du
	// suivi d'import, de la transformation et du document sauvegardés — le
	// maillage arrivera par `resolveConversion` comme pour la source.
	function copyObjectToPlate(id: string, plateId: string): string | null {
		const snap = snapshotObject(id);
		if (snap) {
			const pos = snap.object.position ?? [0, 0, 0];
			return instantiateSnapshot(snap, [pos[0], pos[1], pos[2]], plateId);
		}
		const importItem = imports.find((item) => item.objectId === id);
		if (!importItem?.modelId) return null;
		const node = tree.duplicate(id);
		if (!node) return null;
		const copyId = node.id;
		imports = [...imports, { ...importItem, objectId: copyId }];
		const t = savedTransformByObject[id];
		if (t)
			savedTransformByObject[copyId] = {
				position: [...t.position],
				rotation: [...t.rotation],
				scale: [...t.scale]
			};
		const doc = savedDocumentByObject[id];
		if (doc) savedDocumentByObject[copyId] = { ...structuredClone(doc), id: copyId };
		if (paintings[id]) paintings[copyId] = TrianglePainting.deserialize(paintings[id].serialize());
		if (layerProfiles[id]) layerProfiles[copyId] = [...layerProfiles[id]];
		if (brimEarsByObject[id])
			brimEarsByObject[copyId] = BrimEars.deserialize(brimEarsByObject[id].serialize());
		if (volumesByObject[id]?.length)
			volumesByObject[copyId] = structuredClone(
				$state.snapshot(volumesByObject[id]) as SceneVolume[]
			);
		plates.assign(copyId, plateId);
		return copyId;
	}

	// Édition › Dupliquer le plateau courant : nouveau plateau (id et nom
	// uniques) portant une copie de chaque instance du plateau actif, puis activé.
	function duplicateActivePlate() {
		const source = activePlate;
		if (!source) {
			saveMessage = 'Aucun plateau actif à dupliquer.';
			return;
		}
		// Un objet sans `modelId` (upload en cours) ne serait jamais rattaché à
		// son modèle : la copie serait perdue à la sauvegarde. On refuse plutôt
		// qu'une copie partielle.
		const uploading = source.objectIds.filter((id) => {
			const item = imports.find((i) => i.objectId === id);
			return !item?.modelId;
		});
		if (uploading.length > 0) {
			saveMessage = `Dupliquer le plateau : attendez la fin de ${uploading.length} upload(s).`;
			return;
		}
		let copied = 0;
		const plate = plates.duplicatePlate(source.id, (objectId, plateId) => {
			const copyId = copyObjectToPlate(objectId, plateId);
			if (copyId) copied += 1;
			return copyId;
		});
		if (!plate) {
			saveMessage = 'Dupliquer le plateau : plateau introuvable.';
			return;
		}
		ws = setSelection(ws, new Set());
		closeContext();
		const missed = source.objectIds.length - copied;
		saveMessage =
			`Plateau « ${plate.name} » créé : ${copied} objet(s) copié(s).` +
			(missed > 0 ? ` ${missed} objet(s) non copiable(s).` : '');
	}

	// Objet › Ajouter pièce / volume négatif / modificateur / bloqueur ou forceur
	// de supports : pavé générique centré sur l'objet (repère du maillage
	// recentré), transmis au tranchage avec son rôle (`RawVolume.role`).
	function addVolume(id: string, action: string) {
		const spec = VOLUME_ACTIONS[action];
		const object = sceneObjects.find((o) => o.id === id);
		if (!spec || !object) return;
		const p = object.mesh.positions;
		const min = [Infinity, Infinity, Infinity];
		const max = [-Infinity, -Infinity, -Infinity];
		for (let i = 0; i < p.length; i += 3)
			for (let a = 0; a < 3; a++) {
				min[a] = Math.min(min[a], p[i + a]);
				max[a] = Math.max(max[a], p[i + a]);
			}
		const extent = [0, 1, 2].map((a) => (Number.isFinite(min[a]) ? max[a] - min[a] : 10)) as [
			number,
			number,
			number
		];
		const list = volumesByObject[id] ?? [];
		const index = list.filter((v) => v.type === spec.type).length + 1;
		const volume = defaultVolume(spec.type, extent, crypto.randomUUID(), `${spec.name} ${index}`);
		volumesByObject = { ...volumesByObject, [id]: [...list, volume] };
		// Comme Orca (ObjectList::add_generic_subobject) : le volume ajouté est
		// sélectionné et ses réglages s'ouvrent dans l'onglet Objets.
		ws = setSelection(ws, new Set([id]));
		selectedVolumeId = volume.id;
		configTab = 'objects';
		saveMessage = `${volume.name} ajouté (${volume.size.join(' × ')} mm).`;
	}

	// Objet › Height range Modifier (Orca) : nouvelle plage de hauteur
	// ([0, 2] la première fois, puis après la dernière) avec layer_height courant.
	function addLayerRange(id: string) {
		const node = tree.get(id);
		const layerHeight = Number(node?.settings.layer_height ?? processValues.layer_height ?? 0.2);
		layerRangesByObject = {
			...layerRangesByObject,
			[id]: addRootRange(
				layerRangesByObject[id] ?? [],
				Number.isFinite(layerHeight) ? layerHeight : 0.2
			)
		};
		ws = setSelection(ws, new Set([id]));
		selectedVolumeId = null;
		configTab = 'objects';
	}

	function removeVolume(objectId: string, volumeId: string) {
		const list = volumesByObject[objectId] ?? [];
		volumesByObject = { ...volumesByObject, [objectId]: list.filter((v) => v.id !== volumeId) };
		if (selectedVolumeId === volumeId) selectedVolumeId = null;
	}

	// Remplace les surcharges d'un objet (persistées dans objects[].settings).
	function setObjectSettings(id: string, next: Record<string, unknown>) {
		const current = tree.get(id)?.settings ?? {};
		for (const key of Object.keys(current)) if (!(key in next)) tree.setSetting(id, key, undefined);
		for (const [key, value] of Object.entries(next)) tree.setSetting(id, key, value);
	}

	// Met à jour un volume (réglages, taille, position) → objects[].volumes[].
	// Remplace un volume (panneau de transformation : position/rotation/taille).
	function replaceVolume(objectId: string, next: SceneVolume) {
		const list = volumesByObject[objectId] ?? [];
		volumesByObject = {
			...volumesByObject,
			[objectId]: list.map((v) => (v.id === next.id ? next : v))
		};
	}

	// Clic sur un volume dans la vue 3D : sélectionne l'objet et ce volume.
	// Une pièce sélectionnée n'a de sens que dans son objet seul sélectionné
	// (Échap, clic sur un autre objet ou rectangle la désélectionnent).
	$effect(() => {
		const sel = [...ws.selection];
		if (
			selectedVolumeId &&
			!(sel.length === 1 && (volumesByObject[sel[0]] ?? []).some((v) => v.id === selectedVolumeId))
		)
			selectedVolumeId = null;
	});

	function pickVolume(objectId: string, volumeId: string) {
		ws = setSelection(ws, new Set([objectId]));
		selectedVolumeId = volumeId;
		configTab = 'objects';
	}

	function updateVolume(objectId: string, volumeId: string, patch: Partial<SceneVolume>) {
		const list = volumesByObject[objectId] ?? [];
		volumesByObject = {
			...volumesByObject,
			[objectId]: list.map((v) => (v.id === volumeId ? { ...v, ...patch } : v))
		};
	}

	// Modèles dont le maillage serveur est en cours de remplacement (réparation
	// ou simplification) : évite deux opérations concurrentes sur un modèle.
	const repairingModels = new SvelteSet<string>();

	// Après remplacement du maillage serveur d'un modèle : relit `/mesh` et le
	// pose sur toutes les instances (transforms, plateau et documents conservés).
	// Renvoie le suffixe d'avertissement (peinture) ou `null` si le rechargement
	// a échoué / la scène a changé entre-temps (message déjà posé).
	// Corps de tranchage : brouillon des presets (Orca tranche le preset édité)
	// et préférence « mélange haute/basse température » (lot 21).
	function currentDraft() {
		return buildSliceDraft({
			process: { resolved: processResolvedValues, current: processValues },
			printer: { resolved: machineResolvedValues, current: machineValues },
			filament: {
				slot: clampFilamentSlot(activePresets, filamentSlot),
				count: activePresets.filaments.length,
				resolved: filamentResolvedValues,
				current: filamentValues
			}
		});
	}
	function sliceRequestWithDraft(scope: 'plate' | 'all', plateIndex: number) {
		const draft = currentDraft();
		return {
			...sliceRequestFor(scope, plateIndex),
			...(draft ? { draft } : {}),
			...(currentPreferences().mixedTemperature ? { allow_mixed_temperatures: true } : {})
		};
	}
	// Contrôle à la volée des filaments haute/basse température (Orca Print.cpp:1149).
	let filamentCompatMessage = $state<{ text: string; blocking: boolean } | null>(null);
	let compatTimer: ReturnType<typeof setTimeout> | null = null;
	$effect(() => {
		const count = activePresets.filaments.length;
		void activePresets.filaments.join(',');
		void filamentValues;
		const allow = $appPreferences.mixedTemperature;
		if (compatTimer) clearTimeout(compatTimer);
		if (count < 2 || !baselineReady) {
			filamentCompatMessage = null;
			return;
		}
		compatTimer = setTimeout(() => {
			const draft = untrack(currentDraft);
			checkFilamentCompatibility(data.project.id, {
				...(draft ? { draft } : {}),
				active_presets: untrack(activePresetsBody),
				allow_mixed_temperatures: allow
			})
				.then((r) => {
					const text = compatibilityText(r);
					filamentCompatMessage = text ? { text, blocking: r.blocking } : null;
				})
				.catch(() => (filamentCompatMessage = null));
		}, 600);
	});

	async function reloadServerMesh(
		modelId: string,
		generation: number,
		what: string,
		meshChange = false
	): Promise<string | null> {
		// Simplifier / Réparer (Orca keep_painting) : peinture sauvegardée avant le
		// rechargement puis reportée sur le nouveau maillage, ou effacée.
		const before = meshChange
			? [...instancesOfModel(imports, modelId)]
					.map((oid) => ({ oid, channels: paintedChannels(paintings[oid]?.serialize()) }))
					.filter((p) => Object.keys(p.channels).length > 0)
			: [];
		// Le cache partagé par modèle contient l'ancien maillage : on l'invalide.
		meshRequests.delete(modelId);
		let mesh: SceneMesh;
		try {
			({ mesh } = await loadCenteredMesh(modelId));
		} catch (e) {
			if (generation !== sceneGeneration) return null;
			saveMessage = `${what} côté serveur, mais rechargement impossible : ${
				e instanceof ApiError ? e.message : 'erreur inconnue'
			}. Utilisez Fichier › Recharger.`;
			return null;
		}
		if (generation !== sceneGeneration) return null;
		const ids = instancesOfModel(imports, modelId);
		const previous = sceneObjects.find((o) => ids.has(o.id));
		const previousTriangles = previous ? previous.mesh.indices.length / 3 : null;
		sceneObjects = replaceMesh(sceneObjects, ids, mesh);
		if (before.length) {
			if (!currentPreferences().keepPainting) {
				for (const { oid } of before) delete paintings[oid];
				paintings = { ...paintings };
				return ' Peinture effacée (Préférences › Comportement › « Garder la peinture » pour la conserver).';
			}
			try {
				for (const { oid, channels } of before) {
					const remapped = await remapPaint(modelId, channels);
					const doc = paintingFor(oid);
					for (const [channel, facets] of Object.entries(remapped.channels))
						doc.replaceChannel(
							channel as Parameters<typeof doc.replaceChannel>[0],
							facets as unknown as Record<string, string>
						);
				}
				paintings = { ...paintings };
				return ' Peinture reportée sur le nouveau maillage.';
			} catch (e) {
				if (!(e instanceof ApiError && (e.status === 404 || e.status === 405))) {
					saveMessage = `Peinture non reportée : ${e instanceof ApiError ? e.message : 'erreur inconnue'}`;
				}
			}
		}
		// La peinture est indexée par triangle : si la topologie change, elle
		// est conservée mais peut ne plus correspondre aux facettes.
		const painted = [...ids].some((oid) =>
			Object.values(paintings[oid]?.serialize() ?? {}).some(
				(channel) => channel && Object.keys(channel).length > 0
			)
		);
		const topologyChanged =
			previousTriangles !== null && previousTriangles !== mesh.indices.length / 3;
		return painted && topologyChanged
			? ' Attention : le nombre de triangles a changé, vérifiez la peinture.'
			: '';
	}

	// Objet › Réparer : `POST /api/models/{id}/repair` (admesh côté worker, le
	// maillage serveur est remplacé), puis rechargement du maillage réparé sur
	// toutes les instances du modèle. Position/rotation/échelle, plateau et
	// documents (réglages, peinture, profils, oreilles) ne sont pas touchés.
	async function repairObject(id: string) {
		const target = repairTarget(id, imports);
		if (!target.ok) {
			saveMessage = `Réparer le maillage : ${target.reason}.`;
			return;
		}
		const { modelId } = target;
		if (repairingModels.has(modelId)) {
			saveMessage = 'Une opération sur ce maillage est déjà en cours…';
			return;
		}
		const generation = sceneGeneration;
		repairingModels.add(modelId);
		saveMessage = 'Réparation du maillage en cours…';
		try {
			let report;
			try {
				report = await repairModel(modelId);
			} catch (e) {
				if (generation !== sceneGeneration) return;
				saveMessage = `Réparation du maillage impossible : ${
					e instanceof ApiError ? e.message : 'erreur inconnue'
				}`;
				return;
			}
			if (generation !== sceneGeneration) return;
			const warning = await reloadServerMesh(modelId, generation, 'Maillage réparé', true);
			if (warning === null) return;
			saveMessage = repairSummary(report) + warning;
		} finally {
			repairingModels.delete(modelId);
		}
	}

	// Objet › Simplifier (outil) : `POST /api/models/{id}/simplify`
	// (`its_quadric_edge_collapse` libslic3r, maillage serveur remplacé), puis
	// rechargement sur toutes les instances, comme la réparation.
	async function simplifyObject(id: string, param: SimplifyToolParam) {
		const target = repairTarget(id, imports);
		if (!target.ok) {
			saveMessage = `Simplifier le maillage : ${target.reason}.`;
			return;
		}
		const request = simplifyRequest(param);
		if (!request.ok) {
			saveMessage = `Simplifier le maillage : ${request.reason}.`;
			return;
		}
		const { modelId } = target;
		if (repairingModels.has(modelId)) {
			saveMessage = 'Une opération sur ce maillage est déjà en cours…';
			return;
		}
		const generation = sceneGeneration;
		repairingModels.add(modelId);
		saveMessage = 'Simplification du maillage en cours…';
		try {
			let report;
			try {
				report = await simplifyModel(modelId, request.body);
			} catch (e) {
				if (generation !== sceneGeneration) return;
				saveMessage = `Simplification du maillage impossible : ${
					e instanceof ApiError ? e.message : 'erreur inconnue'
				}`;
				return;
			}
			if (generation !== sceneGeneration) return;
			const warning = await reloadServerMesh(modelId, generation, 'Maillage simplifié', true);
			if (warning === null) return;
			saveMessage = simplifySummary(report) + warning;
		} finally {
			repairingModels.delete(modelId);
		}
	}

	// Ordre des objets = ordre d'impression séquentielle (Orca : ordre du modèle,
	// glisser-déposer dans la liste d'objets). Arbre et scène réordonnés ensemble :
	// la scène sauvegardée (donc le tranchage) suit l'ordre de `sceneObjects`.
	const sequentialPrint = $derived(
		isByObject(activePlate?.settings?.print_sequence, processValues.print_sequence)
	);
	// Étiquettes « Séquence #N » sur la scène (Orca : sous le nom, en objet par
	// objet) : rang des objets imprimables du plateau actif, dans l'ordre de la
	// liste. Affichées par défaut, masquables depuis la liste d'objets.
	let showPrintOrder = $state(true);
	const scenePrintOrder = $derived.by(() => {
		if (!sequentialPrint || !showPrintOrder) return null;
		const rank: Record<string, number> = {};
		let n = 0;
		for (const node of tree.roots())
			if (node.kind === 'object' && activeObjectIds.has(node.id) && tree.isPrintable(node.id))
				rank[node.id] = ++n;
		return n > 0 ? rank : null;
	});
	// Valeurs des presets que suit un plateau sans surcharge (option « Profil (…) »).
	const plateInherited = $derived({ ...machineValues, ...processValues });
	// — Validation en direct (phase 29) : Print::validate côté serveur
	//   (POST /validate), rejouée après chaque modification avec un délai comme
	//   Plater::priv::update_background_process. Remplace l'estimation locale
	//   « objet par objet » de la phase 27 : le moteur est l'autorité (enveloppes
	//   convexes, jupe, zones d'exclusion, tour d'amorçage, hauteurs…). La route
	//   valide la scène **persistée** : une scène modifiée est d'abord
	//   enregistrée (comme avant chaque tranchage), sans message.
	let validation = $state<ValidateResponse | null>(null);
	let validationBusy = $state(false);
	let validationBlocked = $state<string | null>(null);
	let orcaCatalog = $state<OrcaCatalog | null>(null);
	// Ids persistés (scene.objects[].id) → ids vivants : l'arbre régénère les
	// ids à l'ouverture ; la première sauvegarde les aligne (carte vidée).
	const persistedToLive = new SvelteMap<string, string>();
	let validationTimer: ReturnType<typeof setTimeout> | undefined;
	let validationSeq = 0;
	$effect(() => {
		const sig = currentSignature;
		void JSON.stringify(currentDraft() ?? null);
		void $appPreferences.mixedTemperature;
		const tab = layout.tab;
		const ready = baselineReady;
		const pending = imports.some((item) => isPending(item));
		clearTimeout(validationTimer);
		if (!ready || pending || tab !== 'prepare' || sig === null) return;
		validationTimer = setTimeout(() => void runValidation(), 900);
	});
	async function runValidation() {
		const seq = ++validationSeq;
		if (sceneObjects.length === 0) {
			validation = null;
			validationBlocked = null;
			return;
		}
		validationBusy = true;
		try {
			orcaCatalog ??= await loadOrcaCatalog().catch(() => null);
			// Phase 30 (lot 29) : la scène affichée part dans le corps — plus
			// d'enregistrement silencieux, l'état « non enregistré » est conservé.
			// Repli sur l'enregistrement si le serveur ignore `scene`.
			const dirty = saveSignature() !== savedSignature;
			// Phase 33 (lot 31) : presets actifs envoyés aussi (`active_presets`).
			const inline = dirty && projectConfigSaved() && (await supportsInlineScene(data.project.id));
			if (seq !== validationSeq) return;
			if (dirty && !inline) {
				const saved = await save({ quiet: true });
				if (seq !== validationSeq) return;
				if (saved.status !== 'saved') {
					validationBlocked =
						saved.status === 'conflict'
							? 'Vérification suspendue : le projet a été modifié ailleurs. Rechargez-le.'
							: 'Vérification suspendue : ' + saved.message;
					return;
				}
			}
			const res = await validateProject(data.project.id, {
				...sliceRequestWithDraft('all', 0),
				...(inline ? { scene: buildSceneForSave(), active_presets: activePresetsBody() } : {})
			});
			if (seq !== validationSeq) return;
			validation = res;
			validationBlocked = null;
		} catch (e) {
			if (seq !== validationSeq) return;
			validationBlocked =
				'Vérification impossible : ' + (e instanceof ApiError ? e.message : 'serveur injoignable');
		} finally {
			if (seq === validationSeq) validationBusy = false;
		}
	}
	const validationNotices = $derived(
		validation
			? buildNotices(validation, {
					catalog: orcaCatalog,
					liveId: (id) => persistedToLive.get(id) ?? (tree.get(id) ? id : undefined),
					plateCount: plates.list().length
				})
			: []
	);
	const validationByObject = $derived(issuesByObject(validationNotices));
	const sceneIssueObjects = $derived.by(() => {
		const out: Record<string, 'error' | 'warning'> = {};
		for (const [id, issue] of Object.entries(validationByObject)) out[id] = issue.level;
		return Object.keys(out).length ? out : null;
	});
	// Liste d'objets : problèmes de validation + échecs de conversion (cause).
	const objectIssues = $derived.by(() => {
		const out: Record<string, { level: 'error' | 'warning'; text: string; inline?: boolean }> = {
			...validationByObject
		};
		for (const item of imports)
			if (item.status === 'failed')
				out[item.objectId] = {
					level: 'error',
					text: 'Conversion impossible : ' + (item.error ?? 'fichier illisible'),
					inline: true
				};
		return out;
	});
	const activePlateBlocked = $derived(
		plateBlocked(validation, plates.plateIndex(plates.activeId ?? ''))
	);
	function gotoIssueObject(id: string) {
		const plate = plates.list().find((p) => p.objectIds.includes(id));
		if (plate && plate.id !== plates.activeId) selectPlate(plate.id);
		ws = setSelection(ws, new Set([id]));
		configTab = 'objects';
	}
	function gotoIssueOption(notice: ValidationNotice) {
		if (!notice.optKey || !notice.optKind) return;
		if (notice.objectId) gotoIssueObject(notice.objectId);
		// Les dialogues de réglages vivent dans l'onglet « Traitement ».
		configTab = 'settings';
		openSettingsDialog(
			notice.optKind,
			optionPage(notice.optKey, notice.optKind) ?? undefined,
			notice.optKey
		);
	}
	function gotoIssuePlate(index: number) {
		const plate = plates.list()[index];
		if (plate) selectPlate(plate.id);
	}
	// — Tour d'essuyage sur le plateau (Préparer) : taille estimée comme Orca,
	//   position par plateau dans la config projet (wipe_tower_x/y).
	const firstNum = (v: unknown, d: number) => {
		const n = Number(Array.isArray(v) ? v[0] : v);
		return Number.isFinite(n) ? n : d;
	};
	const towerDefault = $derived<[number, number]>([
		firstNum(processValues.wipe_tower_x ?? machineValues.wipe_tower_x, 15),
		firstNum(processValues.wipe_tower_y ?? machineValues.wipe_tower_y, 220)
	]);
	const activePlateIndex = $derived(plates.plateIndex(plates.activeId ?? ''));
	const sceneWipeTower = $derived.by(() => {
		if (filamentSlots < 2) return null;
		const objs = activeSceneObjects.filter((o) => tree.isPrintable(o.id) && !tree.isHidden(o.id));
		const used = new SvelteSet<number>();
		let maxHeight = 0;
		for (const o of objs) {
			const ex = clampExtruder(tree.effectiveExtruder(o.id), filamentSlots) || 1;
			used.add(ex);
			for (const v of volumesByObject[o.id] ?? [])
				if (v.type === 'part' && v.extruder) used.add(v.extruder);
			maxHeight = Math.max(maxHeight, worldSize(o.mesh, o.rotation, o.scale)[2]);
		}
		if (
			!showWipeTower({
				enabled: primeTowerOn,
				plateExtruders: used.size,
				objects: objs.length,
				byObject: sequentialPrint
			})
		)
			return null;
		const nozzles = Array.isArray(machineValues.nozzle_diameter)
			? machineValues.nozzle_diameter.length
			: 1;
		const size = estimateWipeTowerSize({
			width: firstNum(processValues.prime_tower_width, 60),
			primeVolume: firstNum(processValues.prime_volume, 45),
			layerHeight: firstNum(processValues.layer_height, 0.2),
			infillGap: firstNum(
				String(processValues.prime_tower_infill_gap ?? '150').replace('%', ''),
				150
			),
			wallType: String(processValues.wipe_tower_wall_type ?? 'rib'),
			ribWidth: firstNum(processValues.wipe_tower_rib_width, 8),
			extraRibLength: firstNum(processValues.wipe_tower_extra_rib_length, 0),
			maxHeight,
			plateExtruders: used.size,
			nozzles,
			changeLength: firstNum(filamentValues.filament_change_length, 10),
			filamentDiameter: firstNum(filamentValues.filament_diameter, 1.75)
		});
		if (size[0] <= 0 || size[1] <= 0) return null;
		const brim = effectiveBrim(firstNum(processValues.prime_tower_brim_width, 3), size[2]);
		const [px, py] = towerPosition(projectFilament, activePlateIndex, towerDefault);
		const [x, y] = clampTowerPosition(px, py, size, bed, brim);
		return { x, y, size, brim, rotation: firstNum(processValues.wipe_tower_rotation_angle, 0) };
	});
	function moveWipeTower(x: number, y: number) {
		if (!sceneWipeTower) return;
		const [cx, cy] = clampTowerPosition(x, y, sceneWipeTower.size, bed, sceneWipeTower.brim);
		projectFilament = setTowerPosition(projectFilament, activePlateIndex, cx, cy, towerDefault);
		saveMessage = `Tour d'essuyage déplacée en X ${cx.toFixed(1)} · Y ${cy.toFixed(1)} mm.`;
	}

	function reorderObject(id: string, targetId: string) {
		if (!tree.moveTo(id, targetId)) return;
		// La scène suit l'ordre de l'arbre (et non un déplacement relatif : les
		// maillages chargés dans le désordre peuvent avoir permuté sceneObjects).
		sceneObjects = permuteInSlots(
			sceneObjects,
			tree.list().map((n) => n.id),
			(o) => o.id
		);
		const name = tree.get(id)?.name ?? 'objet';
		const rank =
			tree
				.roots()
				.filter((n) => n.kind === 'object')
				.findIndex((n) => n.id === id) + 1;
		saveMessage = `Ordre des objets : « ${name} » en position ${rank}.`;
	}

	// Duplique un objet donné (décalage de 10 mm en X/Y, Z conservé).
	function duplicateObject(id: string) {
		const snap = snapshotObject(id);
		if (!snap) return;
		const pos = snap.object.position ?? [0, 0, 0];
		const copyId = instantiateSnapshot(snap, [pos[0] + 10, pos[1] + 10, pos[2]]);
		ws = setSelection(ws, new Set([copyId]));
	}

	// — Découpes serveur (split / cut / boolean, worker moteur `mesh-op`) —
	// Chaque pièce est un nouveau modèle serveur au maillage prêt : on l'ajoute
	// comme objet (suivi d'import « prêt », plateau de la source) au placement
	// documenté par l'API, puis on retire la ou les sources (comme Orca).
	let meshOpBusy = $state(false);
	function objectModelId(id: string): string | null {
		return imports.find((item) => item.objectId === id)?.modelId ?? null;
	}

	function objectTransform(object: SceneObject) {
		return {
			position: (object.position ?? [0, 0, 0]) as [number, number, number],
			rotation: (object.rotation ?? [0, 0, 0]) as [number, number, number],
			scale: (object.scale ?? [1, 1, 1]) as [number, number, number]
		};
	}

	/** Ajoute les pièces à la scène puis retire les sources. `source` = objet dont
	 *  le repère porte les centres (split/cut) ; `null` = centres monde (booléen). */
	async function applyMeshOpParts(
		res: MeshOpResponse,
		source: SceneObject | null,
		removeIds: string[]
	): Promise<string[]> {
		const generation = sceneGeneration;
		const plateId = plates.activeId;
		const loaded = await Promise.all(
			res.parts.map(async (part) => ({ part, ...(await loadCenteredMesh(part.model.id)) }))
		);
		if (generation !== sceneGeneration) return [];
		const added: SceneObject[] = [];
		const newIds: string[] = [];
		for (const { part, mesh, center } of loaded) {
			projectModelIds.add(part.model.id);
			const objectId = tree.add(part.model.filename).id;
			if (plateId) plates.assign(objectId, plateId);
			imports = [
				...imports,
				markConverted(
					markUploaded(startImport(objectId, part.model.filename), part.model.id, false)
				)
			];
			let placed;
			if (source) {
				placed = placeSourcePart(objectTransform(source), center);
				// Pose sur le plateau, comme les objets issus d'une coupe dans Orca.
				placed.position[2] = restingZ(mesh, placed.rotation, placed.scale);
			} else {
				placed = {
					position: [center[0], center[1], restingZ(mesh)] as [number, number, number],
					rotation: [0, 0, 0] as [number, number, number],
					scale: [1, 1, 1] as [number, number, number]
				};
			}
			added.push({ id: objectId, mesh, ...placed });
			newIds.push(objectId);
		}
		sceneObjects = [...sceneObjects, ...added];
		for (const id of removeIds) removeObject(id);
		ws = setSelection(ws, new Set(newIds));
		return newIds;
	}

	async function runMeshOp(
		feature: string,
		modelIds: string[],
		call: () => Promise<MeshOpResponse>,
		source: SceneObject | null,
		removeIds: string[]
	) {
		if (meshOpBusy || modelIds.some((m) => repairingModels.has(m))) {
			saveMessage = `${feature} : une opération est déjà en cours sur ce modèle.`;
			return;
		}
		meshOpBusy = true;
		modelIds.forEach((m) => repairingModels.add(m));
		try {
			const res = await call();
			const ids = await applyMeshOpParts(res, source, removeIds);
			saveMessage = `${feature} : ${ids.length} objet(s) créé(s).`;
		} catch (e) {
			saveMessage =
				e instanceof ApiError
					? meshOpErrorMessage(feature, e.status, e.message)
					: meshOpErrorMessage(feature, null, 'erreur inconnue');
		} finally {
			modelIds.forEach((m) => repairingModels.delete(m));
			meshOpBusy = false;
		}
	}

	/** Objet › Séparer (en objets / en pièces) : `TriangleMesh::split`. */
	async function splitObject(id: string | null) {
		const object = id ? sceneObjects.find((o) => o.id === id) : undefined;
		const modelId = id ? objectModelId(id) : null;
		if (!object || !modelId) {
			saveMessage = 'Séparer : sélectionnez un objet dont le maillage est prêt.';
			return;
		}
		await runMeshOp('Séparation', [modelId], () => splitModel(modelId), object, [object.id]);
	}

	// Charge le maillage de chaque volume `mesh` (restauration, undo, split).
	$effect(() => {
		for (const list of Object.values(volumesByObject))
			for (const v of list ?? []) {
				const modelId = v.shape === 'mesh' ? v.modelId : undefined;
				if (!modelId || volumeMeshRequests.has(modelId)) continue;
				volumeMeshRequests.add(modelId);
				projectModelIds.add(modelId);
				loadCenteredMesh(modelId)
					.then(({ mesh }) => (volumeMeshes = { ...volumeMeshes, [modelId]: mesh }))
					.catch(() => volumeMeshRequests.delete(modelId));
			}
	});

	/**
	 * Barre d'outils › Split to parts (Orca `Plater::split_volume`) : l'objet
	 * reste un seul objet dont chaque composante devient un volume pièce avec
	 * son propre maillage (`shape: 'mesh'`). Le maillage de base de l'objet
	 * devient la première composante ; les suivantes sont placées en `offset`
	 * (différence des centres dans le repère recentré de la source).
	 */
	async function splitToParts(id: string | null): Promise<string | null> {
		const object = id ? sceneObjects.find((o) => o.id === id) : undefined;
		const modelId = id ? objectModelId(id) : null;
		const node = id ? tree.get(id) : undefined;
		if (!id || !object || !modelId || !node) {
			saveMessage = 'Séparer en pièces : sélectionnez un objet dont le maillage est prêt.';
			return null;
		}
		if (meshOpBusy || repairingModels.has(modelId)) {
			saveMessage = 'Séparer en pièces : une opération est déjà en cours sur ce modèle.';
			return null;
		}
		const generation = sceneGeneration;
		meshOpBusy = true;
		repairingModels.add(modelId);
		try {
			const res = await splitModel(modelId);
			const loaded = await Promise.all(
				res.parts.map(async (part) => ({ part, ...(await loadCenteredMesh(part.model.id)) }))
			);
			if (generation !== sceneGeneration || loaded.length === 0) return null;
			if (loaded.length < 2) {
				// Orca `ObjectList::split` : « The target object contains only one part
				// and can not be split. »
				saveMessage = `Séparer en pièces : « ${node.name} » ne contient qu'une pièce et ne peut pas être séparé.`;
				return null;
			}
			const [first, ...rest] = loaded;
			const c0 = first.center;
			for (const { part, mesh } of loaded) {
				projectModelIds.add(part.model.id);
				volumeMeshRequests.add(part.model.id);
				volumeMeshes = { ...volumeMeshes, [part.model.id]: mesh };
			}
			// Pièce d'un objet multi-pièces : le résultat reste dans le même parent.
			// Le maillage propre de l'objet est le 1er volume d'Orca : `<nom>_1`.
			const objectId = tree.add(node.name, {
				parentId: node.parentId,
				extruder: node.extruder,
				settings: { ...node.settings },
				printable: node.printable,
				meshName: `${node.name}_1`
			}).id;
			if (plates.activeId) plates.assign(objectId, plates.plateOf(id) ?? plates.activeId);
			imports = [
				...imports,
				markConverted(markUploaded(startImport(objectId, node.name), first.part.model.id, false))
			];
			const bounds = (m: SceneMesh) => {
				const b = meshBounds(m);
				return [0, 1, 2].map((a) => Math.round((b.max[a] - b.min[a]) * 1000) / 1000) as [
					number,
					number,
					number
				];
			};
			const shift = (v: [number, number, number]) =>
				[0, 1, 2].map((a) => Math.round((v[a] - c0[a]) * 10000) / 10000) as [
					number,
					number,
					number
				];
			// Noms Orca (`ModelVolume::split`) : `<nom>_<n>` ; la 1re composante reste le
			// maillage de l'objet (`<nom>_1`), les suivantes sont `<nom>_2`…
			const partVolumes: SceneVolume[] = rest.map(({ part, mesh, center }, i) => ({
				id: crypto.randomUUID(),
				type: 'part',
				name: `${node.name}_${i + 2}`,
				shape: 'mesh',
				modelId: part.model.id,
				size: bounds(mesh),
				offset: shift(center),
				settings: {}
			}));
			// Volumes existants (modificateurs…) : même repère, recentré sur la 1re pièce.
			const kept = (volumesByObject[id] ?? []).map((v) => ({
				...structuredClone($state.snapshot(v) as SceneVolume),
				offset: shift(v.offset)
			}));
			const placed = placeSourcePart(objectTransform(object), c0);
			sceneObjects = [...sceneObjects, { id: objectId, mesh: first.mesh, ...placed }];
			volumesByObject = { ...volumesByObject, [objectId]: [...partVolumes, ...kept] };
			removeObject(id);
			ws = setSelection(ws, new Set([objectId]));
			saveMessage = `Séparation en pièces : ${loaded.length} pièce(s) dans « ${node.name} ».`;
			return objectId;
		} catch (e) {
			saveMessage =
				e instanceof ApiError
					? meshOpErrorMessage('Séparation en pièces', e.status, e.message)
					: meshOpErrorMessage('Séparation en pièces', null, 'erreur inconnue');
			return null;
		} finally {
			repairingModels.delete(modelId);
			meshOpBusy = false;
		}
	}

	/** Pièce d'un objet multi-pièces : enfant d'un nœud groupe. */
	function isPart(id: string): boolean {
		const parent = tree.get(id)?.parentId;
		return !!parent && tree.get(parent)?.kind === 'group';
	}

	/**
	 * Menu pièce › Split / Split › To parts sur une pièce d'un objet multi-pièces
	 * (Orca `ObjectList::split` → `ModelVolume::split`) : la pièce est remplacée,
	 * à sa place dans l'objet, par ses composantes `<nom>_1`, `<nom>_2`… —
	 * pièces sœurs du même objet (ici : enfants du même nœud groupe), chacune à
	 * sa position d'origine, même extrudeur et mêmes réglages que la source.
	 * Renvoie les ids créés (vide si rien n'a été séparé).
	 */
	async function splitPartToParts(id: string): Promise<string[]> {
		const object = sceneObjects.find((o) => o.id === id);
		const modelId = objectModelId(id);
		const node = tree.get(id);
		const groupName = node?.parentId ? tree.get(node.parentId)?.name : undefined;
		if (!object || !modelId || !node) {
			saveMessage = 'Séparer en pièces : sélectionnez une pièce dont le maillage est prêt.';
			return [];
		}
		if (meshOpBusy || repairingModels.has(modelId)) {
			saveMessage = 'Séparer en pièces : une opération est déjà en cours sur ce modèle.';
			return [];
		}
		const generation = sceneGeneration;
		meshOpBusy = true;
		repairingModels.add(modelId);
		try {
			const res = await splitModel(modelId);
			const loaded = await Promise.all(
				res.parts.map(async (part) => ({ part, ...(await loadCenteredMesh(part.model.id)) }))
			);
			if (generation !== sceneGeneration || !tree.get(id)) return [];
			if (loaded.length < 2) {
				saveMessage = `Séparer en pièces : « ${node.name} » ne contient qu'une pièce et ne peut pas être séparé.`;
				return [];
			}
			const plateId = plates.plateOf(id) ?? plates.activeId;
			const source = objectTransform(object);
			const c0 = loaded[0].center;
			const created: string[] = [];
			const added: SceneObject[] = [];
			let after = id;
			loaded.forEach(({ part, mesh, center }, i) => {
				projectModelIds.add(part.model.id);
				const name = `${node.name}_${i + 1}`;
				const pieceId = tree.add(name, {
					parentId: node.parentId,
					extruder: node.extruder,
					settings: { ...node.settings },
					printable: node.printable
				}).id;
				tree.moveAfter(pieceId, after);
				after = pieceId;
				if (plateId) plates.assign(pieceId, plateId);
				imports = [
					...imports,
					markConverted(markUploaded(startImport(pieceId, name), part.model.id, false))
				];
				// Position conservée (`translate(offset)` d'Orca) : pas de repose au sol.
				added.push({ id: pieceId, mesh, ...placeSourcePart(source, center) });
				created.push(pieceId);
			});
			// Volumes propres à la pièce (modificateurs…) : suivent la 1re composante.
			const kept = (volumesByObject[id] ?? []).map((v) => ({
				...structuredClone($state.snapshot(v) as SceneVolume),
				offset: [0, 1, 2].map((a) => Math.round((v.offset[a] - c0[a]) * 10000) / 10000) as [
					number,
					number,
					number
				]
			}));
			sceneObjects = [...sceneObjects, ...added];
			if (kept.length) volumesByObject = { ...volumesByObject, [created[0]]: kept };
			removeObject(id);
			ws = setSelection(ws, new Set(created));
			saveMessage = `Séparation en pièces : ${created.length} pièce(s) dans « ${groupName ?? node.name} ».`;
			return created;
		} catch (e) {
			saveMessage =
				e instanceof ApiError
					? meshOpErrorMessage('Séparation en pièces', e.status, e.message)
					: meshOpErrorMessage('Séparation en pièces', null, 'erreur inconnue');
			return [];
		} finally {
			repairingModels.delete(modelId);
			meshOpBusy = false;
		}
	}

	/**
	 * Split appliqué à chaque objet (multi-sélection Orca `multi_selection_menu`),
	 * en une seule étape d'historique. `parts` : les morceaux d'un objet restent
	 * groupés (`split_volume`) ; ceux d'une pièce restent dans son objet parent.
	 */
	async function splitObjects(ids: string[], mode: 'objects' | 'parts') {
		const batch = ids.length > 1;
		if (batch) onGizmoDragChange(true);
		const created: string[] = [];
		let done = 0;
		try {
			for (const id of ids) {
				if (!tree.get(id)) continue;
				if (mode === 'parts' && isPart(id)) {
					// Pièce d'un objet multi-pièces : composantes sœurs dans le même objet.
					const pieces = await splitPartToParts(id);
					if (pieces.length) {
						done += 1;
						created.push(...pieces);
					}
					continue;
				}
				if (mode === 'parts') {
					// Orca `split_volume` sur un objet : volumes maillés dans le même
					// objet (menu objet, multi-sélection et barre d'outils).
					const objectId = await splitToParts(id);
					if (objectId) {
						done += 1;
						created.push(objectId);
					}
					continue;
				}
				await splitObject(id);
				// Échec (une seule pièce, conversion…) : la source est conservée.
				if (tree.get(id)) continue;
				done += 1;
				created.push(...ws.selection);
			}
		} finally {
			if (batch) onGizmoDragChange(false);
		}
		if (created.length) ws = setSelection(ws, new Set(created));
		if (batch && done > 0)
			saveMessage =
				mode === 'parts'
					? `Séparation en pièces : ${done} objet(s) séparé(s) en pièces.`
					: `Séparation : ${done} objet(s) séparé(s) en ${created.length} morceau(x).`;
	}

	/**
	 * Menu pièce › Split › To objects : `Plater::split_object` sur l'objet
	 * sélectionné (`ModelObject::split`) — chaque pièce de l'objet parent est
	 * séparée en ses composantes connexes, toutes deviennent des objets. Une
	 * seule étape d'historique.
	 */
	async function splitPartObject(id: string) {
		const group = tree.get(id)?.parentId;
		if (!group || !isPart(id)) return;
		const children = tree.children(group).map((n) => n.id);
		onGizmoDragChange(true);
		const result: string[] = [];
		try {
			tree.ungroup(group);
			for (const child of children) {
				await splitObject(child);
				// Pièce d'une seule composante : elle devient un objet telle quelle.
				if (tree.get(child)) result.push(child);
				else result.push(...ws.selection);
			}
		} finally {
			onGizmoDragChange(false);
		}
		ws = setSelection(ws, new Set(result));
		saveMessage = `Séparation en objets : ${result.length} objet(s).`;
	}

	/**
	 * « Subdivision mesh (Lost color) » (`ObjectList::smooth_mesh`) : subdivision
	 * de Loop côté worker → nouveau modèle posé sur toutes les instances, posées
	 * sur le plateau (`ensure_on_bed`) ; la peinture est perdue.
	 */
	async function smoothObject(id: string) {
		const modelId = objectModelId(id);
		if (!modelId) {
			saveMessage = 'Subdivision : maillage pas encore prêt.';
			return;
		}
		if (repairingModels.has(modelId)) {
			saveMessage = 'Une opération sur ce maillage est déjà en cours…';
			return;
		}
		const generation = sceneGeneration;
		repairingModels.add(modelId);
		saveMessage = 'Subdivision du maillage en cours…';
		try {
			const res = await smoothModel(modelId);
			const part = res.parts[0];
			const { mesh, center } = await loadCenteredMesh(part.model.id);
			if (generation !== sceneGeneration) return;
			projectModelIds.add(part.model.id);
			const ids = instancesOfModel(imports, modelId);
			sceneObjects = sceneObjects.map((o) => {
				if (!ids.has(o.id)) return o;
				const placed = placeSourcePart(objectTransform(o), center);
				placed.position[2] = restingZ(mesh, placed.rotation, placed.scale);
				return { ...o, mesh, position: placed.position };
			});
			for (const oid of ids) {
				patchImport(oid, (item) => markConverted(markUploaded(item, part.model.id, false)));
				delete paintings[oid];
			}
			paintings = { ...paintings };
			saveMessage = `Subdivision : ${part.triangles} triangles (peinture perdue).`;
		} catch (e) {
			if (generation !== sceneGeneration) return;
			saveMessage =
				e instanceof ApiError
					? meshOpErrorMessage('Subdivision', e.status, e.message)
					: meshOpErrorMessage('Subdivision', null, 'erreur inconnue');
		} finally {
			repairingModels.delete(modelId);
		}
	}

	/**
	 * Outil Coupe complet (phase 28, lot 26 : Slic3r::Cut côté serveur). Plan,
	 * connecteurs et options viennent de cutSession ; chaque pièce devient un
	 * objet, ses volumes additionnels (trous de connecteurs, tenon, 2e pièce de
	 * « Couper en pièces ») des volumes maillés de la scène. Rotation/échelle
	 * de l'objet cuites par le serveur (instance_matrix) → pièces sans
	 * rotation. Les pièces qui se chevauchent sont posées à côté (cellule libre).
	 */
	async function cutObject(id: string | null) {
		const object = id ? sceneObjects.find((o) => o.id === id) : undefined;
		const modelId = id ? objectModelId(id) : null;
		if (!object || !modelId) {
			saveMessage = 'Coupe : sélectionnez un objet dont le maillage est prêt.';
			return;
		}
		if (meshOpBusy || repairingModels.has(modelId)) {
			saveMessage = 'Coupe : une opération est déjà en cours sur ce modèle.';
			return;
		}
		const source = objectTransform(object);
		const body = cutSession.request(() =>
			worldMatrix({ position: [0, 0, 0], rotation: source.rotation, scale: source.scale })
		);
		if (!body) return;
		// Phase 37 : le serveur lit le cutId de l'objet dans la scène enregistrée
		// (update_object_cut_id) et numérote après le plus grand id du projet.
		if (tree.list().some((n) => n.cutId)) {
			const saved = await save({ quiet: true });
			if (saved.status !== 'saved') {
				saveMessage = 'Coupe : enregistrez d’abord le projet (informations de découpe).';
				return;
			}
		}
		body.object_id = object.id;
		const connectorCount = body.connectors?.length ?? 0;
		const generation = sceneGeneration;
		const plateId = plates.plateOf(object.id) ?? plates.activeId;
		meshOpBusy = true;
		repairingModels.add(modelId);
		try {
			const res = await cutModel(modelId, body);
			const loaded = await Promise.all(
				res.parts.map(async (part) => ({
					part,
					...(await loadCenteredMesh(part.model.id)),
					volumes: await Promise.all(
						(part.volumes ?? []).map(async (v) => ({
							v,
							mesh: (await loadCenteredMesh(v.model.id)).mesh
						}))
					)
				}))
			);
			if (generation !== sceneGeneration) return;
			const baked = !!body.instance_matrix;
			const others = objectsOnPlate(sceneObjects, plates.get(plateId ?? '') ?? activePlate)
				.filter((o) => o.id !== object.id)
				.map((o) => objectFootprint(o, [bed.center.x, bed.center.y]));
			const added: SceneObject[] = [];
			const newIds: string[] = [];
			const names: string[] = [];
			for (const { part, mesh, center, volumes } of loaded) {
				projectModelIds.add(part.model.id);
				const objectId = tree.add(part.model.filename).id;
				if (plateId) plates.assign(objectId, plateId);
				// Phase 37 : cut_id recopié (pièces liées) + synchronize_model_after_cut.
				const cut = parseCutId(part.cut_id);
				if (cut) {
					const node = tree.get(objectId);
					if (node) node.cutId = cut;
					for (const other of syncAfterCut(tree.list(), cut, [objectId, object.id])) {
						const n = tree.get(other);
						if (n) n.cutId = { ...cut };
					}
				}
				imports = [
					...imports,
					markConverted(
						markUploaded(startImport(objectId, part.model.filename), part.model.id, false)
					)
				];
				const placed = baked
					? {
							position: [0, 1, 2].map((a) => source.position[a] + center[a]) as [
								number,
								number,
								number
							],
							rotation: [0, 0, 0] as [number, number, number],
							scale: [1, 1, 1] as [number, number, number]
						}
					: placeSourcePart(source, center);
				placed.position[2] = restingZ(mesh, placed.rotation, placed.scale);
				const candidate: SceneObject = { id: objectId, mesh, ...placed };
				const fp = objectFootprint(candidate, [bed.center.x, bed.center.y]);
				const overlaps = others.some(
					(f) =>
						(f.width + fp.width) / 2 - Math.abs(f.x - fp.x) > 0.5 &&
						(f.depth + fp.depth) / 2 - Math.abs(f.y - fp.y) > 0.5
				);
				if (overlaps) {
					const [x, y] = nearestFreeCell(fp, others, {
						x: bed.origin.x,
						y: bed.origin.y,
						width: bed.width,
						depth: bed.depth
					});
					candidate.position = [x, y, placed.position[2]];
					fp.x = x;
					fp.y = y;
				}
				others.push(fp);
				added.push(candidate);
				newIds.push(objectId);
				names.push(cutPartLabel(part.label));
				if (volumes.length > 0) {
					const sceneVolumes: SceneVolume[] = volumes.map(({ v, mesh: vm }) => {
						projectModelIds.add(v.model.id);
						volumeMeshRequests.add(v.model.id);
						volumeMeshes = { ...volumeMeshes, [v.model.id]: vm };
						const b = meshBounds(vm);
						return {
							id: crypto.randomUUID(),
							type: (['part', 'negative', 'modifier', 'blocker', 'enforcer'].includes(v.type)
								? v.type
								: 'part') as SceneVolume['type'],
							name: v.name,
							shape: 'mesh',
							modelId: v.model.id,
							size: [0, 1, 2].map((a) =>
								Math.max(0.001, Math.round((b.max[a] - b.min[a]) * 1000) / 1000)
							) as [number, number, number],
							offset: [...v.offset] as [number, number, number],
							settings: {},
							...(parseCutInfo(v.cut_info) ? { cutInfo: parseCutInfo(v.cut_info)! } : {})
						};
					});
					volumesByObject = { ...volumesByObject, [objectId]: sceneVolumes };
				}
			}
			sceneObjects = [...sceneObjects, ...added];
			removeObject(object.id);
			ws = setSelection(ws, new Set(newIds));
			saveMessage =
				'Coupe : ' +
				newIds.length +
				' objet(s) créé(s) (' +
				names.join(', ') +
				')' +
				(connectorCount ? ', ' + connectorCount + ' connecteur(s)' : '') +
				'.';
			closeTool();
		} catch (e) {
			saveMessage =
				e instanceof ApiError
					? meshOpErrorMessage('Coupe', e.status, e.message)
					: meshOpErrorMessage('Coupe', null, 'erreur inconnue');
		} finally {
			repairingModels.delete(modelId);
			meshOpBusy = false;
		}
	}

	// Opérandes du booléen : deux objets sélectionnés (ordre de sélection).
	const booleanOperands = $derived.by(() => {
		const ids = [...ws.selection];
		if (ids.length !== 2) return null;
		const objects = ids.map((id) => sceneObjects.find((o) => o.id === id));
		const models = ids.map((id) => objectModelId(id));
		if (objects.some((o) => !o) || models.some((m) => !m)) return null;
		return { objects: objects as SceneObject[], models: models as string[] };
	});

	async function booleanSelected(op: BooleanOp) {
		const operands = booleanOperands;
		if (!operands) return;
		const [a, b] = operands.objects;
		const [ma, mb] = operands.models;
		// Union / intersection consomment les deux entrées ; différence garde
		// l'outil B (comportement par défaut du gizmo Orca, « Delete input » off).
		const remove = op === 'difference' ? [a.id] : [a.id, b.id];
		await runMeshOp(
			'Booléen',
			[...new Set([ma, mb])],
			() =>
				booleanModels(data.project.id, {
					op,
					a: { model_id: ma, matrix: worldMatrix(objectTransform(a)) },
					b: { model_id: mb, matrix: worldMatrix(objectTransform(b)) }
				}),
			null,
			remove
		);
	}

	// Retire un objet donné (arbre + scène ; vide la sélection s'il était sélectionné).
	function removeObject(id: string) {
		tree.remove(id);
		plates.unassign(id);
		sceneObjects = sceneObjects.filter((o) => o.id !== id);
		imports = imports.filter((item) => item.objectId !== id);
		rollbackImportIds.delete(id);
		delete paintings[id];
		delete layerProfiles[id];
		delete brimEarsByObject[id];
		delete layerRangesByObject[id];
		delete volumesByObject[id];
		delete savedTransformByObject[id];
		delete savedDocumentByObject[id];
		if (ws.selection.has(id)) {
			const selection = new SvelteSet(ws.selection);
			selection.delete(id);
			ws = setSelection(ws, selection);
		}
	}

	// Ajoute une instance de l'objet sélectionné (clone d'arbre + copie de scène décalée).
	function duplicateSelected() {
		if (selId) duplicateObject(selId);
	}

	// Retire l'objet sélectionné (arbre + scène + sélection vidée).
	function removeSelected() {
		if (selId) removeObject(selId);
	}

	// — Actions du menu Édition (T108) : opèrent sur `sceneObjects`, l'arbre et le
	//   presse-papier. Identifiants d'objets = identifiants de nœuds d'arbre.
	// Ensemble des objets actuellement sélectionnés (liste stable).
	const selectedIds = $derived([...ws.selection]);

	function copySelection() {
		// Instantanés complets (modèle, nom, documents) : un collage reste adossé
		// au modèle serveur et est donc sauvegardé, même après « Couper ».
		clipboard = copyObjects(sceneObjects, ws.selection)
			.map((o) => snapshotObject(o.id))
			.filter((snap): snap is ObjectSnapshot => snap !== null);
	}
	function pasteClipboard() {
		if (clipboard.length === 0) return;
		const pasted: string[] = [];
		for (const snap of clipboard) {
			pasted.push(instantiateSnapshot(snap, pastedPosition(snap.object.position)));
		}
		ws = setSelection(ws, new Set(pasted));
	}
	function deleteSelection() {
		if (ws.selection.size === 0) return;
		for (const id of [...ws.selection]) removeObject(id);
		ws = setSelection(ws, new Set());
	}
	/**
	 * Édition › Delete all (Orca `Plater::delete_all_objects_from_model`) : tous les
	 * objets du modèle, tous plateaux ; menu du plateau › Delete All
	 * (`remove_curr_plate_all`) : ceux du plateau courant. Les ids viennent de
	 * l'arbre / du plateau et non de `sceneObjects` : un objet restauré dont le
	 * maillage est encore en chargement (juste après un rechargement) est supprimé
	 * lui aussi — sinon il réapparaissait à l'arrivée de son maillage.
	 */
	function deleteAllObjects(scope: 'model' | 'plate' = 'model') {
		const ids = new Set(
			scope === 'model'
				? [...tree.list().map((n) => n.id), ...sceneObjects.map((o) => o.id)]
				: [...(activePlate?.objectIds ?? []), ...activeSceneObjects.map((o) => o.id)]
		);
		for (const id of ids) removeObject(id);
		ws = setSelection(ws, new Set());
	}
	function cloneSelection() {
		for (const id of selectedIds) {
			duplicateObject(id);
		}
		// Orca auto_arrange : « Arranger automatiquement après clonage ».
		if (currentPreferences().autoArrange) void arrangeActive();
	}
	function selectAllObjects() {
		ws = setSelection(ws, new Set(activeSceneObjects.map((o) => o.id)));
	}
	function deselectAllObjects() {
		ws = setSelection(ws, new Set());
	}

	// — Raccourcis clavier de l'éditeur (T114) —
	// Cible focalisée éditable ? (champ de saisie → on n'intercepte rien.)
	function isEditableTarget(target: EventTarget | null): boolean {
		const el = target as HTMLElement | null;
		if (!el) return false;
		const tag = el.tagName;
		return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || el.isContentEditable;
	}
	// Route une action de raccourci vers le bon dispatcher (menu / outil / plateau).
	// « A » dispose directement (Orca : plater->arrange()), le bouton de la barre
	// ouvre lui le menu d'options.
	function dispatchShortcut(action: string) {
		if (action === 'arrange') void arrangeActive();
		else if (action === 'orient') void onPlateAction(action);
		else if (isToolAction(action)) selectTool(action as EditorTool);
		else if (!dispatchEditorShortcut(action)) void onMenuAction(action);
	}
	// Raccourcis d'éditeur sans entrée de menu (groupes Plater / Objects List
	// d'Orca). Renvoie `false` si l'action relève du dispatcher de menus.
	function dispatchEditorShortcut(action: string): boolean {
		const extruder = extruderFromAction(action);
		if (extruder !== null) {
			// Orca ignore un numéro au-delà des filaments du projet.
			if (extruder <= filamentSlots) for (const id of selectedIds) tree.setExtruder(id, extruder);
			return true;
		}
		const turn = rotationFromAction(action);
		if (turn !== null) {
			if (layout.tab === 'prepare') rotateSelectionBy(turn);
			return true;
		}
		switch (action) {
			case 'view.plate':
				cameraView = 'plate';
				cameraFocus = null;
				return true;
			case 'object.increaseInstances':
			case 'object.decreaseInstances': {
				const key = action === 'object.increaseInstances' ? '+' : '-';
				if (layout.tab === 'preview') editColorChangeKey(key);
				else changeInstances(key);
				return true;
			}
			case 'plate.orientAll':
				void orientAllOnPlate();
				return true;
			case 'view.switchTab':
				showTab(layout.tab === 'preview' ? 'prepare' : 'preview');
				return true;
			case 'view.zoomIn':
				cameraZoom = clampZoom(cameraZoom * 1.2);
				return true;
			case 'view.zoomOut':
				cameraZoom = clampZoom(cameraZoom / 1.2);
				return true;
			case 'view.toggleSidebar':
				preferences = { ...preferences, sidebarCollapsed: !preferences.sidebarCollapsed };
				savePreferences(preferences);
				return true;
			case 'object.togglePrintable':
				for (const id of selectedIds) tree.setPrintable(id, !tree.isPrintable(id));
				return true;
			case 'slice.plate':
				void sliceActive();
				return true;
			case 'object.renameSelected':
				if (selId) {
					configTab = 'objects';
					renamingObjectId = selId;
				}
				return true;
		}
		return false;
	}
	// Page préc./suiv. (Orca on_key → _do_rotate(±π/4)) : rotation de la
	// sélection autour du Z du plateau ; plusieurs objets tournent ensemble
	// autour du centre de la sélection. Objets verrouillés / hors plateau exclus.
	function rotateSelectionBy(deg: number) {
		const movable = sceneObjects.filter(
			(o) => ws.selection.has(o.id) && canTransformObject(o.id, activePlate, objectLocked)
		);
		if (movable.length === 0) return;
		const poses = rotateSelectionZ(movable, deg);
		sceneObjects = sceneObjects.map((o) => {
			const pose = poses.get(o.id);
			return pose ? { ...o, position: pose.position, rotation: pose.rotation } : o;
		});
	}
	// « + » / « − » en Préparer (Orca Plater::increase/decrease_instances) : un
	// seul objet sélectionné ; « + » ajoute une instance décalée (sélectionnée,
	// sans réarranger), « − » retire la dernière instance du modèle — jamais la
	// seule (can_decrease_instances).
	function changeInstances(key: '+' | '-') {
		if (!selId || !sceneObjects.some((o) => o.id === selId)) {
			if (selectedIds.length > 1)
				saveMessage = 'Instances : sélectionnez un seul objet pour utiliser « + » / « − ».';
			return;
		}
		if (key === '+') {
			if (!tree.isPrintable(selId)) {
				saveMessage = 'Ajouter une instance : l’objet est marqué non imprimable.';
				return;
			}
			duplicateObject(selId);
			return;
		}
		const modelId = imports.find((i) => i.objectId === selId)?.modelId;
		const instances = modelId
			? imports.filter((i) => i.modelId === modelId && tree.get(i.objectId)).map((i) => i.objectId)
			: [selId];
		if (instances.length < 2) {
			saveMessage = 'Retirer une instance : c’est la seule instance de ce modèle.';
			return;
		}
		const last = instances[instances.length - 1];
		removeObject(last);
		const remaining = instances.filter((id) => id !== last);
		ws = setSelection(ws, new Set([remaining[remaining.length - 1]]));
	}
	// « + » / « − » en Aperçu (Orca EVT_GLCANVAS_EDIT_COLOR_CHANGE) : ajoute un
	// changement de couleur à la couche haute du curseur, ou retire le code de
	// cette couche. Stocké dans le plateau, appliqué au prochain tranchage.
	function editColorChangeKey(key: '+' | '-') {
		const plateId = plates.activeId;
		const layers = previewMeta?.layers;
		if (!plateId || !layers?.length) return;
		// G-code ouvert (pas de résultat de tranchage) : rien à retrancher.
		if (!session.gcodeId) {
			saveMessage = 'G-code ouvert : les changements de couleur se règlent sur un plateau tranché.';
			return;
		}
		const z = layers[layerRange.high]?.z;
		if (z === undefined) return;
		const res = editColorChangeAt(
			plates.get(plateId)?.customGcodes ?? [],
			key,
			z,
			String(layerRange.high + 1),
			filamentSlotColors[0] ?? '#00AE42'
		);
		if (res.changed) plates.setCustomGcodes(plateId, res.list);
		saveMessage = res.message;
	}

	// Phase 37 — menu du curseur de couches (Orca IMSlider render_add_menu /
	// render_edit_menu) : couche de la poignée active ; codes du plateau.
	function layerMenuContext(layer: number) {
		const z = previewMeta?.layers[layer]?.z;
		return {
			z,
			items:
				z === undefined
					? []
					: layerCodeMenu({
							codes: activePlate?.customGcodes ?? [],
							z,
							filamentColors: filamentSlotColors.slice(0, filamentSlots),
							templateGcode: String(machineValues.template_custom_gcode ?? ''),
							sequential: sequentialPrint
						}).filter((item) => !item.disabled)
		};
	}
	const layerMenuItems = $derived(
		layerMenu.open ? layerMenuContext(layerMenu.layer).items : ([] as LayerCodeMenuItem[])
	);
	function activeThumbLayer(): number {
		return layerRange.active === 'low' && !layerRange.oneLayerMode
			? layerRange.low
			: layerRange.high;
	}
	function openLayerMenu(x: number, y: number, layer = activeThumbLayer()) {
		if (!session.gcodeId) {
			saveMessage = 'G-code ouvert : les codes par couche se règlent sur un plateau tranché.';
			return;
		}
		layerMenu = { open: true, x, y, layer };
	}
	function onLayerMenuAction(action: string) {
		const index = Number(action.replace('layer.', ''));
		const item = layerMenuItems[index];
		const plateId = plates.activeId;
		const layer = layerMenu.layer;
		const z = previewMeta?.layers[layer]?.z;
		if (!item || !plateId || z === undefined) return;
		const codes = plates.get(plateId)?.customGcodes ?? [];
		if (item.action === 'add-custom' || item.action === 'edit-custom') {
			customGcodeEdit = {
				layer,
				z,
				text: item.action === 'edit-custom' ? (codeAt(codes, z)?.extra ?? '') : '',
				edit: item.action === 'edit-custom'
			};
			return;
		}
		const res = applyLayerCode(codes, item, z, String(layer + 1), {
			filamentColors: filamentSlotColors
		});
		if (res.changed) plates.setCustomGcodes(plateId, res.list);
		saveMessage = res.message;
	}
	function submitCustomGcode() {
		const edit = customGcodeEdit;
		const plateId = plates.activeId;
		if (!edit || !plateId) return;
		const res = applyLayerCode(
			plates.get(plateId)?.customGcodes ?? [],
			{ action: edit.edit ? 'edit-custom' : 'add-custom' },
			edit.z,
			String(edit.layer + 1),
			{ extra: edit.text }
		);
		if (res.changed) plates.setCustomGcodes(plateId, res.list);
		saveMessage = res.message;
		if (res.changed || !edit.text.trim()) customGcodeEdit = null;
	}
	// Flèches : déplace la sélection dans le plan du plateau (objets verrouillés
	// ou hors plateau actif exclus par `moveObject`).
	function nudgeSelection([dx, dy]: [number, number]) {
		for (const id of selectedIds) {
			const obj = sceneObjects.find((o) => o.id === id);
			if (!obj) continue;
			const [x, y, z] = obj.position ?? [0, 0, 0];
			moveObject(id, [x + dx, y + dy, z]);
		}
	}
	// Bouton/lien focalisé : Espace, Entrée et Tab gardent leur rôle natif
	// (activation, navigation au clavier) au lieu de déclencher un raccourci.
	function keepsNativeKey(e: KeyboardEvent): boolean {
		const el = e.target as HTMLElement | null;
		if (!el || !el.closest?.('button, a[href], [role="menuitem"], [role="tab"]')) return false;
		return e.key === ' ' || e.key === 'Enter' || e.key === 'Tab';
	}
	// Raccourcis du groupe Preview (T115) : navigation de couches (flèches / L),
	// bascule Préparer↔Aperçu (Tab). Prioritaires sur les raccourcis d'éditeur
	// quand l'onglet Aperçu est actif. Renvoie `true` si la touche est consommée.
	function handlePreviewKey(e: KeyboardEvent): boolean {
		const action = resolvePreviewKey({ key: e.key, shiftKey: e.shiftKey, ctrlKey: e.ctrlKey });
		if (!action) return false;
		e.preventDefault();
		const step = (fast: boolean) => (fast ? FAST_STEP : 1);
		switch (action.type) {
			case 'thumb':
				moveLayerThumb(action.dir === 'up' ? step(action.fast) : -step(action.fast));
				break;
			case 'toggle-one-layer':
				layerRange = toggleOneLayerMode(layerRange);
				void refreshGeometry();
				break;
			case 'switch-tab':
				showTab(layout.tab === 'preview' ? 'prepare' : 'preview');
				break;
			case 'toggle-gcode-window':
				showGcodeWindow = !showGcodeWindow;
				void ensureGcodeLines();
				break;
			case 'go-to-layer':
				if (previewMeta && previewMeta.layer_count > 1) openGoToLayer();
				break;
			case 'cursor':
				moveCursor = moveCursorBy(
					moveCursor,
					action.dir === 'right' ? step(action.fast) : -step(action.fast)
				);
				void refreshGeometry();
				break;
			case 'cursor-start':
				moveCursor = moveCursorToStart(moveCursor);
				void refreshGeometry();
				break;
			case 'cursor-end':
				moveCursor = moveCursorToEnd(moveCursor);
				void refreshGeometry();
				break;
			// Scrubber horizontal intra-couche (cursor / gcode window) : raffinement T116.
			default:
				break;
		}
		return true;
	}

	// Interaction utilisateur depuis l'ouverture (souris ou touche autre que Tab).
	let editorInteracted = false;
	onMount(() => {
		const mark = (e: Event) => {
			if (e instanceof KeyboardEvent && e.key === 'Tab') return;
			editorInteracted = true;
		};
		window.addEventListener('pointerdown', mark, true);
		window.addEventListener('keydown', mark, true);
		return () => {
			window.removeEventListener('pointerdown', mark, true);
			window.removeEventListener('keydown', mark, true);
		};
	});

	function onEditorKeydown(e: KeyboardEvent) {
		// Laisse les dialogues modaux gérer leurs propres touches.
		if (showShortcuts) return;
		// « + » / « − » sur le curseur de couches focalisé (champ range, sinon
		// ignoré comme éditable) : changement de couleur, comme sur l'IMSlider d'Orca.
		if (
			layout.tab === 'preview' &&
			(e.key === '+' || e.key === '-') &&
			!e.ctrlKey &&
			!e.metaKey &&
			!e.altKey &&
			(e.target as HTMLElement | null)?.closest?.('.preview-vslider')
		) {
			e.preventDefault();
			editColorChangeKey(e.key);
			return;
		}
		if (isEditableTarget(e.target)) return;
		// Tab ne bascule Préparer/Aperçu que depuis la zone 3D (ou sans focus) ;
		// ailleurs il garde la navigation clavier native.
		// Premier Tab après l'ouverture, sans autre interaction : navigation native,
		// pour qu'un utilisateur clavier puisse entrer dans la page (sinon Tab
		// depuis <body> bascule la vue et le focus n'avance jamais).
		const nativeTab =
			e.key === 'Tab' &&
			!e.ctrlKey &&
			!e.altKey &&
			!e.metaKey &&
			(!tabTogglesView(document.activeElement) ||
				(!editorInteracted && document.activeElement === document.body));
		if (nativeTab) return;
		// En Aperçu, les touches de navigation de couches priment sur les outils.
		if (layout.tab === 'preview' && handlePreviewKey(e)) return;
		if (keepsNativeKey(e)) return;
		const chord = {
			key: e.key,
			ctrl: e.ctrlKey,
			shift: e.shiftKey,
			alt: e.altKey,
			meta: e.metaKey
		};
		const cameraNudge =
			layout.tab === 'prepare' && (e.ctrlKey || e.metaKey)
				? cameraSpaceNudge(cameraView, e.key, e.shiftKey)
				: null;
		if (cameraNudge) {
			if (selectedIds.length === 0) return;
			e.preventDefault();
			nudgeSelection(cameraNudge);
			return;
		}
		const nudge = layout.tab === 'prepare' ? resolveNudge(chord, false) : null;
		if (nudge) {
			if (selectedIds.length === 0) return;
			e.preventDefault();
			nudgeSelection(nudge);
			return;
		}
		const action = resolveShortcut(
			{ key: e.key, ctrl: e.ctrlKey, shift: e.shiftKey, alt: e.altKey, meta: e.metaKey },
			isEditableTarget(e.target)
		);
		if (!action) return;
		e.preventDefault();
		dispatchShortcut(action);
	}

	// — Menu contextuel objet (T112) —
	function openContext(id: string, x: number, y: number) {
		contextMenu = { open: true, x, y, targetId: id, part: isPart(id) };
	}
	function closeContext() {
		contextMenu = { ...contextMenu, open: false };
	}
	// Clic droit sur la scène : menu objet si un objet est sélectionné, sinon menu
	// d'ajout (plateau vide) — parité OrcaSlicer.
	function onSceneContext(e: MouseEvent) {
		if (layout.tab !== 'prepare') return;
		e.preventDefault();
		if (selId) openContext(selId, e.clientX, e.clientY);
		else addMenu = { open: true, x: e.clientX, y: e.clientY, scene: true };
	}
	function closeAddMenu() {
		addMenu = { ...addMenu, open: false };
	}
	// Ajoute une primitive paramétrique au plateau actif, centrée, et la sélectionne.
	// La primitive est générée côté serveur par libslic3r (`POST …/primitive`) et
	// devient un modèle du projet : elle rejoint le pipeline des imports (restaurée
	// à l'ouverture via `loadProjectModels`, incluse dans le tranchage). Aucun
	// maillage TS n'entre dans la sortie (native-parity).
	async function addPrimitive(kind: PrimitiveKind) {
		// Comme `importOne` : la restauration initiale de la scène remplace l'arbre,
		// les objets et la sélection — une primitive ajoutée avant sa fin perdait
		// sa sélection (puis réapparaissait comme modèle orphelin non sélectionné).
		await editorReady;
		const generation = sceneGeneration;
		const name = kind.charAt(0).toUpperCase() + kind.slice(1);
		const id = tree.add(name).id;
		rollbackImportIds.add(id);
		if (plates.activeId) plates.assign(id, plates.activeId);
		imports = [...imports, startImport(id, `${name}.stl`)];
		// Sélection immédiate (comme avant la génération moteur) : une sélection
		// posée après l'aller-retour moteur écraserait un Échap/clic intervenu entre-temps.
		ws = setSelection(ws, new Set([id]));
		try {
			// Géométrie d'Orca, pas une copie TS : libslic3r `its_make_*` avec les
			// paramètres de `ObjectList::create_mesh` (côté = 0,1 × plus grande
			// dimension du plateau, pas angulaire 2°/3°), via engine-worker. Le
			// modèle serveur créé est la source du tranchage et du 3MF.
			const res = await createPrimitive(data.project.id, {
				shape: kind,
				bed_size_mm: [bed.width, bed.depth]
			});
			if (generation !== sceneGeneration) return;
			const part = res.parts[0];
			projectModelIds.add(part.model.id);
			// Maillage recentré comme ceux restaurés depuis le serveur : pose Z =
			// demi-hauteur, identique avant et après rechargement.
			const { mesh } = await loadCenteredMesh(part.model.id);
			if (generation !== sceneGeneration) return;
			sceneObjects = [
				...sceneObjects,
				{ id, mesh, position: [bed.center.x, bed.center.y, restingZ(mesh)] }
			];
			patchImport(id, (i) => markConverted(markUploaded(i, part.model.id, false)));
			rollbackImportIds.delete(id);
		} catch (e) {
			if (generation !== sceneGeneration) return;
			failImport(id, e instanceof ApiError ? e.message : 'échec de l’upload');
		}
	}
	// Dispatch du menu d'ajout (T113) : Load (import) ou primitive.
	function onAddAction(action: string) {
		const handyKey = handyKeyOf(action);
		if (handyKey) {
			void addHandyModelToScene(handyKey);
			return;
		}
		if (action === 'add.load') {
			fileInput?.click();
			return;
		}
		if (action === 'view.labels') {
			showLabels = !showLabels;
			return;
		}
		const kind = action.replace('add.', '') as PrimitiveKind;
		void addPrimitive(kind);
	}
	// Déplace un objet de la scène (position absolue, mm).
	function moveObject(id: string, position: [number, number, number]) {
		if (!canTransformObject(id, activePlate, objectLocked)) return;
		sceneObjects = sceneObjects.map((o) => (o.id === id ? { ...o, position } : o));
	}
	// Remplit le plateau d'instances du même modèle, sur une grille sans collision.
	// Les copies web partagent le modèle serveur (comme les instances Orca) tout en
	// conservant une transformation et des réglages propres.
	// « Remplir le plateau avec des instances » d'Orca (FillBedJob) : copies
	// placées par l'arrangement libslic3r (POST …/arrange, fill_object_id)
	// autour des autres objets du plateau, qui restent fixes.
	async function fillBedWithInstances(id: string) {
		const object = sceneObjects.find((o) => o.id === id);
		if (!object) return;
		const others = movableObjectsOnPlate(sceneObjects, activePlate, objectLocked);
		const items = arrangeItems([object, ...others.filter((o) => o.id !== id)]);
		let placements: Awaited<ReturnType<typeof arrangeScene>>['placements'];
		try {
			const res = await arrangeScene(data.project.id, {
				bed_width: bed.width,
				bed_depth: bed.depth,
				spacing: 0,
				items,
				bed_shape: bed.polygon.map((point) => [point.x, point.y]),
				excluded: ((pts) =>
					pts.length >= 3 ? [pts.map((p) => [p.x, p.y] as [number, number])] : [])(
					parseAreaPoints(machineValues.bed_exclude_area)
				),
				max_height: bed.height,
				fill_object_id: id
			});
			placements = res.placements;
		} catch (e) {
			saveMessage = `Remplir le plateau : ${e instanceof ApiError ? e.message : 'échec du moteur'}`;
			return;
		}
		const added = placements.filter((p) => p.id !== id);
		if (added.length === 0) {
			saveMessage = 'Remplir le plateau : aucune instance supplémentaire ne tient sur le plateau.';
			return;
		}
		const snap = snapshotObject(id);
		if (!snap) return;
		const z = object.position?.[2] ?? restingZ(object.mesh, object.rotation, object.scale);
		const own = placements.find((p) => p.id === id);
		if (own)
			sceneObjects = sceneObjects.map((o) =>
				o.id === id ? { ...o, position: [own.x, own.y, z] as [number, number, number] } : o
			);
		for (const p of added) instantiateSnapshot(snap, [p.x, p.y, z]);
		ws = setSelection(ws, new Set([id]));
		saveMessage = `Remplissage du plateau : ${added.length + 1} instances.`;
	}

	// Détache une instance : nouveau modèle serveur, transformations conservées.
	async function makeIndividualObject(id: string) {
		const object = sceneObjects.find((o) => o.id === id);
		if (!object) return;
		try {
			const name = tree.get(id)?.name ?? 'Objet';
			const model = await uploadModel(data.project.id, sceneMeshToStlFile(object.mesh, name));
			projectModelIds.add(model.id);
			patchImport(id, (item) => markConverted(markUploaded(item, model.id, false)));
			saveMessage = `${name} est maintenant un objet individuel.`;
		} catch (e) {
			saveMessage = `Objet individuel : ${e instanceof ApiError ? e.message : 'échec de l’upload'}`;
		}
	}

	async function reloadObject(id: string) {
		const modelId = objectModelId(id);
		if (!modelId) return;
		meshRequests.delete(modelId);
		const warning = await reloadServerMesh(modelId, sceneGeneration, 'Maillage rechargé');
		if (warning !== null) saveMessage = `Maillage rechargé depuis le disque.${warning}`;
	}

	function chooseReplacement(id: string, all: boolean) {
		replacementTargetIds = all && ws.selection.has(id) ? [...selectedIds] : [id];
		if (replacementInput) {
			replacementInput.multiple = all;
			replacementInput.click();
		}
	}

	async function onReplacementPicked(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		const files = [...(input.files ?? [])];
		const targets = [...replacementTargetIds];
		input.value = '';
		for (let i = 0; i < Math.min(files.length, targets.length); i++) {
			const id = targets[i];
			if (!sceneObjects.some((o) => o.id === id)) continue;
			try {
				const model = await uploadModel(data.project.id, files[i]);
				// Conversion moteur asynchrone : `/mesh` répond 409 tant qu'elle tourne.
				let loaded: { mesh: SceneMesh } | null = null;
				for (let attempt = 0; !loaded; attempt++) {
					try {
						loaded = await loadCenteredMesh(model.id);
					} catch (err) {
						if (!(err instanceof ApiError && err.status === 409) || attempt >= 120) throw err;
						await new Promise((resolve) => setTimeout(resolve, 500));
					}
				}
				const { mesh } = loaded;
				projectModelIds.add(model.id);
				sceneObjects = sceneObjects.map((o) => (o.id === id ? { ...o, mesh } : o));
				patchImport(id, (item) => markConverted(markUploaded(item, model.id, false)));
			} catch (err) {
				saveMessage = `Remplacement : ${err instanceof ApiError ? err.message : 'maillage illisible'}`;
				return;
			}
		}
		saveMessage = `Remplacement : ${Math.min(files.length, targets.length)} maillage(s) remplacé(s), transformations conservées.`;
	}

	// Dispatch des actions du menu contextuel sur l'objet ciblé.
	async function onObjectAction(action: string) {
		const id = contextMenu.targetId;
		if (!id) return;
		const obj = sceneObjects.find((o) => o.id === id);
		const unitAction = unitActionOf(action);
		if (unitAction) {
			for (const target of ws.selection.has(id) ? selectedIds : [id])
				convertObjectUnits(target, unitAction.unit, unitAction.restore);
			return;
		}
		if (action === 'object.editEmboss') {
			ws = setSelection(ws, new Set([id]));
			openEmbossEdit(id);
			return;
		}
		// Phase 37 : type de pièce, tableau des paramètres, informations de coupe.
		if (action.startsWith('volume.type:')) {
			if (selectedVolumeId)
				setVolumeType(id, selectedVolumeId, action.slice('volume.type:'.length) as VolumeType);
			return;
		}
		if (action === 'object.parameterTable') {
			openObjectTable(id);
			return;
		}
		if (action === 'object.selectCutParts') {
			selectCutParts(id);
			return;
		}
		if (action === 'object.invalidateCut') {
			await invalidateCutInfo(id);
			return;
		}
		switch (action) {
			case 'object.hideShow':
				tree.setHidden(id, !tree.isHidden(id));
				break;
			case 'object.rename':
				configTab = 'objects';
				renamingObjectId = id;
				break;
			case 'object.edit':
				ws = setSelection(ws, new Set([id]));
				tools = { active: 'move' };
				break;
			case 'object.addSettings':
				ws = setSelection(ws, new Set([id]));
				selectedVolumeId = null;
				configTab = 'objects';
				objectSettingsAdding = true;
				break;
			case 'object.clone':
			case 'object.addInstance':
				duplicateObject(id);
				if (action === 'object.clone' && currentPreferences().autoArrange) void arrangeActive();
				break;
			case 'object.removeInstance': {
				// Instance = objet partageant le même modèle serveur. Comme Orca
				// (entrée désactivée à 1 instance), la dernière n'est pas supprimée.
				const modelId = imports.find((i) => i.objectId === id)?.modelId;
				const siblings = modelId
					? imports.filter(
							(i) => i.modelId === modelId && i.objectId !== id && tree.get(i.objectId)
						)
					: [];
				if (siblings.length === 0) {
					saveMessage = 'Retirer une instance : c’est la seule instance de ce modèle.';
					break;
				}
				removeObject(id);
				break;
			}
			case 'object.delete':
				removeObject(id);
				break;
			case 'object.individual':
				await makeIndividualObject(id);
				break;
			case 'object.fillBed':
				fillBedWithInstances(id);
				break;
			case 'object.orient':
				ws = setSelection(ws, new Set([id]));
				await orientActive();
				break;
			case 'object.center':
				if (obj) moveObject(id, [bed.center.x, bed.center.y, obj.position?.[2] ?? 0]);
				break;
			case 'object.drop':
				// Maillages centrés : poser = amener le point le plus bas (rotation et
				// échelle comprises) à z = 0, et non placer le centre à z = 0.
				if (obj)
					moveObject(id, [
						obj.position?.[0] ?? 0,
						obj.position?.[1] ?? 0,
						restingZ(obj.mesh, obj.rotation, obj.scale)
					]);
				break;
			case 'object.simplify':
				ws = setSelection(ws, new Set([id]));
				tools = { active: 'simplify' };
				break;
			case 'object.meshBoolean':
				ws = setSelection(ws, new Set([id]));
				tools = { active: 'boolean' };
				break;
			case 'object.mirrorX':
			case 'object.mirrorY':
			case 'object.mirrorZ':
				if (obj) {
					const axis = action === 'object.mirrorX' ? 0 : action === 'object.mirrorY' ? 1 : 2;
					onTransform(id, { ...objectTransform(obj), scale: mirrorScale(obj.scale, axis) });
				}
				break;
			case 'object.scaleToBed':
				if (obj) {
					const factor = fitScaleFactor(worldSize(obj.mesh, obj.rotation, obj.scale), bed);
					if (factor === null) break;
					const scale = scaleBy(obj.scale, factor);
					// Agrandi/réduit autour de son centre, puis reposé sur le plateau.
					const [x, y] = obj.position ?? [bed.center.x, bed.center.y, 0];
					onTransform(id, {
						...objectTransform(obj),
						position: [x, y, restingZ(obj.mesh, obj.rotation, scale)],
						scale
					});
				}
				break;
			case 'object.exportStl':
				await exportStl(false, ws.selection.has(id) ? selectedIds : [id]);
				break;
			case 'object.exportStls':
				await exportStl(true, ws.selection.has(id) ? selectedIds : [id]);
				break;
			case 'object.exportDrc':
			case 'object.exportDrcs':
				await exportDrc(action === 'object.exportDrcs', ws.selection.has(id) ? selectedIds : [id]);
				break;
			case 'object.setInstances': {
				const input = window.prompt('Nombre d’instances', '2');
				const count = input === null ? NaN : Math.floor(Number(input));
				if (!Number.isFinite(count) || count < 1 || count > 100) break;
				// Pas d'instances partagées côté scène : N − 1 copies de l'objet.
				for (let i = 1; i < count; i++) duplicateObject(id);
				break;
			}
			case 'object.addHeightRange':
				addLayerRange(id);
				break;
			case 'object.addPart':
			case 'object.addNegativeVolume':
			case 'object.addModifier':
			case 'object.addSupportBlocker':
			case 'object.addSupportEnforcer':
				addVolume(id, action);
				break;
			case 'object.fix':
				await repairObject(id);
				break;
			case 'object.reload':
				await reloadObject(id);
				break;
			case 'object.replace':
				chooseReplacement(id, false);
				break;
			case 'object.replaceAll':
				chooseReplacement(id, true);
				break;
			case 'object.assemble': {
				const ids = ws.selection.has(id) ? selectedIds : [id];
				if (ids.length > 1 && tree.group(ids, 'Assemblage'))
					saveMessage = `Assemblage : ${ids.length} objets regroupés en pièces.`;
				break;
			}
			case 'object.assembleSingle':
				if (ws.selection.has(id) && selectedIds.length === 2) await booleanSelected('union');
				else saveMessage = 'Assemblage en une pièce : sélectionnez exactement deux objets.';
				break;
			case 'object.toObjects':
			case 'object.split':
				// Multi-sélection : split appliqué à chaque objet sélectionné.
				await splitObjects(ws.selection.has(id) ? selectedIds : [id], 'objects');
				break;
			case 'object.toParts':
				// Orca `split_volume` : les morceaux restent les pièces d'un même objet
				// (ici : regroupés sous un nœud portant le nom de la source).
				await splitObjects(ws.selection.has(id) ? selectedIds : [id], 'parts');
				break;
			case 'part.split':
			case 'part.toParts':
				await splitObjects([id], 'parts');
				break;
			case 'part.toObjects':
				await splitPartObject(id);
				break;
			case 'object.smooth':
				await smoothObject(id);
				break;
		}
	}
	// — Menus Fichier/Vue/Application (parité Orca côté web) —
	// Téléchargement d'un contenu généré côté client.
	function downloadBlob(
		data: Uint8Array | Blob,
		filename: string,
		type = 'application/octet-stream'
	) {
		const blob = data instanceof Blob ? data : new Blob([data as BlobPart], { type });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = filename;
		document.body.appendChild(a);
		a.click();
		a.remove();
		setTimeout(() => URL.revokeObjectURL(url), 1000);
	}

	// Export DRC (Draco, `store_drc` d'Orca côté serveur) : la scène est
	// d'abord enregistrée (le serveur lit la scène persistée), puis un `.drc`
	// fusionné ou un ZIP d'un `.drc` par objet est téléchargé. `ids` = sélection
	// (menu objet), `null` = tous les objets (menu Fichier).
	async function exportDrc(split: boolean, ids: string[] | null) {
		await exportMeshes('drc', split, ids);
	}

	// Export STL / DRC côté serveur (POST /export/{stl,drc}), après sauvegarde.
	async function exportMeshes(format: 'stl' | 'drc', split: boolean, ids: string[] | null) {
		const label = format.toUpperCase();
		const saved = await save();
		if (saved.status !== 'saved') {
			saveMessage = `Export ${label} : ${saved.status === 'error' ? saved.message : 'sauvegarde impossible'}`;
			return;
		}
		try {
			const res = await fetch(`/api/projects/${data.project.id}/export/${format}`, {
				method: 'POST',
				credentials: 'include',
				headers: { 'Content-Type': 'application/json' },
				// Phase 37 : quantification Draco (préférence drc_bits d'Orca).
				body: JSON.stringify({
					split,
					object_ids: ids,
					...(format === 'drc' ? { bits: $appPreferences.dracoBits } : {})
				})
			});
			if (!res.ok) {
				const err = (await res.json().catch(() => ({}))) as { message?: string };
				saveMessage = `Export ${label} : ${err.message ?? res.statusText}`;
				return;
			}
			const name =
				/filename="([^"]+)"/.exec(res.headers.get('content-disposition') ?? '')?.[1] ??
				safeFileName(projectName || data.project.name, split ? 'zip' : format);
			downloadBlob(await res.blob(), name);
			saveMessage = `Export ${label} : ${split ? 'archive (un fichier par objet) exportée' : 'fichier fusionné exporté'} — « ${name} ».`;
		} catch {
			saveMessage = `Export ${label} : échec du téléchargement.`;
		}
	}

	// Export STL (menu Fichier « Export all objects as one STL / as STLs », menu
	// objet « Export as one STL / as STLs ») : maillages calculés côté serveur par
	// libslic3r comme Plater::export_stl d'Orca (volumes négatifs soustraits,
	// toutes les instances), STL binaire fusionné ou ZIP d'un STL par objet.
	async function exportStl(split: boolean, ids: string[] | null) {
		await exportMeshes('stl', split, ids);
	}

	// Export plate sliced file : G-code du dernier tranchage (plateau actif).
	function exportPlateGcode() {
		if (!session.gcodeId) {
			saveMessage = 'Export du plateau : tranchez d’abord le plateau.';
			return;
		}
		downloadUrl(`/api/gcodes/${session.gcodeId}/download`);
	}

	// Tranche tous les plateaux (« Slice all ») après sauvegarde et attend les
	// jobs. Renvoie les jobs réussis (ordre des plateaux) et le nombre d'échecs.
	// Répartition des filaments par buse avant le tranchage (Orca
	// try_pop_up_before_slice) : deux buses Bambu Lab + préférence active.
	// Résout `true` pour trancher (choix écrit sur le(s) plateau(x)), `false` = annulé.
	let filamentMapPrompt = $state<{
		allPlates: boolean;
		mode: FilamentMapMode;
		map: number[];
		filaments: number[];
		resolve: (ok: boolean) => void;
	} | null>(null);
	function askFilamentMap(allPlates: boolean): Promise<boolean> {
		const extruders = extruderCount(machineValues.nozzle_diameter);
		const printerName = machinePresets.find((m) => m.id === activePresets.printer)?.name;
		if (
			!shouldPromptFilamentMap({
				enabled: $appPreferences.filamentMapPopup,
				extruders,
				bambu: isBambuMachine(machineValues, printerName)
			})
		)
			return Promise.resolve(true);
		const settings = plates.list().find((p) => p.id === plates.activeId)?.settings ?? {};
		const count = filamentSlotColors.length;
		const used: number[] = [];
		if (!allPlates)
			for (const o of activeSceneObjects) {
				if (!tree.isPrintable(o.id)) continue;
				used.push(clampExtruder(tree.effectiveExtruder(o.id), Math.max(1, count)) || 1);
				for (const v of volumesByObject[o.id] ?? [])
					if (v.type === 'part' && v.extruder) used.push(v.extruder);
			}
		return new Promise((resolve) => {
			filamentMapPrompt = {
				allPlates,
				mode: effectiveMapMode(
					settings.filament_map_mode,
					processValues.filament_map_mode ?? machineValues.filament_map_mode
				),
				map: readFilamentMap(settings.filament_map, count, extruders),
				filaments: listedFilaments(used, count),
				resolve
			};
		});
	}
	function closeFilamentMap(choice: { mode: FilamentMapMode; map: number[] } | null) {
		const prompt = filamentMapPrompt;
		filamentMapPrompt = null;
		if (!prompt) return;
		if (choice) {
			const ids = prompt.allPlates
				? plates.list().map((p) => p.id)
				: plates.activeId
					? [plates.activeId]
					: [];
			for (const id of ids)
				for (const [key, value] of filamentMapChoice(choice.mode, choice.map))
					plates.setPlateSetting(id, key, value);
			// Choix explicite de l'utilisateur : enregistré comme un réglage de plateau
			// (le tranchage, lui, part de la scène affichée sans sauvegarder).
			void saveNow({ quiet: true });
		}
		prompt.resolve(choice !== null);
	}

	async function sliceAllPlatesJobs(
		label: string,
		progress: (message: string) => void
	): Promise<{ succeeded: JobResponse[]; missed: number; firstError: string | null }> {
		await editorReady;
		if (!(await askFilamentMap(true))) throw new Error('tranchage annulé');
		const res = await requestSlice(sliceRequestWithDraft('all', 0));
		progress(`${label} : tranchage de ${res.jobs.length} plateau(x)…`);
		const deadline = Date.now() + 10 * 60_000;
		// Table locale (non réactive) des jobs terminés.
		const done: Record<string, JobResponse> = {};
		const finished = () => Object.keys(done).length;
		while (finished() < res.jobs.length && Date.now() < deadline) {
			for (const job of res.jobs) {
				if (done[job.id]) continue;
				const current = await getJob(job.id);
				if (['succeeded', 'failed', 'cancelled'].includes(current.status)) done[job.id] = current;
			}
			if (finished() < res.jobs.length) {
				progress(`${label} : ${finished()} / ${res.jobs.length} plateau(x) tranché(s)…`);
				await new Promise((r) => setTimeout(r, 1000));
			}
		}
		const all = Object.values(done);
		const succeeded = all
			.filter((j) => j.status === 'succeeded' && j.gcode_id)
			.sort((a, b) => Number(a.plate_index) - Number(b.plate_index));
		const failed = all.find((j) => j.status === 'failed');
		const firstError = failed
			? friendlySliceError(
					typeof failed.error === 'string'
						? failed.error
						: ((failed.error as { message?: string } | null)?.message ?? 'échec')
				)
			: null;
		return { succeeded, missed: res.jobs.length - succeeded.length, firstError };
	}

	// Export all plate sliced file (Orca `Plater::export_gcode_3mf`, tous les
	// plateaux) : tranche tous les plateaux puis télécharge le .gcode.3mf écrit
	// par libslic3r (POST /export/sliced, un Metadata/plate_<n>.gcode par plateau).
	let exportingAll = $state(false);
	async function exportAllPlatesGcode() {
		if (exportingAll) return;
		exportingAll = true;
		const label = 'Export des plateaux';
		try {
			const { succeeded, missed, firstError } = await sliceAllPlatesJobs(
				label,
				(m) => (saveMessage = m)
			);
			if (succeeded.length === 0) {
				saveMessage = `${label} : aucun plateau n’a pu être tranché${firstError ? ` — ${firstError}` : '.'}`;
				return;
			}
			const resp = await fetch(`/api/projects/${data.project.id}/export/sliced`, {
				method: 'POST',
				credentials: 'include',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ job_ids: succeeded.map((j) => j.id) })
			});
			if (!resp.ok) {
				const err = (await resp.json().catch(() => ({}))) as { message?: string };
				throw new ApiError(resp.status, 'error', err.message ?? resp.statusText);
			}
			const name =
				/filename="([^"]+)"/.exec(resp.headers.get('content-disposition') ?? '')?.[1] ??
				`${safeFileName(projectName || data.project.name, 'gcode')}.3mf`;
			downloadBlob(await resp.blob(), name);
			const nPlates = (n: number) => `${n} plateau${n > 1 ? 'x' : ''}`;
			saveMessage =
				`${label} : ${nPlates(succeeded.length)} dans « ${name} ».` +
				(missed > 0 ? ` ${nPlates(missed)} en échec${firstError ? ` — ${firstError}` : '.'}` : '');
		} catch (e) {
			saveMessage = `${label} : ${e instanceof Error ? e.message : 'échec'}`;
		} finally {
			exportingAll = false;
		}
	}

	// Send all / Print all (Orca `send_gcode_legacy(PLATE_ALL_IDX)`) : tranche
	// tous les plateaux puis les envoie (POST /printers/{id}/upload-sliced).
	async function sendAllPlates(
		printerId: string,
		startNow: boolean,
		progress: (message: string) => void,
		queue = false
	) {
		const { succeeded, missed, firstError } = await sliceAllPlatesJobs('Envoi', progress);
		if (succeeded.length === 0)
			throw new Error(`aucun plateau n’a pu être tranché${firstError ? ` — ${firstError}` : '.'}`);
		if (missed > 0)
			throw new Error(
				`${missed} plateau(x) en échec, rien n'a été envoyé${firstError ? ` — ${firstError}` : '.'}`
			);
		progress('Envoi : transfert vers l’imprimante…');
		return uploadSliced(printerId, {
			project_id: data.project.id,
			job_ids: succeeded.map((j) => j.id),
			start_now: startNow,
			...(queue ? { queue: true } : {})
		});
	}

	// Open G-code… : aperçu d'un fichier local, parsé par le backend (même
	// parseur que les tranchages), sans stockage ni tranchage.
	let gcodeFileInput: HTMLInputElement | undefined = $state();
	let loadedGcodeFile: File | null = null;
	// Temps écrit dans le pied du G-code ouvert (comparé à la simulation).
	let openedDeclaredTime = $state<string | null>(null);
	async function openLocalGcode(file: File) {
		try {
			// Lot 31 : le fichier est aussi simulé par GCodeProcessor (comme la
			// visionneuse d'Orca) → statistiques réelles, méta moteur, mouvements
			// et coutures par jeton. Repli : pied de G-code relu côté client.
			const [meta, processed] = await Promise.all([
				localGcodeMeta(file),
				processLocalGcode(file).catch(() => null)
			]);
			loadedGcodeFile = file;
			previewMeta = meta;
			loader = new LazyPreviewLoader({
				layerCount: meta.layer_count,
				fetchRange: (from, to) => localGcodeLayers(file, from, to)
			});
			engineSource = processed ? openedEngineSource(processed.token, processed.meta) : null;
			openedDeclaredTime = processed?.footer?.estimated_time_text ?? null;
			rawStats = processed
				? statsFromProcessed(processed)
				: parseGcodeFooterStats(await file.text());
			setGcodeSource(() => file.text());
			layerRange = makeLayerRange(meta.layer_count);
			session = { ...prepareSession(), phase: 'preview', progress: 1 };
			previewLoadedFor = null;
			layout = setTab(layout, 'preview');
			await refreshGeometry();
			saveMessage = `G-code « ${file.name} » ouvert (${meta.layer_count} couches).`;
		} catch (e) {
			saveMessage = `Ouverture du G-code : ${e instanceof ApiError ? e.message : 'échec'}`;
		}
	}

	// Préférences (thème, langue, unités, avertissement avant fermeture).
	let showPreferences = $state(false);
	let showPresetBundle = $state(false);
	const presetBundleItems = $derived.by(() => {
		const items: BundlePresetRef[] = [];
		const machine = activePresetOf('machine');
		const process = activePresetOf('process');
		if (machine) items.push({ id: machine.id, kind: 'machine', name: machine.name });
		if (process) items.push({ id: process.id, kind: 'process', name: process.name });
		for (const id of activePresets.filaments) {
			const filament = filamentPresets.find((p) => p.id === id);
			if (filament) items.push({ id, kind: 'filament', name: filament.name });
		}
		return items;
	});
	let preferences = $state<AppPreferences>(DEFAULT_PREFERENCES);
	onMount(() => {
		preferences = loadPreferences();
		// Rapport d'import 3MF en attente (ouverture « comme projet ») : boîte
		// « G-code modifié / Presets personnalisés » + renvoi vers l'onglet Projet.
		importReport = takeImportReport(data.project.id);
		// Préférences modifiées ailleurs (dialogue des purges…) : même état.
		const unsubscribe = appPreferences.subscribe((p) => (preferences = p));
		const warn = (event: BeforeUnloadEvent) => {
			if (preferences.confirmClose && unsavedNow()) event.preventDefault();
		};
		window.addEventListener('beforeunload', warn);
		return () => {
			unsubscribe();
			window.removeEventListener('beforeunload', warn);
		};
	});

	// Projection de la vue 3D (menu Vue).
	let projection = $state<Projection>('perspective');

	// Menu Vue › Show Labels / Show Overhang (scène de préparation).
	let showLabels = $state(false);
	let showOverhang = $state(false);
	const overhangNormalZ = $derived(
		showOverhang ? supportNormalZ({ ...machineValues, ...processValues }) : null
	);
	const objectLabels = $derived(
		showLabels
			? Object.fromEntries(visibleSceneObjects.map((o) => [o.id, tree.get(o.id)?.name ?? o.id]))
			: null
	);

	// Menu Vue › Show G-code Window : texte du G-code affiché (tranchage ou
	// fichier local), chargé à la demande.
	let showGcodeWindow = $state(false);
	let gcodeSource: (() => Promise<string>) | null = null;
	let gcodeLines = $state<string[]>([]);
	let gcodeLinesFor: (() => Promise<string>) | null = null;
	async function ensureGcodeLines() {
		const source = gcodeSource;
		if (!showGcodeWindow || !source || source === gcodeLinesFor) return;
		gcodeLinesFor = source;
		try {
			gcodeLines = (await source()).split(/\r?\n/);
		} catch {
			gcodeLinesFor = null;
			saveMessage = 'Fenêtre G-code : lecture du G-code impossible.';
		}
	}
	function setGcodeSource(source: () => Promise<string>) {
		gcodeSource = source;
		gcodeLines = [];
		gcodeLinesFor = null;
		previewFrame = null;
		previewFit = 'objects';
		void loadPreviewTrack(source);
		void ensureGcodeLines();
	}

	// Lit le G-code complet et construit la piste d'aperçu (une fois par source).
	async function loadPreviewTrack(source: () => Promise<string>) {
		previewTrack = null;
		previewTrackLoading = true;
		try {
			let track = parsePreviewTrack(await source());
			const textTrack = track;
			if (gcodeSource !== source) return;
			engineMeta = null;
			engineBase = null;
			engineCache = null;
			engineFullMoves = null;
			engineSeams = null;
			const src = engineSource;
			if (src) {
				try {
					const meta = await src.meta();
					if (gcodeSource !== source) return;
					// Coutures du moteur (lot 31, révision 3) ; repli : coutures du texte.
					if (meta.seam_count !== undefined) engineSeams = await src.seams().catch(() => null);
					if (gcodeSource !== source) return;
					const windowed = meta.ranges && meta.layers.every((l) => l.first_line !== undefined);
					if (windowed) {
						// Gros G-code : mouvements par fenêtre de couches (méta v2).
						engineCache = new EngineMoveCache(meta.layer_count, (from, to) => src.moves(from, to));
						const [from, to] = engineLayersFor(track, meta, layerRange.low, layerRange.high);
						const chunks = ENGINE_SEGMENT_COLORATIONS.has(coloration)
							? await engineCache.ensure(from, to)
							: [];
						track = decorateEngine(applyEngine(track, meta, chunks, engineTimeMode), meta, chunks);
					} else {
						engineFullMoves = await src.moves();
						track = decorateEngine(withEngine(track, engineFullMoves, engineTimeMode), meta, [
							engineFullMoves
						]);
					}
					if (gcodeSource !== source) return;
					engineBase = textTrack;
					engineMeta = meta;
				} catch {
					// Moteur indisponible : piste texte (temps nominal) en repli.
					engineCache = null;
					engineFullMoves = null;
				}
			}
			// Le modèle WSPv peut arriver après le texte : l'alignement est vérifié
			// au rendu (refreshGeometry) et ici quand la méta est déjà connue.
			previewTrack = !previewMeta || trackMatches(track, previewMeta.layer_count) ? track : null;
			void refreshGeometry();
		} catch {
			if (gcodeSource === source) previewTrack = null;
		} finally {
			if (gcodeSource === source) previewTrackLoading = false;
		}
	}

	// Export toolpaths as OBJ : trajets d'extrusion de tout l'aperçu courant.
	async function exportToolpathsObj() {
		if (!loader || !previewMeta || session.phase !== 'preview') {
			saveMessage = 'Export des trajets : tranchez ou ouvrez d’abord un G-code.';
			return;
		}
		const last = Math.max(0, previewMeta.layer_count - 1);
		await loader.ensureRange(0, last);
		const base = projectName || data.project.name;
		const text = toolpathsToObj(loader.rangeSegments(0, last), base);
		const file = safeFileName(`${base}_trajets`, 'obj');
		downloadBlob(new TextEncoder().encode(text), file, 'model/obj');
		saveMessage = `Trajets exportés en OBJ — « ${file} ».`;
	}

	// — Ouverture d'un 3MF (lot 34) : « Ouvrir comme projet / Importer la
	// géométrie seule » selon la préférence project_load_behaviour d'Orca
	// (determine_load_type) ; le rapport d'import (avertissement G-code,
	// métadonnées) passe par sessionStorage à la page du projet créé. —
	let open3mf = $state<File | null>(null);
	let importReport = $state<ImportReport | null>(null);

	async function openProjectFile(file: File) {
		let load: ImportLoadMode | undefined;
		if (isProjectArchive(file.name)) {
			const decision = decideLoadMode(
				currentPreferences().projectLoadBehaviour,
				sceneObjects.length > 0
			);
			if (decision === 'ask') {
				open3mf = file;
				return;
			}
			load = decision;
		}
		await openProjectWithLoad(file, load);
	}

	async function openProjectWithLoad(file: File, load?: ImportLoadMode) {
		open3mf = null;
		saveMessage = `Ouverture du projet « ${file.name} »…`;
		try {
			const project = await importProject(file, load);
			if (project.import) stashImportReport(project.id, project.import);
			window.location.assign(resolve('/projects/[id]', { id: project.id }));
		} catch (e) {
			saveMessage = `Ouverture du projet : ${e instanceof ApiError ? e.message : 'échec'}`;
		}
	}

	// « Se souvenir de mon choix » du dialogue : devient le comportement permanent.
	function rememberLoadChoice(mode: ImportLoadMode) {
		savePreferences({ ...currentPreferences(), projectLoadBehaviour: behaviourFromChoice(mode) });
	}

	// « Ne plus afficher » de l'avertissement G-code modifié (préférence du compte).
	function ignoreModifiedGcodes() {
		savePreferences({ ...currentPreferences(), noWarnWhenModifiedGcodes: true });
	}

	async function importZipFile(file: File) {
		try {
			const files = await modelsFromZip(file);
			if (files.length === 0) {
				saveMessage = 'Import ZIP : aucun modèle compatible dans l’archive.';
				return;
			}
			await importFiles(files);
			saveMessage = `Import ZIP : ${files.length} modèle(s) importé(s).`;
		} catch (e) {
			saveMessage = `Import ZIP : ${e instanceof Error ? e.message : 'archive illisible'}`;
		}
	}

	async function importConfigFiles(files: File[]) {
		try {
			const documents = await presetsFromFiles(files);
			for (const document of documents) {
				const created = await importPreset(document);
				if (document.kind === 'process') selectPreset('process', created.id);
				else if (document.kind === 'machine') selectPreset('machine', created.id);
				else if (document.kind === 'filament') selectPreset('filament', created.id);
			}
			await loadPresets();
			saveMessage = `Import des configurations : ${documents.length} profil(s) importé(s).`;
		} catch (e) {
			saveMessage = `Import des configurations : ${e instanceof Error ? e.message : 'échec'}`;
		}
	}

	// Exporte la configuration courante, y compris les modifications encore non
	// enregistrées : un JSON Orca par imprimante/filament/process dans un ZIP.
	async function exportPresetBundle() {
		try {
			const entries: { name: string; data: Uint8Array }[] = [];
			const active: [PresetSummary | undefined, 'machine' | 'filament' | 'process'][] = [
				[activePresetOf('machine'), 'machine'],
				...[...activePresets.filaments].map(
					(id) =>
						[filamentPresets.find((p) => p.id === id), 'filament'] as [
							PresetSummary | undefined,
							'filament'
						]
				),
				[activePresetOf('process'), 'process']
			];
			const used = new SvelteSet<string>();
			for (const [preset, kind] of active) {
				if (!preset) continue;
				const base = safeFileName(preset.name, 'json');
				let name = base;
				for (let n = 2; used.has(name); n++) name = base.replace(/\.json$/, `_${n}.json`);
				used.add(name);
				const values =
					kind === 'filament' && preset.id !== filamentSel ? null : valuesForKind(kind);
				const source = values ?? (await getPreset(preset.id)).values ?? {};
				const json = {
					type: kind,
					name: preset.name,
					...(preset.inherits ? { inherits: preset.inherits } : {}),
					...(source as Record<string, unknown>)
				};
				entries.push({ name, data: new TextEncoder().encode(JSON.stringify(json, null, 2)) });
			}
			if (entries.length === 0) {
				saveMessage = 'Export des presets : aucun preset actif.';
				return;
			}
			downloadBlob(
				buildZip(entries),
				safeFileName(`${projectName || data.project.name}_presets`, 'zip')
			);
			saveMessage = `Export des presets : ${entries.length} profil(s) exporté(s).`;
		} catch (e) {
			saveMessage = `Export des presets : ${e instanceof Error ? e.message : 'échec'}`;
		}
	}

	async function exportGeneric3mf() {
		const outcome = await save();
		if (outcome.status !== 'saved') {
			saveMessage = `Export 3MF : ${outcome.status === 'error' ? outcome.message : 'sauvegarde impossible'}`;
			return;
		}
		// Phase 28 : vignette du plateau actif (Metadata/plate_<n>.png), capturée
		// dans la vue 3D comme « Save project » d'Orca ; sinon rendu serveur.
		const plateIndex = Math.max(
			0,
			plates.list().findIndex((p) => p.id === plates.activeId)
		);
		if (layout.tab === 'prepare' && activeSceneObjects.length > 0) {
			const png = await capturePlatePng();
			if (png) await putPlateThumbnail(data.project.id, plateIndex, png).catch(() => {});
		}
		downloadUrl(`/api/projects/${data.project.id}/export/3mf`);
	}

	// Item de menu sans commande câblée en v1 : informe au lieu d'un no-op muet.
	function menuTodo(action: string) {
		saveMessage = `Menu « ${action} » : commande non disponible en v1.`;
	}

	// Déclenche le téléchargement d'une ressource API (export) via un lien éphémère.
	function downloadUrl(url: string) {
		const a = document.createElement('a');
		a.href = url;
		a.download = '';
		document.body.appendChild(a);
		a.click();
		a.remove();
	}

	// Dispatch de la barre de menus (T107 : menu Fichier). Édition/Vue/Aide sont
	// câblés par T108/T109/T111 ; les items sans objet renvoient un message.
	async function onMenuAction(action: string) {
		switch (action) {
			// — Fichier —
			case 'project.new':
				try {
					const p = await createProject({ name: 'Nouveau projet' });
					// Rechargement complet : la même route SvelteKit ne rejoue pas onMount.
					window.location.assign(resolve('/projects/[id]', { id: p.id }));
				} catch (e) {
					saveMessage = e instanceof ApiError ? e.message : 'création de projet impossible';
				}
				break;
			case 'project.open': {
				// Sélecteur ouvert à la tâche suivante (activation utilisateur
				// toujours valide) : depuis Ctrl+O, le clic synchrone dans le keydown
				// précédait parfois l'interception du sélecteur par l'automate de
				// test (dialogue natif aussitôt annulé, aucun « filechooser »).
				const input = projectFileInput;
				setTimeout(() => input?.click(), 0);
				break;
			}
			case 'project.save':
				await save();
				break;
			case 'project.saveAs': {
				// La copie serveur part de la scène **persistée** : sauvegarder d'abord,
				// sinon les modifications non enregistrées seraient perdues.
				await editorReady;
				const saved = await save();
				if (saved.status !== 'saved') {
					if (saved.status === 'error') saveMessage = `Enregistrer sous : ${saved.message}`;
					break;
				}
				try {
					const dup = await duplicateProject(data.project.id);
					window.location.assign(resolve('/projects/[id]', { id: dup.id }));
				} catch (e) {
					saveMessage = e instanceof ApiError ? e.message : 'duplication impossible';
				}
				break;
			}
			case 'import.geometry':
				fileInput?.click();
				break;
			case 'import.zip':
				zipFileInput?.click();
				break;
			case 'import.configs':
				configFileInput?.click();
				break;
			case 'export.gcode':
			case 'export.plate':
				exportPlateGcode();
				break;
			case 'export.print':
				showTab('preview');
				if (session.gcodeId) printDialogRequest += 1;
				else saveMessage = 'Imprimer le plateau : tranchez d’abord le plateau.';
				break;
			case 'export.allPlates':
				await exportAllPlatesGcode();
				break;
			case 'export.stlOne':
				await exportStl(false, null);
				break;
			case 'export.stlMany':
				await exportStl(true, null);
				break;
			case 'export.drcOne':
			case 'export.drcMany':
				await exportDrc(action === 'export.drcMany', null);
				break;
			case 'gcode.open':
				gcodeFileInput?.click();
				break;
			case 'app.preferences':
				showPreferences = true;
				break;
			case 'view.perspective':
				projection = 'perspective';
				break;
			case 'view.orthogonal':
				projection = 'orthographic';
				break;
			case 'view.labels':
				showLabels = !showLabels;
				break;
			case 'view.overhang':
				showOverhang = !showOverhang;
				break;
			case 'view.gcodeWindow':
				showGcodeWindow = !showGcodeWindow;
				void ensureGcodeLines();
				break;
			case 'export.toolpathsObj':
				await exportToolpathsObj();
				break;
			case 'export.generic3mf':
				await exportGeneric3mf();
				break;
			case 'export.presetBundle':
				await exportPresetBundle();
				break;
			case 'preset.bundle':
				showPresetBundle = true;
				break;
			case 'project.reload':
				if (loadedGcodeFile) await openLocalGcode(loadedGcodeFile);
				else gcodeFileInput?.click();
				break;
			// — Édition (T108) —
			case 'edit.copy':
				copySelection();
				break;
			case 'edit.cut':
				copySelection();
				deleteSelection();
				break;
			case 'edit.paste':
				pasteClipboard();
				break;
			case 'edit.deleteSelected':
				deleteSelection();
				break;
			case 'edit.deleteAll':
				deleteAllObjects();
				break;
			case 'edit.clone':
				cloneSelection();
				break;
			case 'edit.selectAll':
				selectAllObjects();
				break;
			case 'edit.deselectAll':
				deselectAllObjects();
				break;
			case 'edit.duplicatePlate':
				// La restauration remplace le jeu de plateaux : dupliquer après.
				await editorReady;
				duplicateActivePlate();
				break;
			// — Vue (T109) : vues caméra nommées + affichage de la grille —
			case 'view.default':
				cameraView = 'default';
				cameraFocus = null;
				break;
			case 'view.top':
				cameraView = 'top';
				cameraFocus = null;
				break;
			case 'view.bottom':
				cameraView = 'bottom';
				cameraFocus = null;
				break;
			case 'view.front':
				cameraView = 'front';
				cameraFocus = null;
				break;
			case 'view.rear':
				cameraView = 'rear';
				cameraFocus = null;
				break;
			case 'view.left':
				cameraView = 'left';
				cameraFocus = null;
				break;
			case 'view.right':
				cameraView = 'right';
				cameraFocus = null;
				break;
			case 'view.gridlines':
				showGrid = !showGrid;
				break;
			// Phase 37 : navigateur 3D et axes (préférences show_3d_navigator / show_axes).
			case 'view.navigator': {
				const show = !currentPreferences().show3dNavigator;
				savePreferences({ ...currentPreferences(), show3dNavigator: show });
				saveMessage = show ? 'Navigateur 3D affiché.' : 'Navigateur 3D masqué.';
				break;
			}
			// Phase 38 : vue réaliste (Orca « Realistic View », préférence opengl_realistic_mode).
			case 'view.realistic': {
				const prefs = currentPreferences();
				const on = !prefs.graphics.realisticView;
				savePreferences({ ...prefs, graphics: { ...prefs.graphics, realisticView: on } });
				saveMessage = on
					? paintChannel && paintChannel !== 'mmu'
						? 'Vue réaliste activée (reprend à la fermeture de l’outil de peinture).'
						: 'Vue réaliste activée.'
					: 'Vue réaliste désactivée.';
				break;
			}
			case 'view.axes': {
				const show = !currentPreferences().showAxes;
				savePreferences({ ...currentPreferences(), showAxes: show });
				saveMessage = show ? 'Axes affichés.' : 'Axes masqués.';
				break;
			}
			// — Aide (T111) —
			case 'edit.undo':
				undoScene();
				break;
			case 'edit.redo':
				redoScene();
				break;
			case 'help.shortcuts':
				showShortcuts = true;
				break;
			case 'help.setupWizard':
				await goto(resolve('/setup'));
				break;
			case 'help.tipOfTheDay':
				showTipOfTheDay = true;
				break;
			case 'help.about':
				showAbout = true;
				break;
			case 'calib.flowPass1':
			case 'calib.flowPass2':
				await startFlowCalibration(action === 'calib.flowPass1' ? 1 : 2);
				break;
			case 'calib.flowRate':
				flowCalibError = null;
				flowCalibOpen = true;
				break;
			case 'calib.paHistory':
				showPaHistory = true;
				break;
			case 'calib.guide':
				window.open(
					'https://www.orcaslicer.com/wiki/calibration_guide',
					'_blank',
					'noopener,noreferrer'
				);
				break;
			case 'calib.retraction':
			case 'calib.vfa':
			case 'calib.maxVolumetricSpeed':
			case 'calib.paPattern':
			case 'calib.inputShapingFreq':
			case 'calib.inputShapingDamp':
			case 'calib.cornering':
				engineCalibKind = (
					{
						'calib.retraction': 'retraction',
						'calib.vfa': 'vfa',
						'calib.maxVolumetricSpeed': 'max_volumetric_speed',
						'calib.paPattern': 'pa_pattern',
						'calib.inputShapingFreq': 'input_shaping_freq',
						'calib.inputShapingDamp': 'input_shaping_damp',
						'calib.cornering': 'cornering'
					} as const
				)[action];
				engineCalibError = null;
				engineCalibOpen = true;
				break;
			case 'calib.temperature':
			case 'calib.pressureAdvance':
				calibKind = action === 'calib.temperature' ? 'temperature' : 'pressure_advance';
				calibError = null;
				showCalibration = true;
				break;
			default:
				menuTodo(action);
		}
	}

	function showTab(tab: EditorTab) {
		layout = setTab(layout, tab);
	}

	// Phase 31 (lot 29) : POST /slice reçoit la scène affichée (`scene`), comme
	// /validate — plus d'enregistrement avant tranchage, l'état « non
	// enregistré » est conservé (Orca tranche le plateau courant sans
	// enregistrer). Replis : serveur antérieur (champ ignoré, cf.
	// supportsInlineScene) ou modèle pas encore rattaché au projet (422
	// field=scene) → enregistrement puis tranchage de la scène enregistrée.
	// Presets actifs modifiés : envoyés dans `active_presets` (lot 31, plus
	// d'enregistrement forcé) ; config projet modifiée : enregistrée d'abord
	// (projectConfigSaved).
	const SLICE_CONFLICT = 'Conflit : le projet a été modifié ailleurs. Rechargez avant de trancher.';
	async function requestSlice(
		req: ReturnType<typeof sliceRequestWithDraft>
	): Promise<Awaited<ReturnType<typeof sliceProject>>> {
		const saveFirst = async () => {
			const saved = await save();
			if (saved.status === 'conflict') throw new SliceSaveError(SLICE_CONFLICT);
			if (saved.status === 'error') throw new SliceSaveError(saved.message);
		};
		const dirty = saveSignature() !== savedSignature;
		const inline = dirty && projectConfigSaved() && (await supportsInlineScene(data.project.id));
		if (dirty && !inline) await saveFirst();
		if (!inline) return sliceProject(data.project.id, req);
		try {
			return await sliceProject(data.project.id, {
				...req,
				scene: buildSceneForSave(),
				active_presets: activePresetsBody()
			});
		} catch (e) {
			const field =
				e instanceof ApiError ? (e.details as { field?: unknown } | undefined)?.field : null;
			if (!(e instanceof ApiError && e.status === 422 && field === 'scene')) throw e;
			await saveFirst();
			return sliceProject(data.project.id, req);
		}
	}
	class SliceSaveError extends Error {}
	// Presets actifs affichés (filaments ajoutés, imprimante…) : `scene` ne les
	// transporte pas ; lot 31 → champ `active_presets` de /slice, /validate et
	// /filament-compatibility (même format que le projet). Remplace la
	// sauvegarde forcée de la phase 31 (activePresetsSaved).
	function activePresetsBody(): { printer?: string; process?: string; filaments?: string[] } {
		return serializeActivePresets(activePresets) as {
			printer?: string;
			process?: string;
			filaments?: string[];
		};
	}
	// Config projet hors objets (`scene.projectConfig` : couleurs des slots,
	// matrice et multiplicateur de purge, position de la tour) : même règle que
	// les presets actifs. Le tranchage la lit bien dans la scène envoyée, mais
	// elle appartient au projet (export 3MF, vérification des filaments, autre
	// onglet) : modifiée depuis la dernière sauvegarde → enregistrement avant
	// validation / tranchage. Comparaison normalisée (parse + compactage) avec
	// la scène enregistrée (`data.project.scene`, mise à jour par saveNow).
	function projectConfigSaved(): boolean {
		const saved = (data.project.scene as { projectConfig?: unknown } | null)?.projectConfig;
		return sameProjectConfig(saved, $state.snapshot(projectFilament));
	}

	// Lance le tranchage du plateau actif et affiche la progression dans l'aperçu
	// (scène affichée envoyée telle quelle, cf. requestSlice).
	async function sliceActive() {
		// Module de l'aperçu chargé pendant le tranchage (pas à l'ouverture).
		void loadPreviewScene();
		await editorReady;
		await presetSwitches;
		if (!(await askFilamentMap(false))) return;
		const req = sliceRequestWithDraft('plate', plates.plateIndex(plates.activeId ?? ''));
		try {
			const res = await requestSlice(req);
			session = startSlicing(
				res.jobs.map((j) => j.id),
				res.warnings
			);
			sliceBatch = makeBatch(
				res.jobs,
				plates.list().map((p) => p.id),
				sliceSignature()
			);
			layout = setTab(layout, 'preview');
		} catch (e) {
			session = sliceFailed(
				e instanceof ApiError || e instanceof SliceSaveError
					? e.message
					: 'échec du lancement du tranchage'
			);
			layout = setTab(layout, 'preview');
		}
	}

	// « Slice all » d'Orca (bouton Trancher › Trancher tout) : un job par plateau
	// non vide ; l'aperçu montre le plateau actif dès qu'il est prêt, chaque
	// résultat est mémorisé et réaffiché en changeant d'onglet de plateau.
	let sliceMenuOpen = $state(false);
	async function sliceAll() {
		sliceMenuOpen = false;
		void loadPreviewScene();
		await editorReady;
		await presetSwitches;
		if (!(await askFilamentMap(true))) return;
		try {
			const res = await requestSlice(sliceRequestWithDraft('all', 0));
			session = startSlicing(
				res.jobs.map((j) => j.id),
				res.warnings
			);
			sliceBatch = makeBatch(
				res.jobs,
				plates.list().map((p) => p.id),
				sliceSignature()
			);
			if (res.jobs.length > 1)
				saveMessage = `Trancher tout : 0 / ${res.jobs.length} plateaux tranchés…`;
			layout = setTab(layout, 'preview');
		} catch (e) {
			session = sliceFailed(
				e instanceof ApiError || e instanceof SliceSaveError
					? e.message
					: 'échec du lancement du tranchage'
			);
			layout = setTab(layout, 'preview');
		}
	}

	// Charge la méta d'aperçu puis prépare le chargeur paresseux (T084).
	async function loadPreview(gcodeId: string) {
		engineSource = gcodeEngineSource(gcodeId);
		openedDeclaredTime = null;
		// Projet rouvert (pas d'événement job.finished dans cette session) : les
		// statistiques du G-code alimentent le « Résultat du tranchage ».
		if (!rawStats) {
			void fetch(`/api/gcodes/${gcodeId}/stats`, { credentials: 'include' })
				.then((r) => (r.ok ? r.json() : null))
				.then((stats) => {
					if (stats && engineSource?.key === 'g:' + gcodeId && !rawStats) rawStats = stats;
				})
				.catch(() => {});
		}
		setGcodeSource(async () => {
			const resp = await fetch(`/api/gcodes/${gcodeId}/download`, { credentials: 'include' });
			if (!resp.ok) throw new ApiError(resp.status, 'error', resp.statusText);
			return resp.text();
		});
		previewMeta = await getPreviewMeta(gcodeId);
		loader = new LazyPreviewLoader({
			layerCount: previewMeta.layer_count,
			fetchRange: (from, to) => fetchPreviewLayers(gcodeId, from, to)
		});
		layerRange = makeLayerRange(previewMeta.layer_count);
		await refreshGeometry();
	}

	// Couleurs distinctes (hex) des segments rendus, plafonnées à 16 : exposées en
	// attribut de test pour vérifier la coloration sans lire le canvas WebGL.
	function previewColors(geometry: PreviewGeometry): string[] {
		// Tableau local (non réactif) : pas de `Set` mutable dans un composant.
		const seen: string[] = [];
		const c = geometry.colors;
		const hex = (v: number) =>
			Math.round(Math.min(1, Math.max(0, v)) * 255)
				.toString(16)
				.padStart(2, '0');
		for (let i = 0; i < geometry.visibleCount * 2 && seen.length < 16; i++) {
			const color = `#${hex(c[i * 3])}${hex(c[i * 3 + 1])}${hex(c[i * 3 + 2])}`;
			if (!seen.includes(color)) seen.push(color);
		}
		return seen.sort();
	}
	// Reconstruit la géométrie de lignes pour la couche haute visible, avec la
	// coloration active et les types de ligne visibles (T116).
	//
	// Appels concurrents regroupés (phase 25) : à l'ouverture de l'aperçu, la
	// méta WSPv, la piste texte puis la méta moteur déclenchaient chacune une
	// reconstruction complète (×3 téléchargements des tranches et ×3 rendus
	// WebGL lourds, plusieurs secondes de fil principal bloqué en rendu
	// logiciel). Une seule reconstruction à la fois, relancée une fois à la fin si
	// d'autres demandes sont arrivées entre-temps.
	let geometryRun: Promise<void> | null = null;
	let geometryAgain = false;
	function refreshGeometry(): Promise<void> {
		if (geometryRun) {
			geometryAgain = true;
			return geometryRun;
		}
		const run = (async () => {
			try {
				do {
					geometryAgain = false;
					await refreshGeometryNow();
				} while (geometryAgain);
			} finally {
				geometryRun = null;
			}
		})();
		geometryRun = run;
		return run;
	}
	async function refreshGeometryNow() {
		if (!loader || !previewMeta) return;
		const { from, to } = visibleRange(layerRange);
		await loader.ensureRange(from, to);
		const moves = loader.rangeSegments(to, to).reduce((total, layer) => total + layer.count, 0);
		if (moveCursorLayer !== to) {
			moveCursorLayer = to;
			moveCursor = makeMoveCursor(moves);
		} else {
			moveCursor = { total: moves, position: Math.min(moveCursor.position, moves) };
		}
		const visibleTypes = hiddenTypes.length
			? new Set(
					(previewStats?.types ?? []).map((t) => t.kind).filter((k) => !hiddenTypes.includes(k))
				)
			: undefined;
		const windowSegments = loader.rangeSegments(from, to);
		// Débit : une seule échelle pour toute la fenêtre (géométrie et légende).
		previewFlowRange = windowSegments
			.map(computeFlowRange)
			.reduce<[number, number]>(
				(acc, [lo, hi]) => (hi > 0 ? [Math.min(acc[0], lo), Math.max(acc[1], hi)] : acc),
				[Infinity, 0]
			);
		if (!Number.isFinite(previewFlowRange[0])) previewFlowRange = [0, 0];
		previewGeometry = buildWindowGeometry(windowSegments, {
			coloration,
			ranges: { ...rangesFromMeta(previewMeta), flow: previewFlowRange },
			visibleTypes,
			filamentColors: previewFilamentColors,
			layerRange: { low: from, high: to },
			moveCursor: { layer: to, position: moveCursor.position },
			track:
				previewTrack && trackMatches(previewTrack, previewMeta.layer_count) ? previewTrack : null
		});
		previewTool = toolPositionAt(
			loader.rangeSegments(to, to),
			to,
			moveCursor.position,
			previewTrack && trackMatches(previewTrack, previewMeta.layer_count) ? previewTrack : null
		);
		if (!previewFrame) {
			previewFrame = boundsOf(previewGeometry.positions);
			if (previewFrame) previewFrameKey++;
		}
	}
	// Bascule la visibilité d'un type de ligne et rafraîchit la géométrie.
	function toggleType(kind: number) {
		hiddenTypes = hiddenTypes.includes(kind)
			? hiddenTypes.filter((k) => k !== kind)
			: [...hiddenTypes, kind];
		void refreshGeometry();
	}
	// Déplace une poignée du double curseur de couches (T116).
	function moveLayerThumb(delta: number) {
		layerRange = moveThumb(layerRange, delta);
		void refreshGeometry();
	}
	// Shift/Ctrl + molette : déplacement rapide du curseur vertical (groupe Preview).
	function onPreviewWheel(e: WheelEvent) {
		if (!e.shiftKey && !e.ctrlKey && !e.metaKey) return;
		e.preventDefault();
		moveLayerThumb((e.deltaY < 0 ? 1 : -1) * FAST_STEP);
	}
	// Ctrl + molette sur la scène en mode peinture : rayon du pinceau (Gizmo).
	function onSceneWheel(e: WheelEvent) {
		if (!paintChannel) return;
		if (e.altKey && !e.ctrlKey) {
			// Alt+molette : position du plan de coupe (GLGizmoPainterBase).
			e.preventDefault();
			sectionNormalDir ??= sectionNormal(cameraForward);
			sectionRatio = stepSectionRatio(sectionRatio, e.deltaY < 0 ? 1 : -1);
			return;
		}
		if (!e.ctrlKey) return;
		e.preventDefault();
		// Ctrl + molette : paramètre de l'outil actif (rayon, hauteur, angle, aire).
		const next = wheelParams(paintTool, paintParams, e.deltaY < 0);
		paintRadius = next.radius;
		paintHeight = next.height;
		paintFillAngle = next.fillAngle;
		paintGapArea = next.gapArea;
	}
	// La coupe ne vit que pendant la peinture (réinitialisée à la fermeture du gizmo).
	$effect(() => {
		if (!paintChannel) {
			sectionRatio = 0;
			sectionNormalDir = null;
		}
	});
	// Hauteur monde du plan de coupe sur l'objet sélectionné (maillage centré).
	const sectionClip = $derived.by(() => {
		if (!paintChannel || !activeObject || !sectionNormalDir || sectionRatio <= 0) return null;
		// Sphère englobante de l'instance (rayon = demi-diagonale de sa boîte).
		const radius =
			Math.hypot(...worldSize(activeObject.mesh, activeObject.rotation, activeObject.scale)) / 2;
		return sectionPlane(sectionNormalDir, activeObject.position ?? [0, 0, 0], radius, sectionRatio);
	});

	/** Scène telle que la sauvegarde l'envoie (sert aussi à l'indicateur
	 *  « modifié depuis la dernière sauvegarde »). */
	function buildSceneForSave() {
		// Objets sérialisables : ceux adossés à un modèle serveur (les primitives
		// aussi, téléversées en STL). La clé de restauration est le `modelId` ;
		// on persiste la transformation (placement/rotation/échelle) et le profil de
		// hauteur de couche. Les objets sans `modelId` (upload en cours) sont ignorés
		// et seront inclus au prochain enregistrement.
		// Ordre de la liste d'objets = ordre du modèle = ordre d'impression
		// séquentielle (Orca) : la scène sauvegardée (donc le tranchage) le suit,
		// quel que soit l'ordre d'arrivée des maillages.
		const treeOrder = tree.list().map((n) => n.id);
		const objects: SceneObjectState[] = permuteInSlots(sceneObjects, treeOrder, (o) => o.id)
			.map((o) => {
				const modelId = imports.find((i) => i.objectId === o.id)?.modelId;
				if (!modelId) return null;
				return {
					id: o.id,
					modelId,
					transform: {
						position: o.position ?? [0, 0, 0],
						rotation: o.rotation ?? [0, 0, 0],
						scale: o.scale ?? [1, 1, 1]
					},
					extruder: clampExtruder(tree.effectiveExtruder(o.id), filamentSlots),
					name: tree.get(o.id)?.name,
					meshName: tree.get(o.id)?.meshName,
					emboss: tree.get(o.id)?.emboss,
					cutId: tree.get(o.id)?.cutId,
					printable: tree.isPrintable(o.id),
					settings: { ...(tree.get(o.id)?.settings ?? {}) },
					painting:
						paintings[o.id] ??
						(savedDocumentByObject[o.id]
							? deserializeObject(savedDocumentByObject[o.id]).painting
							: new TrianglePainting()),
					layerProfile: layerProfiles[o.id] ?? [],
					brimEars:
						brimEarsByObject[o.id] ??
						(savedDocumentByObject[o.id]
							? deserializeObject(savedDocumentByObject[o.id]).brimEars
							: new BrimEars()),
					volumes: $state.snapshot(
						volumesByObject[o.id] ??
							(savedDocumentByObject[o.id]
								? (deserializeObject(savedDocumentByObject[o.id]).volumes ?? [])
								: [])
					) as SceneVolume[],
					layerRanges: $state.snapshot(
						layerRangesByObject[o.id] ??
							(savedDocumentByObject[o.id]
								? (deserializeObject(savedDocumentByObject[o.id]).layerRanges ?? [])
								: [])
					) as HeightRange[]
				} satisfies SceneObjectState;
			})
			.filter((o) => o !== null) as SceneObjectState[];
		const scene = serializeScene(
			plates.serialize(),
			objects,
			{
				projectId: data.project.id,
				ids: [...new Set([...projectModelIds, ...objects.map((o) => o.modelId)])]
			},
			sceneCalibration,
			compactProjectConfig(projectFilament) as Record<string, unknown> | null
		);
		if (sceneCalibrationOverrides)
			scene.calibrationOverrides = structuredClone(sceneCalibrationOverrides);
		return scene;
	}

	// — Indicateur « non enregistré » : empreinte de ce que la sauvegarde
	//   persisterait (scène, réglages objet, peinture, presets actifs) comparée à
	//   celle de la dernière sauvegarde / ouverture, plus les brouillons de profil
	//   non enregistrés. Référence prise une fois l'éditeur restauré.
	function saveSignature(scene = buildSceneForSave()): string {
		return JSON.stringify(scene) + JSON.stringify(serializeActivePresets(activePresets));
	}
	let savedSignature = $state<string | null>(null);
	let currentSignature = $state<string | null>(null);
	let signatureTimer: ReturnType<typeof setTimeout> | undefined;
	$effect(() => {
		// Dépendances suivies : toute modification persistée par la sauvegarde.
		void sceneObjects;
		void $state.snapshot(tree.list());
		void plates.serialize();
		void $state.snapshot(activePresets);
		void $state.snapshot(layerProfiles);
		void $state.snapshot(volumesByObject);
		void $state.snapshot(layerRangesByObject);
		// Config projet (couleurs, purge, tour) : persistée dans la scène.
		void $state.snapshot(projectFilament);
		void activePaintedCount;
		void baselineReady;
		const pending = imports.some((item) => isPending(item));
		clearTimeout(signatureTimer);
		if (pending) return;
		signatureTimer = setTimeout(() => {
			const sig = saveSignature();
			currentSignature = sig;
			if (savedSignature === null && baselineReady) savedSignature = sig;
		}, 300);
	});
	let baselineReady = $state(false);
	/** Ouverture : objets restaurés depuis des modèles absents de la scène enregistrée. */
	let restoredOrphans = false;
	const profilesDirty = $derived(processModified || filamentModified || machineModified);
	// Projet neuf et vide : rien à perdre (les presets par défaut ne comptent pas).
	const hasContent = $derived(
		sceneObjects.length > 0 ||
			((data.project.scene as { objects?: unknown[] } | null)?.objects?.length ?? 0) > 0
	);
	const unsaved = $derived(
		savedSignature !== null &&
			hasContent &&
			((currentSignature !== null && currentSignature !== savedSignature) ||
				profilesDirty ||
				imports.some((item) => isPending(item)))
	);
	/** Calcul synchrone (alerte de sortie) : ne dépend pas du différé. */
	function unsavedNow(): boolean {
		if (savedSignature === null) return sceneObjects.length > 0;
		if (!hasContent) return false;
		if (profilesDirty || imports.some((item) => isPending(item))) return true;
		return saveSignature() !== savedSignature;
	}

	// Sauvegardes sérialisées (phase 29) : la validation en direct enregistre la
	// scène ; sans file, un « Trancher » pendant cet enregistrement partait avec
	// la même version attendue et finissait en faux conflit 409.
	let saveChain: Promise<unknown> = Promise.resolve();
	function save(opts: { quiet?: boolean } = {}): Promise<SaveOutcome> {
		const run = saveChain.then(() => saveNow(opts));
		saveChain = run.catch(() => {});
		return run;
	}
	async function saveNow(opts: { quiet?: boolean } = {}): Promise<SaveOutcome> {
		if (arrangeInFlight) await arrangeInFlight;
		const pending = imports.filter((item) => isPending(item));
		if (pending.length > 0) {
			return {
				status: 'error',
				message: `Attendez la fin de ${pending.length} import(s) avant de sauvegarder ou trancher.`
			};
		}
		const scene = buildSceneForSave();
		const outcome = await saveScene(
			data.project.id,
			Number(data.project.version),
			scene,
			serializeActivePresets(activePresets)
		);
		// Verrou optimiste : on adopte la version renvoyée pour que les
		// enregistrements suivants (dont le save implicite avant tranchage) ne
		// déclenchent pas un faux conflit 409.
		// La scène sauvegardée devient la référence locale : « Recharger » relit
		// `data.project.scene`, qui ne doit pas rester sur l'état d'ouverture.
		if (outcome.status === 'saved') {
			data.project.version = BigInt(outcome.version);
			data.project.scene = scene;
			savedSignature = saveSignature(scene);
			currentSignature = savedSignature;
			// Les ids persistés sont désormais les ids vivants.
			persistedToLive.clear();
			// Phase 37 : erreurs de maillage de la scène enregistrée.
			void refreshMeshErrors();
		}
		// Enregistrement implicite de la validation en direct : sans message.
		if (opts.quiet) return outcome;
		saveMessage =
			outcome.status === 'saved'
				? 'Projet sauvegardé.'
				: outcome.status === 'conflict'
					? 'Conflit : le projet a été modifié dans un autre onglet.'
					: outcome.message;
		return outcome;
	}

	// Onglets supérieurs : libellé de parité anglais → i18n.
	const TABS: { id: EditorTab; label: string; shortLabel: string }[] = [
		{ id: 'prepare', label: 'Prepare', shortLabel: 'Prepare' },
		{ id: 'preview', label: 'Preview', shortLabel: 'Preview short' },
		{ id: 'device', label: 'Device', shortLabel: 'Device' },
		{ id: 'project', label: 'Project', shortLabel: 'Project' }
	];
</script>

<!-- Titre d'onglet = nom du projet (barre de titre d'Orca : « nom — OrcaSlicer »). -->
<svelte:head><title>{projectName.trim() || data.project.name} — Web-Slicer</title></svelte:head>

<!-- Raccourcis clavier de l'éditeur (T114) : dispatch global, isolé des champs. -->
<svelte:window onkeydown={onEditorKeydown} />

<div class="flex h-screen flex-col bg-surface text-content">
	<!-- Barre supérieure : onglets de vue (parité OrcaSlicer) + actions plateau. -->
	<header
		class="flex items-center justify-between gap-3 border-b border-border px-2 py-2 2xl:px-4"
		data-testid="editor-topbar"
		data-preset-switch-pending={presetSwitchPending ?? undefined}
		aria-busy={presetSwitchPending ? 'true' : undefined}
	>
		<h1 class="ws-sr-only">{projectName || data.project.name} — éditeur web-slicer</h1>
		<!-- overflow-x-clip (et non overflow-hidden) : la barre reste coupée en largeur
		     sur écran étroit, mais les menus déroulants (Fichier, Édition…) débordent
		     vers le bas ; overflow-hidden les rendait invisibles. -->
		<div class="flex min-w-0 flex-1 items-center gap-2 overflow-x-clip 2xl:gap-4">
			<a
				href={resolve('/library')}
				class="text-sm text-primary hover:underline"
				title="Bibliothèque">←</a
			>
			<!-- Barre de menus principale (T107–T111) : Fichier/Édition/Vue/Aide. -->
			<MenuBar menus={MAIN_MENUS} onaction={onMenuAction} />
			<nav
				class="flex shrink-0 items-center gap-1 whitespace-nowrap"
				aria-label="Vues de l'éditeur"
			>
				{#each TABS as tab (tab.id)}
					<button
						class="ws-btn px-2 2xl:px-3 {layout.tab === tab.id ? 'ws-btn-primary' : 'ws-btn-ghost'}"
						onclick={() => showTab(tab.id)}
						aria-current={layout.tab === tab.id ? 'page' : undefined}
					>
						<span class="2xl:hidden">
							{tab.id === 'preview'
								? $locale === 'fr'
									? 'Aperçu'
									: 'Preview'
								: $t(tab.shortLabel)}
						</span>
						<span class="hidden 2xl:inline">{$t(tab.label)}</span>
					</button>
				{/each}
			</nav>
			<span
				class="ml-2 hidden min-w-0 truncate text-sm text-content-subtle 2xl:inline"
				data-testid="project-name"
				title={savedName || data.project.name}>{savedName || data.project.name}</span
			>
		</div>

		<div class="flex shrink-0 items-center gap-2 whitespace-nowrap">
			<button class="ws-btn ws-btn-secondary" onclick={() => fileInput?.click()}>
				{$t('Import')}
			</button>
			<input
				bind:this={fileInput}
				type="file"
				data-testid="import-input"
				multiple
				accept={ACCEPT}
				class="hidden"
				onchange={onFilePicked}
			/>
			<input
				bind:this={replacementInput}
				type="file"
				data-testid="replacement-input"
				accept={ACCEPT}
				class="hidden"
				onchange={onReplacementPicked}
			/>
			<!-- Réglages du plateau actif (icône plateau d'Orca) : séquences, vase,
			     type de plaque, en surcharge du profil. -->
			<button
				type="button"
				class="ws-btn ws-btn-secondary"
				data-testid="plate-settings-button"
				title="Réglages du plateau"
				disabled={!plates.activeId}
				onclick={() => (plateSettingsOpen = true)}
			>
				<!-- Libellés longs des actions dès 1920 px seulement : de 1536 à ~1800 px
				     ils recouvraient les onglets « Appareil » et « Projet » (phase 36). -->
				<IconPlateSettings class="h-4 w-4" aria-hidden="true" /><span
					class="hidden min-[1920px]:inline">Plateau</span
				>
			</button>
			<PlateSettingsDialog
				open={plateSettingsOpen}
				settings={plates.list().find((p) => p.id === plates.activeId)?.settings ?? {}}
				bind:mode={settingsMode}
				onClose={() => (plateSettingsOpen = false)}
				onsetting={(key, value) =>
					plates.activeId && plates.setPlateSetting(plates.activeId, key, value)}
				extruders={extruderCount(machineValues.nozzle_diameter)}
				filamentColours={filamentSlotColors}
				inherited={plateInherited}
			/>
			<!-- Bouton Trancher + options (Orca : « Slice plate » / « Slice all »). -->
			<div class="relative flex">
				<button
					class="ws-btn ws-btn-primary relative rounded-r-none"
					data-testid="slice-button"
					data-blocked={activePlateBlocked ? 'true' : undefined}
					onclick={sliceActive}
					disabled={!canSliceNow}
					title={!canSliceNow
						? sliceBlocker
						: activePlateBlocked
							? 'Le plateau a des erreurs bloquantes : voir la barre de vérification en bas à droite'
							: 'Trancher le plateau actif'}
				>
					{#if activePlateBlocked && canSliceNow}
						<!-- Orca grise « Trancher » ; ici le bouton reste actif (le moteur
						     redira pourquoi) mais signale les erreurs connues. -->
						<span
							class="absolute -top-1 -right-1 h-2.5 w-2.5 rounded-full border-2 border-surface-raised bg-danger"
							data-testid="slice-blocked-dot"
							aria-hidden="true"
						></span>
					{/if}
					<span class="min-[1920px]:hidden">{$locale === 'fr' ? 'Trancher' : 'Slice'}</span>
					<span class="hidden min-[1920px]:inline">{$t('Slice plate')}</span>
				</button>
				<button
					type="button"
					class="ws-btn ws-btn-primary rounded-l-none border-l border-l-white/30 px-1.5"
					data-testid="slice-menu-button"
					aria-haspopup="menu"
					aria-expanded={sliceMenuOpen}
					aria-label="Options de découpe"
					title="Trancher le plateau ou tous les plateaux"
					onclick={() => (sliceMenuOpen = !sliceMenuOpen)}
				>
					<span aria-hidden="true">▾</span>
				</button>
				{#if sliceMenuOpen}
					<button
						type="button"
						class="fixed inset-0 z-40 cursor-default"
						tabindex="-1"
						aria-label="Fermer le menu"
						onclick={() => (sliceMenuOpen = false)}
					></button>
					<div
						class="absolute top-full right-0 z-50 mt-1 min-w-52 ws-menu"
						role="menu"
						data-testid="slice-menu"
					>
						<button
							type="button"
							class="ws-menu-item"
							role="menuitem"
							data-testid="slice-plate-item"
							disabled={!canSliceNow}
							onclick={() => {
								sliceMenuOpen = false;
								void sliceActive();
							}}>{$t('Slice plate')}</button
						>
						<button
							type="button"
							class="ws-menu-item"
							role="menuitem"
							data-testid="slice-all"
							disabled={!canSliceAll}
							onclick={() => void sliceAll()}
							>{$t('Slice all')}
							<span class="ml-auto text-xs text-content-subtle"
								>{plates.list().length} plateau{plates.list().length > 1 ? 'x' : ''}</span
							></button
						>
					</div>
				{/if}
			</div>
			<SaveControls {save} {unsaved} />
			<button
				type="button"
				class="ws-btn ws-btn-secondary"
				data-testid="export-gcode-button"
				disabled={!session.gcodeId}
				onclick={() => session.gcodeId && downloadUrl(`/api/gcodes/${session.gcodeId}/download`)}
			>
				<span class="min-[1920px]:hidden">G-code…</span>
				<span class="hidden min-[1920px]:inline">{$t('Export G-code…')}</span>
			</button>
			<SendToPrinter
				gcodeId={session.gcodeId}
				openRequest={printDialogRequest}
				plateCount={plates.list().length}
				sendAll={sendAllPlates}
			/>
			<!-- Export projet 3MF (FR-044) : ressource API backend, pas une route SvelteKit. -->
			<!-- eslint-disable svelte/no-navigation-without-resolve -->
			<!-- Doublon du menu Fichier : masqué sous 1536 px pour que les onglets de vue
			     ne passent pas sous les actions (1280/1440 px, libellés français). -->
			<a
				class="hidden rounded bg-primary px-3 py-1.5 text-sm text-primary-content hover:bg-primary-hover 2xl:inline-block"
				href="/api/projects/{data.project.id}/export/3mf"
				download
				onclick={(e) => {
					e.preventDefault();
					void exportGeneric3mf();
				}}
			>
				{$t('Export Generic 3MF…')}
			</a>
			<!-- eslint-enable svelte/no-navigation-without-resolve -->
		</div>
	</header>

	<!-- Dialogue des raccourcis clavier (menu Aide, T111). -->
	<ShortcutsDialog open={showShortcuts} onClose={() => (showShortcuts = false)} />
	<TipOfTheDayDialog open={showTipOfTheDay} onClose={() => (showTipOfTheDay = false)} />
	<AboutDialog open={showAbout} onclose={() => (showAbout = false)} />
	{#if unitPending}
		<UnitConversionDialog
			filename={unitPending.filename}
			size={unitPending.size}
			suggested={unitPending.suggested}
			onconvert={(unit) => {
				if (unitPending) convertObjectUnits(unitPending.objectId, unit, false);
				unitPending = null;
			}}
			onkeep={(dontAsk) => {
				if (dontAsk) savePreferences({ ...currentPreferences(), unitConversionPrompt: false });
				unitPending = null;
			}}
		/>
	{/if}
	{#if filamentMapPrompt}
		<FilamentMapDialog
			extruders={extruderCount(machineValues.nozzle_diameter)}
			colours={filamentSlotColors}
			mode={filamentMapPrompt.mode}
			map={filamentMapPrompt.map}
			filaments={filamentMapPrompt.filaments}
			allPlates={filamentMapPrompt.allPlates}
			onconfirm={(mode, map) => closeFilamentMap({ mode, map })}
			oncancel={() => closeFilamentMap(null)}
			ondisable={() => savePreferences({ ...currentPreferences(), filamentMapPopup: false })}
		/>
	{/if}
	{#if handySuggestion}
		<HandySuggestionDialog
			suggestion={handySuggestion}
			values={processValues}
			onapply={() => {
				if (handySuggestion) processValues = { ...processValues, ...handySuggestion.apply };
				handySuggestion = null;
			}}
			onignore={() => (handySuggestion = null)}
		/>
	{/if}

	<!-- Options d'arrangement (lot 34) : panneau togglé par le bouton Arrange. -->
	<!-- Préparer seulement : l'Aperçu d'Orca n'a pas de menu d'arrangement. -->
	<ArrangeOptionsPanel
		open={arrangePanel && layout.tab === 'prepare'}
		options={arrangeSaved}
		info={arrangeInfo}
		activePlateLocked={plates.get(plates.activeId ?? '')?.locked === true}
		busy={arrangeBusy}
		onchange={setArrangeOptions}
		onarrange={(scope) => {
			// Le panneau reste ouvert : Orca garde son menu d'arrangement actif
			// tant que l'utilisateur ne le ferme pas (ré-arrangement en un clic).
			void arrangeGui(scope);
		}}
		onclose={() => (arrangePanel = false)}
	/>

	<!-- Ouverture d'un 3MF : projet ou géométrie seule (lot 34). -->
	<OpenProjectDialog
		open={open3mf !== null}
		filename={open3mf?.name ?? ''}
		onconfirm={(mode, remember) => {
			if (remember && open3mf) rememberLoadChoice(mode);
			if (open3mf) void openProjectWithLoad(open3mf, mode);
		}}
		oncancel={() => (open3mf = null)}
	/>

	<!-- Avertissement « G-code modifié » du 3MF ouvert (lot 34). -->
	<ImportReportDialog
		report={importReport}
		onignore={ignoreModifiedGcodes}
		onshowmetadata={() => {
			configTab = 'project';
			importReport = null;
		}}
		onclose={() => (importReport = null)}
	/>

	<input
		bind:this={projectFileInput}
		type="file"
		data-testid="project-open-input"
		accept={ACCEPT}
		class="hidden"
		onchange={(e) => {
			const file = e.currentTarget.files?.[0];
			if (file) void openProjectFile(file);
			e.currentTarget.value = '';
		}}
	/>
	<input
		bind:this={zipFileInput}
		type="file"
		data-testid="zip-import-input"
		accept=".zip"
		class="hidden"
		onchange={(e) => {
			const file = e.currentTarget.files?.[0];
			if (file) void importZipFile(file);
			e.currentTarget.value = '';
		}}
	/>
	{#if stepMeshPending}
		<StepMeshDialog
			open
			filename={stepMeshPending.filename}
			modelId={stepMeshPending.modelId}
			initialTriangles={stepMeshPending.triangles}
			onremeshed={() => reloadStepMesh(stepMeshPending!.modelId)}
			onclose={() => (stepMeshPending = null)}
		/>
	{/if}
	{#if objColorPending}
		<ObjColorDialog
			open
			filename={objColorPending.filename}
			palette={objColorPending.res.palette}
			slotColours={filamentSlotColors}
			onapply={applyObjColors}
			onskip={nextObjColor}
		/>
	{/if}
	<input
		bind:this={configFileInput}
		type="file"
		data-testid="config-import-input"
		accept=".json,.zip,.orca_printer,.orca_bundle,.orca_filament"
		multiple
		class="hidden"
		onchange={(e) => {
			const files = [...(e.currentTarget.files ?? [])];
			if (files.length) void importConfigFiles(files);
			e.currentTarget.value = '';
		}}
	/>

	<!-- Préférences (menu Application) et ouverture d'un G-code local. -->
	<PreferencesDialog
		open={showPreferences}
		prefs={preferences}
		onchange={(next) => (preferences = next)}
		onclose={() => (showPreferences = false)}
	/>
	<input
		bind:this={gcodeFileInput}
		type="file"
		data-testid="gcode-open-input"
		accept=".gcode,.gco,.g"
		class="hidden"
		onchange={(e) => {
			const file = e.currentTarget.files?.[0];
			if (file) void openLocalGcode(file);
			e.currentTarget.value = '';
		}}
	/>
	<CalibrationDialog
		open={showCalibration}
		kind={calibKind}
		busy={calibBusy}
		error={calibError}
		onclose={() => (showCalibration = false)}
		onconfirm={startCalibration}
	/>
	<EngineCalibrationDialog
		open={engineCalibOpen}
		kind={engineCalibKind}
		busy={engineCalibBusy}
		error={engineCalibError}
		onclose={() => (engineCalibOpen = false)}
		onconfirm={startEngineCalibration}
	/>
	<FlowRateDialog
		open={flowCalibOpen}
		busy={flowCalibBusy}
		error={flowCalibError}
		onclose={() => (flowCalibOpen = false)}
		onconfirm={startFlowCalibration}
	/>
	<PresetBundleDialog
		open={showPresetBundle}
		projectName={data.project.name}
		presets={presetBundleItems}
		onclose={() => (showPresetBundle = false)}
		onimport={() => {
			showPresetBundle = false;
			configFileInput?.click();
		}}
		onexport={() => {
			void exportPresetBundle();
			showPresetBundle = false;
		}}
	/>

	<!-- Grille de sélection d'imprimante (façon OrcaSlicer) : ajout d'une imprimante
	     possédée depuis l'éditeur (Phase 14, refonte). « Toujours cet écran ». -->
	{#if showCatalog}
		<div
			class="fixed inset-0 z-50 flex items-center justify-center bg-scrim p-4"
			role="presentation"
			onclick={() => (showCatalog = false)}
		>
			<div
				class="ws-dialog flex h-[85vh] w-full max-w-4xl flex-col overflow-hidden"
				role="dialog"
				aria-modal="true"
				aria-label="Ajouter une imprimante"
				onclick={(e) => e.stopPropagation()}
				onkeydown={(e) => e.stopPropagation()}
				tabindex="-1"
				use:dialogFocus={() => (showCatalog = false)}
			>
				<PrinterCatalog
					vendors={printerCatalog}
					onconfirm={addPrintersFromCatalog}
					oncancel={() => (showCatalog = false)}
					confirmLabel="Ajouter"
					busy={addingPrinter}
				/>
			</div>
		</div>
	{/if}

	<!-- Menu contextuel objet (T112) : scène + liste d'objets. -->
	<ContextMenu
		open={contextMenu.open}
		x={contextMenu.x}
		y={contextMenu.y}
		items={[
			...(embossTarget(contextMenu.targetId)
				? [
						{
							label:
								embossEditLabel(embossTarget(contextMenu.targetId)?.emboss) ?? 'Modifier le texte',
							action: 'object.editEmboss'
						},
						'separator' as const
					]
				: []),
			...objectMenuExtras,
			...(contextMenu.part
				? PART_CONTEXT_ITEMS
				: withUnitItems(
						OBJECT_CONTEXT_ITEMS,
						sceneObjects.find((o) => o.id === contextMenu.targetId)?.scale
					))
		]}
		onaction={onObjectAction}
		onclose={closeContext}
	/>

	<!-- Menu contextuel d'un onglet de plateau (MenuFactory::create_plate_menu). -->
	<ContextMenu
		open={plateMenu.open}
		x={plateMenu.x}
		y={plateMenu.y}
		items={plates.list().length > 1
			? PLATE_CONTEXT_ITEMS
			: PLATE_CONTEXT_ITEMS.filter((i) => i === 'separator' || i.action !== 'plate.delete')}
		onaction={onPlateMenuAction}
		onclose={() => (plateMenu = { ...plateMenu, open: false })}
	/>

	<!-- Menu d'ajout (T113) : Load + primitives (plateau vide / bouton add). -->
	<ContextMenu
		open={addMenu.open}
		x={addMenu.x}
		y={addMenu.y}
		items={withHandyMenu(addMenu.scene ? SCENE_CONTEXT_ITEMS : SCENE_ADD_ITEMS, handyModels)}
		onaction={onAddAction}
		onclose={closeAddMenu}
	/>

	<!-- Menu contextuel filament : Edit/Delete de `create_filament_action_menu`. -->
	<ContextMenu
		open={filamentMenu.open}
		x={filamentMenu.x}
		y={filamentMenu.y}
		items={[
			...(filamentSlots > 1
				? FILAMENT_CONTEXT_ITEMS
				: FILAMENT_CONTEXT_ITEMS.filter(
						(item) => item === 'separator' || item.action !== 'filament.delete'
					)),
			...(projectFilament.filament_colour?.[filamentMenu.slot]
				? [{ label: 'Couleur du preset', action: 'filament.resetColour' }]
				: []),
			// Phase 37 — « Fusionner avec » (create_filament_action_menu d'Orca).
			...(filamentSlots > 1 && canRemoveFilamentSlot(activePresets, filamentMenu.slot)
				? [
						'separator' as const,
						{
							label: 'Fusionner avec',
							action: 'filament.mergeMenu',
							items: Array.from({ length: filamentSlots }, (_, i) => i)
								.filter((i) => i !== filamentMenu.slot)
								.map((i) => ({
									label: `Filament ${i + 1}${filamentPresets.find((p) => p.id === filamentAt(activePresets, i))?.name ? ' — ' + filamentPresets.find((p) => p.id === filamentAt(activePresets, i))?.name : ''}`,
									action: `filament.merge:${i}`
								}))
						}
					]
				: [])
		]}
		onaction={(action) => {
			filamentSlot = filamentMenu.slot;
			if (action.startsWith('filament.merge:'))
				mergeFilament(filamentMenu.slot, Number(action.slice('filament.merge:'.length)));
			else if (action === 'filament.edit') openSettingsDialog('filament');
			else if (action === 'filament.delete') removeFilament(filamentMenu.slot);
			else if (action === 'filament.resetColour')
				projectFilament = setSlotColour(projectFilament, filamentMenu.slot, null);
		}}
		onclose={() => (filamentMenu = { ...filamentMenu, open: false })}
	/>

	<!-- Phase 37 — menu du curseur de couches (IMSlider d'Orca). -->
	<ContextMenu
		open={layerMenu.open}
		x={layerMenu.x}
		y={layerMenu.y}
		items={layerMenuItems.map((item, i) => ({ label: item.label, action: `layer.${i}` }))}
		onaction={onLayerMenuAction}
		onclose={() => (layerMenu = { ...layerMenu, open: false })}
	/>
	<Dialog
		open={customGcodeEdit !== null}
		label="G-code personnalisé"
		title="G-code personnalisé"
		size="md"
		onclose={() => (customGcodeEdit = null)}
		data-testid="custom-gcode-dialog"
	>
		{#if customGcodeEdit}
			<label class="flex flex-col gap-1 text-sm">
				<span
					>Entrez le G-code personnalisé à utiliser sur la couche {customGcodeEdit.layer + 1}
					({customGcodeEdit.z.toFixed(2)} mm) :</span
				>
				<textarea
					class="ws-field min-h-32 font-mono text-xs"
					data-testid="custom-gcode-input"
					bind:value={customGcodeEdit.text}
					onkeydown={(e) => {
						e.stopPropagation();
						if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) submitCustomGcode();
					}}></textarea>
				<span class="text-xs text-content-subtle"
					>Inséré au début de la couche ; appliqué au prochain tranchage. Ctrl+Entrée pour valider.</span
				>
			</label>
		{/if}
		{#snippet footer()}
			<button type="button" class="ws-btn ws-btn-secondary" onclick={() => (customGcodeEdit = null)}
				>Annuler</button
			>
			<button
				type="button"
				class="ws-btn ws-btn-primary"
				data-testid="custom-gcode-ok"
				disabled={!customGcodeEdit?.text.trim()}
				onclick={submitCustomGcode}>OK</button
			>
		{/snippet}
	</Dialog>

	<!-- Phase 37 — tableau des paramètres d'objets (« Edit in Parameter Table »). -->
	<ObjectTable
		open={objectTableOpen}
		objects={objectTableObjects}
		inherited={processValues}
		filamentColors={filamentSlotColors.slice(0, filamentSlots)}
		focusId={objectTableFocus}
		onclose={() => (objectTableOpen = false)}
		onsetting={onTableSetting}
		onextruder={onTableExtruder}
		onrename={onTableRename}
		onprintable={(id, printable) => tree.setPrintable(id, printable)}
	/>

	<!-- Suppression d'un preset utilisé par d'autres projets (lot 13). -->
	{#if presetDeleteReport}
		<div
			class="ws-floating fixed bottom-4 left-4 z-40 flex max-w-md flex-col gap-2 p-3 text-sm"
			role="status"
			data-testid="preset-delete-report"
		>
			<div class="flex items-start gap-2">
				<p class="flex-1 font-medium">{presetDeleteReport.title}</p>
				<button
					type="button"
					class="ws-icon-btn ws-icon-btn-xs"
					aria-label="Fermer"
					onclick={() => (presetDeleteReport = null)}
					><IconX class="h-3.5 w-3.5" aria-hidden="true" /></button
				>
			</div>
			<ul class="flex flex-col gap-0.5">
				{#each presetDeleteReport.projects as p (p.id)}
					<li data-testid="preset-delete-project">
						<a class="text-primary hover:underline" href={resolve('/projects/[id]', { id: p.id })}
							>{p.name}</a
						>
					</li>
				{/each}
			</ul>
		</div>
	{/if}

	<!-- Historique PA (menu Calibration) : hors de la colonne, toujours monté. -->
	<PaHistoryDialog
		open={showPaHistory}
		printer={activePresetOf('machine')?.name ?? 'Imprimante'}
		nozzle={String(
			Array.isArray(machineValues.nozzle_diameter)
				? machineValues.nozzle_diameter[0]
				: (machineValues.nozzle_diameter ?? '0.4')
		)}
		filament={activePresetOf('filament')?.name ?? 'Filament'}
		filamentId={((v) => (Array.isArray(v) ? (v[0] ?? null) : typeof v === 'string' ? v : null))(
			filamentValues.filament_id ?? null
		)}
		currentK={((v) => (Number.isFinite(v) ? v : null))(
			Number(
				Array.isArray(filamentValues.pressure_advance)
					? filamentValues.pressure_advance[0]
					: filamentValues.pressure_advance
			)
		)}
		onclose={() => {
			showPaHistory = false;
			paSuggestion = null;
		}}
		onapply={applyPressureAdvance}
		suggestion={paSuggestion}
	/>

	{#if pendingDraft}
		<div
			class="flex items-center justify-between gap-4 bg-warning-soft px-6 py-2 text-sm text-warning-content"
			role="status"
		>
			<span>Un brouillon local plus récent existe pour ce projet.</span>
			<button onclick={dismissDraft} class="font-medium hover:underline">Ignorer</button>
		</div>
	{/if}

	<!-- Barre de messages (phase 30) : notification flottante hors du flux (aucun
	     décalage de la mise en page), absente sans message, estompée après
	     quelques secondes comme les notifications d'Orca. Transparente aux clics
	     (ne masque jamais un bouton) ; le texte reste dans le DOM jusqu'au message
	     suivant (lecteurs d'écran, tests). -->
	{#if notice}
		<div
			class="ws-floating pointer-events-none fixed bottom-4 left-1/2 z-40 max-w-[min(40rem,90vw)] -translate-x-1/2 px-4 py-2 text-sm text-content shadow-md transition-opacity duration-700 {messageFaded
				? 'opacity-0'
				: 'opacity-100'}"
			data-testid="editor-message"
			data-faded={messageFaded ? 'true' : undefined}
			data-seq={notice.seq}
			role="region"
			aria-label="Messages"
			aria-live="polite"
		>
			<!-- Re-monté à chaque notification (même texte) : relu par aria-live. -->
			{#key notice.seq}<span class="ws-notice-in">{notice.text}</span>{/key}
		</div>
	{/if}

	{#if importError}
		<div
			class="flex items-center justify-between bg-danger-soft px-6 py-2 text-sm text-danger-content"
			role="alert"
		>
			<span>{importError}</span>
			<button onclick={() => (importError = null)} class="font-medium hover:underline"
				>Fermer</button
			>
		</div>
	{/if}

	<div class="flex min-h-0 flex-1">
		<!-- Zone gauche : colonne de configuration (objets / réglages). -->
		<aside
			aria-label="Configuration"
			class="flex w-96 flex-col border-r border-border bg-surface-raised"
			class:hidden={preferences.sidebarCollapsed}
		>
			<div class="flex border-b border-border px-1">
				<button
					class="ws-tab flex-1 py-2.5 text-sm {configTab === 'settings' ? 'ws-tab-on' : ''}"
					onclick={() => (configTab = 'settings')}>{$t('Process')}</button
				>
				<button
					class="ws-tab flex-1 py-2.5 text-sm {configTab === 'objects' ? 'ws-tab-on' : ''}"
					onclick={() => (configTab = 'objects')}>Objets</button
				>
				<button
					class="ws-tab flex-1 py-2.5 text-sm {configTab === 'project' ? 'ws-tab-on' : ''}"
					data-testid="config-tab-project"
					aria-label="Fiche projet"
					onclick={() => (configTab = 'project')}>Projet</button
				>
			</div>

			<div class="min-h-0 flex-1 overflow-auto p-3">
				<!-- Aperçu : résultat du tranchage en tête de colonne (Orca garde la liste
				     et affiche les stats de tranchage) ; la configuration reste dessous. -->
				{#if layout.tab === 'preview' && plates.list().length > 1}
					<PreviewPlates
						plates={plates
							.list()
							.map((p) => ({ id: p.id, name: p.name, objects: p.objectIds.length }))}
						activeId={plates.activeId}
						sliced={previewSlicedPlates}
						totals={previewPlatesTotals}
						all={platesStats?.all ?? null}
						currency={preferences.currency}
						onselect={selectPlate}
						onsliceall={session.phase === 'slicing' ? undefined : () => void sliceAll()}
					/>
				{/if}
				{#if layout.tab === 'preview' && session.phase === 'preview' && previewStats}
					<section
						class="mb-4 flex flex-col gap-3 border-b border-border pb-4"
						data-testid="slice-summary"
						aria-label="Résultat du tranchage"
					>
						<div class="flex items-center justify-between gap-2">
							<h2 class="ws-section-title">Résultat du tranchage</h2>
							<div class="flex gap-1.5">
								<button
									type="button"
									class="ws-btn ws-btn-sm ws-btn-secondary"
									disabled={!session.gcodeId}
									onclick={() =>
										session.gcodeId && downloadUrl(`/api/gcodes/${session.gcodeId}/download`)}
									>Exporter</button
								>
								<button
									type="button"
									class="ws-btn ws-btn-sm ws-btn-primary"
									disabled={!session.gcodeId}
									onclick={() => session.gcodeId && (printDialogRequest += 1)}>Envoyer</button
								>
							</div>
						</div>
						<!-- Un seul résumé : simulation Orca (moteur) si disponible, sinon
						     relecture texte du G-code signalée « estimation approchée ». -->
						<EngineSummary
							meta={engineMeta}
							fallback={previewStats}
							mode={engineTimeMode}
							onmode={(m) => (engineTimeMode = m)}
							currency={preferences.currency}
							hidden={new Set(hiddenTypes)}
							ontoggletype={coloration === 'type' ? toggleType : undefined}
							declared={openedDeclaredTime}
						/>
					</section>
				{/if}
				{#if calibReading}
					<CalibrationResultCard
						reading={calibReading}
						onapplyk={applyPressureAdvance}
						onrecordk={(k) => {
							const mode = sceneCalibration?.mode;
							paSuggestion = {
								k,
								method: mode === 'pa_tower' || mode === 'pa_pattern' ? mode : 'pa_line'
							};
							showPaHistory = true;
						}}
						onapplytemp={applyCalibTemperature}
					/>
				{/if}
				{#if configTab === 'objects'}
					<PlateBar
						plates={plates.list()}
						activeId={plates.activeId}
						onselect={selectPlate}
						onadd={() => plates.addPlate()}
						onremove={removePlate}
						ontype={(id, plateType) => plates.setPlateType(id, plateType)}
						onrename={(id, name) => plates.rename(id, name)}
						onsetting={(id, key, value) => plates.setPlateSetting(id, key, value)}
						inherited={plateInherited}
						oncontext={(id, x, y) => (plateMenu = { open: true, x, y, plateId: id })}
						sliced={Object.keys(plateResults)}
						onlock={(id, locked) => plates.setLocked(id, locked)}
						onfront={movePlateToFront}
					/>
				{:else if configTab === 'project'}
					{#if layout.tab === 'project'}
						<!-- Une seule fiche montée à la fois (sinon deux brouillons divergents). -->
						<p class="text-xs text-content-muted" data-testid="project-metadata-elsewhere">
							La fiche du modèle (titre, auteur, licence, description, couverture) est affichée dans
							la page Projet, à droite.
						</p>
					{:else}
						<ProjectMetadataPanel projectId={data.project.id} />
					{/if}
				{:else}
					<div
						class="flex flex-col gap-4 [&>section+section]:border-t [&>section+section]:border-border [&>section+section]:pt-4"
					>
						<!-- Imprimante : imprimantes possédées + buse ; ＋ ajoute, ✎ édite. -->
						<section class="flex flex-col gap-2">
							<span class="ws-section-title">{$t('Printer')}</span>
							{#if ownedPrinters.length === 0}
								<button
									type="button"
									class="ws-btn ws-btn-secondary text-center"
									onclick={() => (showCatalog = true)}>Ajouter une imprimante…</button
								>
							{:else}
								<OwnedPrinterPicker
									printers={ownedPrinters}
									catalog={printerCatalog}
									{machinePresets}
									modified={machineModified}
									bind:selectedMachinePresetId={activePresets.printer}
									onadd={() => (showCatalog = true)}
									onedit={() => openSettingsDialog('machine')}
								/>
							{/if}
							{#if nozzleFlows.length}
								<!-- Phase 37 — débit de buse par extrudeur (Orca ExtruderGroup « Débit »). -->
								<div class="flex flex-wrap items-center gap-2 text-sm" data-testid="nozzle-flow">
									{#each nozzleFlows as choice (choice.index)}
										<label class="flex items-center gap-1">
											<span class="text-content-muted"
												>{nozzleFlows.length > 1
													? `Débit buse ${nozzleFlows.length === 2 ? (choice.index === 0 ? 'gauche' : 'droite') : choice.index + 1}`
													: 'Débit de la buse'}</span
											>
											<select
												class="ws-field ws-field-sm"
												data-testid="nozzle-flow-select"
												data-index={choice.index}
												value={choice.value}
												disabled={choice.options.length < 2}
												onchange={(e) =>
													setExtruderFlow(choice.index, e.currentTarget.value as NozzleFlow)}
											>
												{#each choice.options as option (option)}
													<option value={option}>{NOZZLE_FLOW_LABELS[option]}</option>
												{/each}
											</select>
										</label>
									{/each}
								</div>
							{/if}
						</section>
						<!-- Filament : un slot par filament (slot n ↔ extrudeur n de la liste
						     d’objets), uniquement les presets compatibles avec l’imprimante ;
						     pastille couleur + dropdown + ✎ (slot actif) + retrait. -->
						<section class="flex flex-col gap-2" data-testid="filament-slots">
							<div class="flex items-center justify-between gap-2">
								<span class="ws-section-title">{$t('Filament')}</span>
								<button
									type="button"
									class="ws-btn ws-btn-sm ws-btn-ghost ml-auto"
									data-testid="create-filament"
									title="Créer un filament personnalisé"
									onclick={() => (showCreateFilament = true)}>Créer…</button
								>
								<CreateFilamentDialog
									open={showCreateFilament}
									presets={filamentPresets}
									baseId={filamentAt(activePresets, clampFilamentSlot(activePresets, filamentSlot))}
									onclose={() => (showCreateFilament = false)}
									oncreated={async (created) => {
										const slot = clampFilamentSlot(activePresets, filamentSlot);
										await loadPresets();
										setFilamentSlot(slot, created.id);
										saveMessage = `Filament « ${created.name} » créé et sélectionné.`;
									}}
								/>
								<button
									type="button"
									class="ws-btn ws-btn-sm ws-btn-secondary"
									title="Ajouter un filament"
									aria-label="Ajouter un filament"
									data-testid="add-filament-slot"
									disabled={!activePresets.filaments.length && !filamentPresets.length}
									onclick={addFilament}
								>
									<IconPlus class="h-4 w-4" aria-hidden="true" />
								</button>
							</div>
							{#each Array.from({ length: filamentSlots }, (_, i) => i) as slot (slot)}
								{@const active = slot === clampFilamentSlot(activePresets, filamentSlot)}
								<div
									class="flex items-center gap-1 rounded {active && filamentSlots > 1
										? 'ring-1 ring-primary/60'
										: ''}"
									data-testid="filament-slot"
									data-slot={slot}
									data-active={active ? 'true' : 'false'}
									role="presentation"
									oncontextmenu={(e) => {
										e.preventDefault();
										filamentSlot = slot;
										filamentMenu = { open: true, x: e.clientX, y: e.clientY, slot };
									}}
								>
									<!-- Pastille : numéro de l'extrudeur ; un clic ouvre le sélecteur de
									     couleur (couleur du projet, comme Orca : pas une modification du
									     preset filament). -->
									<label
										class="relative flex h-7 w-6 shrink-0 cursor-pointer items-center justify-center rounded text-xs font-semibold shadow-inner focus-within:ring-2 focus-within:ring-accent"
										style={swatchStyle(filamentSlotColors[slot])}
										title={`Filament ${slot + 1} (extrudeur ${slot + 1}) — cliquer pour changer la couleur`}
										data-testid="filament-swatch"
										data-colour={filamentSlotColors[slot]}
										data-custom={projectFilament.filament_colour?.[slot] ? 'true' : undefined}
									>
										{slot + 1}
										<input
											type="color"
											class="absolute inset-0 h-full w-full cursor-pointer opacity-0"
											aria-label={`Couleur du filament ${slot + 1}`}
											data-testid="filament-colour"
											value={filamentSlotColors[slot]}
											onclick={() => (filamentSlot = slot)}
											onchange={(e) =>
												(projectFilament = setSlotColour(
													projectFilament,
													slot,
													e.currentTarget.value
												))}
										/>
									</label>
									<div class="min-w-0 flex-1">
										{#key presetSelectSeq}
											<PresetSelect
												compact
												presets={filamentPresets}
												printer={machinePresets.find((m) => m.id === activePresets.printer) ?? null}
												testid={slot === 0
													? 'preset-select-filament'
													: `preset-select-filament-${slot + 1}`}
												modified={active && filamentModified}
												bind:selectedId={
													() => filamentAt(activePresets, slot),
													(id) =>
														slot === clampFilamentSlot(activePresets, filamentSlot)
															? requestPresetSwitch('filament', id, () => setFilamentSlot(slot, id))
															: setFilamentSlot(slot, id)
												}
												label={filamentSlots > 1 ? `${$t('Filament')} ${slot + 1}` : $t('Filament')}
											/>
										{/key}
									</div>
									<button
										type="button"
										class="ws-btn ws-btn-secondary shrink-0 justify-center"
										title="Réglages du filament"
										aria-label={filamentSlots > 1
											? `Réglages du filament ${slot + 1}`
											: 'Réglages du filament'}
										data-testid={slot === 0
											? 'open-settings-filament'
											: `open-settings-filament-${slot + 1}`}
										disabled={!filamentAt(activePresets, slot)}
										onclick={() => {
											filamentSlot = slot;
											openSettingsDialog('filament');
										}}
									>
										<IconPencil class="h-4 w-4" aria-hidden="true" />
									</button>
									{#if filamentSlots > 1}
										<button
											type="button"
											class="ws-icon-btn ws-icon-btn-sm"
											title="Retirer ce filament"
											aria-label={`Retirer le filament ${slot + 1}`}
											data-testid="remove-filament-slot"
											disabled={!canRemoveFilamentSlot(activePresets, slot)}
											onclick={() => removeFilament(slot)}
										>
											<IconX class="h-3.5 w-3.5" aria-hidden="true" />
										</button>
									{/if}
								</div>
							{/each}
							{#if filamentCompatMessage}
								<p
									class="rounded border px-2 py-1 text-xs {filamentCompatMessage.blocking
										? 'border-danger/40 bg-danger-soft text-danger-content'
										: 'border-warning/40 bg-warning-soft text-warning-text'}"
									role="status"
									data-testid="filament-compatibility"
								>
									{filamentCompatMessage.text}
									{#if filamentCompatMessage.blocking}
										<button
											type="button"
											class="ws-link ml-1"
											onclick={() => (showPreferences = true)}>Préférences…</button
										>
									{/if}
								</p>
							{/if}
							{#if filamentSlots > 1}
								<!-- Multi-filament : purge entre filaments (config projet) et tour
								     d'essuyage (réglages d'impression, onglet Multimatériau). -->
								<div
									class="flex flex-wrap items-center gap-2 pt-1"
									data-testid="multi-filament-tools"
								>
									<button
										type="button"
										class="ws-btn ws-btn-sm ws-btn-secondary"
										data-testid="open-flush-volumes"
										onclick={() => (flushDialogOpen = true)}
										>Volumes de purge
										{#if projectFilament.flush_volumes_matrix || projectFilament.flush_multiplier}
											<span class="ws-badge h-4 px-1.5">projet</span>
										{/if}</button
									>
									<button
										type="button"
										class="ws-btn ws-btn-sm ws-btn-ghost"
										data-testid="open-prime-tower"
										title="Réglages de la tour d'essuyage (réglages d'impression › Multimatériau)"
										onclick={() => openSettingsDialog('process', 'Multimaterial')}
									>
										Tour d'essuyage :
										<span
											class={primeTowerOn ? 'text-success-content' : 'text-content-muted'}
											data-testid="prime-tower-state"
											>{primeTowerOn
												? `activée · ${Number(processValues.prime_tower_width ?? 35)} mm`
												: 'désactivée'}</span
										>
									</button>
								</div>
								{#if sceneWipeTower}
									<!-- Alternative au glisser sur le plateau (clavier, valeurs exactes). -->
									<div
										class="flex items-center gap-2 text-xs text-content-muted"
										data-testid="wipe-tower-position"
									>
										<span>Position de la tour</span>
										{#each [0, 1] as axis (axis)}
											<label class="flex items-center gap-1">
												<span>{axis === 0 ? 'X' : 'Y'}</span>
												<input
													type="number"
													step="1"
													class="ws-field ws-field-sm w-16 tabular-nums"
													aria-label={`Position ${axis === 0 ? 'X' : 'Y'} de la tour d'essuyage (mm)`}
													data-testid={`wipe-tower-${axis === 0 ? 'x' : 'y'}`}
													value={(axis === 0 ? sceneWipeTower.x : sceneWipeTower.y).toFixed(1)}
													onchange={(e) => {
														const v = Number(e.currentTarget.value);
														if (!Number.isFinite(v) || !sceneWipeTower) return;
														moveWipeTower(
															axis === 0 ? v : sceneWipeTower.x,
															axis === 0 ? sceneWipeTower.y : v
														);
													}}
												/>
											</label>
										{/each}
										<span>mm</span>
										<span
											class="ml-auto text-content-subtle tabular-nums"
											data-testid="wipe-tower-size"
											>{sceneWipeTower.size[0].toFixed(0)} × {sceneWipeTower.size[1].toFixed(0)} mm</span
										>
									</div>
								{/if}
								{#if !primeTowerOn}
									<p
										class="text-xs text-warning-content"
										role="note"
										data-testid="prime-tower-warning"
									>
										Sans tour d'essuyage, la purge des changements de filament se fait dans l'objet
										ou en l'air.
									</p>
								{/if}
							{/if}
						</section>
						<ComparePresetsDialog
							open={compare !== null}
							kind={compare?.kind ?? 'process'}
							presets={compare ? presetsOfKind(compare.kind) : []}
							leftId={compare?.left ?? null}
							rightId={compare?.right ?? null}
							onclose={() => (compare = null)}
						/>
						<UnsavedPresetDialog
							open={pendingSwitch !== null}
							kindLabel={pendingSwitch?.kind === 'filament'
								? 'filament'
								: pendingSwitch?.kind === 'machine'
									? "d'imprimante"
									: "d'impression"}
							fromName={pendingSwitch ? (activePresetOf(pendingSwitch.kind)?.name ?? '') : ''}
							toName={pendingSwitch?.toName ?? ''}
							changes={pendingSwitch?.changes ?? []}
							ontransfer={() => {
								const p = pendingSwitch;
								if (!p) return;
								pendingTransfer[p.kind] = changedPresetValues(
									resolvedForKind(p.kind),
									valuesForKind(p.kind)
								);
								pendingSwitch = null;
								p.apply();
							}}
							ondiscard={() => {
								const p = pendingSwitch;
								pendingSwitch = null;
								p?.apply();
							}}
							oncancel={() => {
								pendingSwitch = null;
								presetSelectSeq++;
							}}
						/>
						<FlushVolumesDialog
							open={flushDialogOpen}
							colours={filamentSlotColors}
							matrix={projectFilament.flush_volumes_matrix}
							multiplier={projectFilament.flush_multiplier ?? presetFlushMultiplier}
							defaultMultiplier={presetFlushMultiplier}
							onclose={() => (flushDialogOpen = false)}
							compute={async (colours, multiplier) =>
								(await computeFlushVolumes(colours, multiplier, data.project.id)).matrix}
							computeAvailable={flushVolumesAvailable}
							onapply={(matrix, multiplier) =>
								(projectFilament = {
									...projectFilament,
									flush_volumes_matrix: matrix,
									flush_multiplier: multiplier
								})}
						/>
						<!-- Traitement : dropdown + réglages en ligne (onglets), sans bouton d’enregistrement. -->
						<section class="flex flex-col gap-2">
							<div class="flex items-center justify-between gap-2">
								<span class="ws-section-title">{$t('Process')}</span>
								<div class="flex gap-0.5 rounded border border-border-strong p-0.5 text-xs">
									<span class="rounded bg-primary px-2 py-0.5 text-primary-content">Global</span>
									<span
										class="rounded px-2 py-0.5 text-content-subtle"
										title="Réglages par objet (à venir)">Objets</span
									>
								</div>
							</div>
							{#key presetSelectSeq}
								<PresetSelect
									compact
									presets={processPresets}
									searchable
									printer={machinePresets.find((m) => m.id === activePresets.printer) ?? null}
									testid="preset-select-print"
									modified={processModified}
									bind:selectedId={
										() => activePresets.process,
										(id) => requestPresetSwitch('process', id, () => (activePresets.process = id))
									}
									label={$t('Process')}
								/>
							{/key}
							<button
								type="button"
								class="ws-btn ws-btn-secondary self-end"
								title="Réglages d'impression"
								aria-label="Réglages d'impression"
								data-testid="open-settings-print"
								disabled={!activePresets.process}
								onclick={() => openSettingsDialog('process')}
							>
								<IconPencil class="h-4 w-4" aria-hidden="true" />
							</button>
						</section>
						<!-- Dialogues de réglages (ouverts par les crayons) : éditer un preset système
						     puis Enregistrer dérive automatiquement une copie utilisateur.
						     Le conteneur note les clés saisies (cf. `recordSettingEdit`). -->
						<div
							class="contents"
							oninputcapture={recordSettingEdit}
							onchangecapture={recordSettingEdit}
						>
							<PresetSettingsDialog
								open={settingsDialog === 'process'}
								title="Réglages d'impression"
								kind="process"
								initialPage={settingsInitialPage}
								focusKey={settingsFocusKey}
								savedByKind={{
									process: processResolvedValues,
									filament: filamentResolvedValues,
									machine: machineResolvedValues
								}}
								systemByKind={systemValues}
								preset={activePresetOf('process') ?? null}
								onrename={(name) => renameActivePreset('process', name)}
								ondelete={() => deleteActivePreset('process')}
								oncompare={() => openCompare('process')}
								ondetach={(name) => detachActivePreset('process', name)}
								bind:mode={settingsMode}
								bind:values={processValues}
								valuesByKind={{
									process: processValues,
									filament: filamentValues,
									machine: machineValues
								}}
								onClose={() => (settingsDialog = null)}
								onsave={saveSettingsDialog}
								bind:saveAsName
								{saveAsError}
							/>
							<PresetSettingsDialog
								open={settingsDialog === 'filament'}
								title={filamentSlots > 1
									? `Réglages du filament ${clampFilamentSlot(activePresets, filamentSlot) + 1}`
									: 'Réglages du filament'}
								kind="filament"
								initialPage={settingsInitialPage}
								focusKey={settingsFocusKey}
								savedByKind={{
									process: processResolvedValues,
									filament: filamentResolvedValues,
									machine: machineResolvedValues
								}}
								systemByKind={systemValues}
								preset={activePresetOf('filament') ?? null}
								onrename={(name) => renameActivePreset('filament', name)}
								ondelete={() => deleteActivePreset('filament')}
								oncompare={() => openCompare('filament')}
								ondetach={(name) => detachActivePreset('filament', name)}
								bind:mode={settingsMode}
								bind:values={filamentValues}
								valuesByKind={{
									process: processValues,
									filament: filamentValues,
									machine: machineValues
								}}
								onClose={() => (settingsDialog = null)}
								onsave={saveSettingsDialog}
								bind:saveAsName
								{saveAsError}
							/>
							<PresetSettingsDialog
								open={settingsDialog === 'machine'}
								title="Paramètres de l'imprimante"
								kind="machine"
								focusKey={settingsFocusKey}
								savedByKind={{
									process: processResolvedValues,
									filament: filamentResolvedValues,
									machine: machineResolvedValues
								}}
								systemByKind={systemValues}
								preset={activePresetOf('machine') ?? null}
								onrename={(name) => renameActivePreset('machine', name)}
								ondelete={() => deleteActivePreset('machine')}
								oncompare={() => openCompare('machine')}
								ondetach={(name) => detachActivePreset('machine', name)}
								bind:mode={settingsMode}
								bind:values={machineValues}
								valuesByKind={{
									process: processValues,
									filament: filamentValues,
									machine: machineValues
								}}
								onClose={() => (settingsDialog = null)}
								onsave={saveSettingsDialog}
								bind:saveAsName
								{saveAsError}
							/>
						</div>
					</div>
				{/if}
				<div class="mt-4 border-t border-border pt-4">
					{#if tree.list().some((n) => n.kind === 'object')}
						<!-- Phase 37 — Orca ParamsPanel « View all object's settings » :
						     tableau des paramètres de tous les objets. -->
						<div class="mb-1 flex justify-end">
							<button
								type="button"
								class="ws-btn ws-btn-sm ws-btn-ghost"
								data-testid="open-object-table"
								data-tip="Voir et modifier les réglages de tous les objets dans un tableau"
								onclick={() => openObjectTable(selId)}>Table des paramètres…</button
							>
						</div>
					{/if}
					<ObjectList
						meshWarnings={meshWarningsByObject}
						onrepair={(id) => void repairFromWarning(id)}
						{cutLinks}
						{tree}
						selection={ws.selection}
						visibleIds={activeObjectIds}
						plates={plates.list()}
						onassign={assignObjectToPlate}
						onselect={selectObject}
						ontogglelock={(id) => tree.setLocked(id, !tree.isLocked(id))}
						ontogglehide={(id) => tree.setHidden(id, !tree.isHidden(id))}
						onextruder={(id, extruder) => tree.setExtruder(id, extruder)}
						extruderCount={filamentSlots}
						extruderColors={filamentSlotColors}
						onduplicate={duplicateObject}
						ondelete={removeObject}
						ongroup={() => tree.group([...ws.selection])}
						oncontext={openContext}
						onrename={(id, name) => tree.rename(id, name)}
						bind:editingId={renamingObjectId}
						overrides={objectOverrideCounts}
						onreorder={reorderObject}
						sequential={sequentialPrint}
						bind:showOrderOnScene={showPrintOrder}
						issues={objectIssues}
						partNames={objectPartNames}
						onlocate={(plateId, id) => {
							selectPlate(plateId);
							selectObject(id, false);
						}}
					/>
					{#if selId && (volumesByObject[selId]?.length ?? 0) > 0}
						<ul class="mt-2 flex flex-col gap-1 text-sm" data-testid="object-volumes">
							<!-- Objet multi-volumes : son maillage propre est le 1er volume (Orca
							     `ObjectList` liste tous les volumes dès qu'il y en a plus d'un). -->
							<li
								class="flex items-center gap-2 rounded px-2 py-0.5 hover:bg-surface-sunken"
								data-testid="object-volume"
								data-type="part"
								data-main="true"
							>
								<span class="text-content-subtle">↳</span>
								<button
									type="button"
									class="flex-1 truncate text-left {selectedVolumeId === null
										? 'font-semibold text-accent'
										: ''}"
									title={volumeTypeName('part')}
									data-testid="object-volume-main"
									aria-pressed={selectedVolumeId === null}
									onclick={() => (selectedVolumeId = null)}
									>{tree.get(selId)?.meshName ?? tree.get(selId)?.name}</button
								>
							</li>
							{#each volumesByObject[selId] as volume (volume.id)}
								<li
									class="flex items-center gap-2 rounded px-2 py-0.5 hover:bg-surface-sunken"
									data-testid="object-volume"
									data-type={volume.type}
									oncontextmenu={(e) => {
										if (!selId) return;
										e.preventDefault();
										openVolumeContext(selId, volume.id, e.clientX, e.clientY);
									}}
								>
									<span class="text-content-subtle">↳</span>
									<button
										type="button"
										class="flex-1 truncate text-left {selectedVolumeId === volume.id
											? 'font-semibold text-accent'
											: ''}"
										title={volumeTypeName(volume.type)}
										data-testid="object-volume-select"
										aria-pressed={selectedVolumeId === volume.id}
										onclick={() => (selectedVolumeId = volume.id)}>{volume.name}</button
									>
									<span class="text-xs text-content-subtle">{volume.size.join('×')}</span>
									{#if filamentSlots > 1 && (volume.type === 'part' || volume.type === 'modifier')}
										<!-- Extrudeur propre du volume (Orca : pièces et modificateurs) ;
										     « objet » = hérite de l'extrudeur de l'objet. -->
										{@const vx = volume.extruder ?? 0}
										<span
											class="h-3 w-3 shrink-0 rounded-full ring-1 ring-border-strong"
											style:background-color={filamentSlotColors[
												(vx || tree.effectiveExtruder(selId) || 1) - 1
											] ?? 'transparent'}
											aria-hidden="true"
										></span>
										<select
											class="ws-field ws-field-sm h-6 px-0.5"
											aria-label={`Extrudeur de ${volume.name}`}
											data-testid="object-volume-extruder"
											value={vx}
											onchange={(e) => {
												const v = Number(e.currentTarget.value);
												if (selId)
													updateVolume(selId, volume.id, { extruder: v > 0 ? v : undefined });
											}}
										>
											<option value={0}>objet</option>
											{#each Array.from({ length: filamentSlots }, (_, i) => i + 1) as ex (ex)}
												<option value={ex}>{ex}</option>
											{/each}
										</select>
									{/if}
									<button
										type="button"
										class="px-1 text-xs text-content-muted hover:text-content"
										aria-label={`Supprimer ${volume.name}`}
										onclick={() => selId && removeVolume(selId, volume.id)}
										><IconX class="h-3.5 w-3.5" aria-hidden="true" /></button
									>
								</li>
							{/each}
						</ul>
					{/if}
					{#if selId && tree.get(selId)?.kind === 'object'}
						{@const objectId = selId}
						{@const volume = (volumesByObject[objectId] ?? []).find(
							(v) => v.id === selectedVolumeId
						)}
						<div class="mt-2 flex flex-col gap-2">
							{#if volume}
								<!-- Panneau de transformation existant : position (repère de l'objet),
								     rotation propre, échelle (× taille du pavé). -->
								<div data-testid="volume-transform">
									<TransformPanel
										transform={volumeTransform(volume)}
										onchange={(t) => replaceVolume(objectId, applyVolumeTransform(volume, t))}
									/>
								</div>
								<div class="grid grid-cols-4 items-center gap-1 text-xs" data-testid="volume-size">
									<span>Taille</span>
									{#each [0, 1, 2] as axis (axis)}
										<input
											type="number"
											step="0.1"
											min="0.1"
											class="ws-field ws-field-sm w-full"
											aria-label={`Taille ${'XYZ'[axis]} de ${volume.name}`}
											value={volume.size[axis]}
											onchange={(e) => {
												const n = Number(e.currentTarget.value);
												if (!(n > 0)) return;
												const size = [...volume.size] as [number, number, number];
												size[axis] = n;
												updateVolume(objectId, volume.id, { size });
											}}
										/>
									{/each}
								</div>
								{#if volume.type === 'modifier' || volume.type === 'part'}
									<ObjectSettingsPanel
										title={`Réglages de ${volume.name}`}
										settings={volume.settings}
										isPart
										mode={settingsMode}
										base={processValues}
										filamentCount={filamentSlots}
										testid="volume-settings-panel"
										onchange={(settings) => updateVolume(objectId, volume.id, { settings })}
									/>
								{/if}
							{:else}
								<ObjectSettingsPanel
									title={`Réglages de ${tree.get(objectId)?.name ?? 'l’objet'}`}
									settings={tree.get(objectId)?.settings ?? {}}
									mode={settingsMode}
									base={processValues}
									filamentCount={filamentSlots}
									bind:adding={objectSettingsAdding}
									onchange={(settings) => setObjectSettings(objectId, settings)}
								/>
								{#if (layerRangesByObject[objectId]?.length ?? 0) > 0}
									<LayerRangesPanel
										ranges={layerRangesByObject[objectId]}
										layerHeight={Number(processValues.layer_height ?? 0.2)}
										mode={settingsMode}
										base={processValues}
										onchange={(ranges) =>
											(layerRangesByObject = { ...layerRangesByObject, [objectId]: ranges })}
									/>
								{/if}
							{/if}
						</div>
					{/if}
				</div>
			</div>
		</aside>

		<!-- Zone centrale : scène 3D (Préparer) / aperçu G-code (Aperçu) / autres vues. -->
		<main
			class="relative min-w-0 flex-1 bg-surface-sunken"
			ondragover={(e) => {
				if (layout.tab === 'prepare') {
					e.preventDefault();
					dragOver = true;
				}
			}}
			ondragleave={() => (dragOver = false)}
			ondrop={onDrop}
			oncontextmenu={onSceneContext}
			onwheel={onSceneWheel}
		>
			{#if layout.tab === 'prepare' && overlapCount > 0}
				<!-- Imports empilés au centre comme dans Orca : on le signale au lieu de
				     déplacer les objets ; « Disposer » les répartit. -->
				<div
					class="ws-floating absolute bottom-3 left-3 z-20 flex items-center gap-2 px-3 py-2 text-xs text-content"
					role="status"
					data-testid="overlap-warning"
				>
					<span class="h-2 w-2 shrink-0 rounded-full bg-warning" aria-hidden="true"></span>
					<span>{overlapCount} objet(s) superposé(s) sur le plateau.</span>
					<button
						type="button"
						class="ws-btn ws-btn-sm ws-btn-secondary"
						data-testid="overlap-arrange"
						onclick={() => void arrangeActive()}>Disposer</button
					>
				</div>
			{/if}
			{#if layout.tab === 'prepare'}
				<!-- Phase 29 : barre de vérification (Print::validate serveur), en bas à
				     droite de la vue comme les notifications d'Orca. -->
				<div class="absolute right-3 bottom-3 z-20">
					<ValidationPanel
						notices={validationNotices}
						busy={validationBusy}
						blocked={validationBlocked}
						activePlate={plates.plateIndex(plates.activeId ?? '')}
						ongotoobject={gotoIssueObject}
						ongotooption={gotoIssueOption}
						ongotoplate={gotoIssuePlate}
						onarrange={() => void arrangeActive()}
					/>
				</div>
			{/if}
			{#if layout.tab === 'prepare'}
				<!-- Phase 28 : double-clic sur un texte / SVG en relief → ré-édition. -->
				{#if $appPreferences.show3dNavigator}
					<!-- Phase 37 — navigateur 3D (Orca : cube en bas à gauche de la vue). -->
					<div class="absolute bottom-3 left-3 z-10">
						<ViewCube forward={navForward} onselect={(v) => void selectNamedView(v)} />
					</div>
				{/if}
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div class="contents" ondblclick={() => openEmbossEdit(selId)}>
					{#await sceneModule then { default: SceneView }}
						<SceneView
							{bed}
							objects={visibleSceneObjects}
							bind:selection={ws.selection}
							gizmoMode={activeGizmoMode}
							view={cameraView}
							zoom={cameraZoom}
							focus={cameraFocus}
							{showGrid}
							{projection}
							{overhangNormalZ}
							labels={objectLabels}
							printOrder={scenePrintOrder}
							issueObjects={sceneIssueObjects}
							extruderColors={filamentSlotColors}
							paintOverlays={scenePaintOverlays}
							objectExtruders={sceneObjectExtruders}
							wipeTower={layout.tab === 'prepare' ? sceneWipeTower : null}
							onmovewipetower={moveWipeTower}
							{selectedVolumeId}
							onpickvolume={pickVolume}
							onmeshpick={onMeshPick}
							paintstroke={!!paintChannel}
							paintAssist={scenePaintAssist}
							hoverPreviews={sceneHoverPreviews}
							realistic={realisticView}
							suspendcontrols={paintDragging}
							flattenMode={tools.active === 'flatten'}
							onflatten={flattenOnFace}
							ontransform={onTransform}
							onvolumechange={replaceVolume}
							ondragchange={onGizmoDragChange}
							{sectionRatio}
							clipPlane={cutClip ?? sectionClip}
							guides={sceneGuides}
							oncameradir={(forward) => {
								cameraForward = forward;
								navForward = forward;
							}}
							onpickobject={() => (selectedVolumeId = null)}
							otherPlates={plateViews}
							activePlate={activePlateView}
							onactivateplate={activatePlateFromScene}
							onobjectdrop={dropObjectOnPlate}
							recenterKey={plates.activeId}
						/>
					{/await}
				</div>

				<!-- Barre d'outils de plateau (T106) : horizontale, en haut au centre,
				     précédée d'Annuler / Rétablir avec la liste des étapes (Orca) — le coin
				     de la vue reste libre pour le clic droit sur le plateau. -->
				<div class="absolute top-3 left-1/2 z-10 flex -translate-x-1/2 items-start gap-2">
					<UndoHistory
						entries={() => sceneHistory.entries()}
						version={historyVersion}
						onflush={flushHistory}
						onundo={undoSteps}
						onredo={redoSteps}
					/>
					<PlateToolbar
						active={plateToolActive}
						hasSelection={selId !== null}
						hasObjects={activeSceneObjects.length > 0}
						onaction={onPlateAction}
					/>
				</div>

				<!-- Éditeur de hauteur de couche variable (T106, item layersediting). -->
				{#if showLayerHeight}
					<div class="ws-floating absolute top-16 right-3 z-10 w-72 p-3">
						<div class="ws-floating-header">
							<span class="ws-section-title">{$t('Variable layer height')}</span>
							<button
								type="button"
								class="ws-icon-btn ws-icon-btn-sm"
								onclick={() => (showLayerHeight = false)}
								aria-label={$t('Close')}><IconX class="h-4 w-4" aria-hidden="true" /></button
							>
						</div>
						{#if selId}
							<LayerHeight
								profile={activeLayerProfile}
								zMax={activeZMax}
								onchange={setLayerProfile}
								onadaptive={(quality) => engineLayerProfile({ kind: 'adaptive', quality })}
								onsmooth={(radius, keep_min) =>
									engineLayerProfile({ kind: 'smooth', radius, keep_min })}
								onbrush={engineLayerBrush}
							/>
						{:else}
							<p class="text-sm text-content-subtle">Sélectionnez un objet.</p>
						{/if}
					</div>
				{/if}

				<!-- Panneau de l'outil actif (T104) : flottant, agit sur l'objet sélectionné.
				     Outils de transformation (Déplacer, actif par défaut) : panneau masqué sans
				     sélection, comme les gizmos d'Orca (GLGizmosManager ferme le gizmo quand la
				     sélection est vide) — sinon il recouvre le plateau vide et intercepte ses clics. -->
				{#if tools.active && (activeObject || !isTransformTool(tools.active))}
					<div
						class="ws-floating absolute top-16 left-3 z-10 w-[19rem] p-3"
						role="region"
						aria-label={$t('Tools')}
					>
						<div class="ws-floating-header">
							<span class="ws-section-title">{$t('Tools')}</span>
							<button
								type="button"
								class="ws-icon-btn ws-icon-btn-sm"
								onclick={closeTool}
								aria-label={$t('Close')}><IconX class="h-4 w-4" aria-hidden="true" /></button
							>
						</div>
						<!-- Booléen sans sélection unique ; Emboss/SVG peuvent créer un objet en relief sans sélection. -->
						{#if !activeObject && !['assembly', 'boolean', 'emboss', 'svg'].includes(tools.active)}
							<p class="text-sm text-content-subtle">Sélectionnez un objet.</p>
						{:else if isTransformTool(tools.active) || tools.active === 'flatten'}
							<TransformPanel
								transform={activeTransform}
								onchange={(tr) => selId && onTransform(selId, tr)}
								canLayFlat={tools.active === 'flatten'}
								onlayflat={layActiveFlat}
							/>
						{:else if tools.active === 'cut'}
							<CutTool
								session={cutSession}
								busy={meshOpBusy}
								oncut={() => cutObject(activeObject?.id ?? null)}
								oncancel={closeTool}
							/>
						{:else if tools.active === 'boolean'}
							<BooleanTool
								available={!!booleanOperands}
								busy={meshOpBusy}
								hint="Sélectionnez deux objets prêts (A puis B, Ctrl+clic)."
								onapply={booleanSelected}
							/>
						{:else if tools.active === 'simplify'}
							{@const simplifyModelId = imports.find(
								(i) => i.objectId === activeObject?.id
							)?.modelId}
							<SimplifyTool
								onsimplify={(param) => activeObject && simplifyObject(activeObject.id, param)}
								busy={!!simplifyModelId && repairingModels.has(simplifyModelId)}
								triangles={activeObject ? activeObject.mesh.indices.length / 3 : null}
								oncancel={closeTool}
							/>
						{:else if tools.active === 'brim-ears'}
							<BrimEarsTool
								points={activeObject ? (brimEarsByObject[activeObject.id]?.list() ?? []) : []}
								onremove={removeBrimEar}
								onclear={() => {
									if (activeObject) {
										brimEarsByObject[activeObject.id]?.clear();
										brimEarsByObject = { ...brimEarsByObject };
									}
								}}
							/>
						{:else if tools.active === 'measure'}
							<MeasureTool {measurement} onclear={() => (measuredPoints = [])} />
						{:else if tools.active === 'emboss'}
							{#key embossEdit}
								<EmbossTool
									available
									fonts={embossFonts}
									busy={embossBusy}
									canEngrave={!!selId}
									initial={embossEdit?.params ?? null}
									initialSource={embossEdit?.params.source ?? 'text'}
									oncancel={closeTool}
									onsubmit={embossEdit ? applyEmbossEdit : createEmboss}
								/>
							{/key}
						{:else if tools.active === 'svg'}
							<EmbossTool
								available
								initialSource="svg"
								fonts={embossFonts}
								busy={embossBusy}
								canEngrave={!!selId}
								onsubmit={createEmboss}
							/>
						{:else if tools.active === 'assembly'}
							<AssemblyView
								bind:exploded={assemblyExploded}
								bind:factor={assemblyFactor}
								bind:mode={assemblyMode}
								picks={assemblyPick ? 1 : 0}
							/>
						{:else if paintChannel}
							<!-- Barre de peinture (T105) : canal verrouillé sur l'outil du rail. -->
							<PaintToolbar
								channel={paintChannel}
								bind:tool={paintTool}
								bind:radius={paintRadius}
								bind:state={paintState}
								bind:height={paintHeight}
								bind:fillAngle={paintFillAngle}
								bind:edgeDetection={paintEdgeDetection}
								bind:gapArea={paintGapArea}
								bind:constraint={paintConstraint}
								busy={paintBusy}
								ongapfill={performGapFill}
								onremap={remapPaintFilaments}
								filamentColours={filamentSlotColors}
							/>
							<div class="mt-3 flex items-center justify-between text-xs text-content-subtle">
								<span>{activePaintedCount} triangle(s) peint(s)</span>
								<button
									type="button"
									class="ws-btn ws-btn-sm ws-btn-secondary"
									data-testid="paint-frame-selection"
									data-tip="Recadrer la vue sur l'objet sélectionné"
									onclick={frameSelection}>Cadrer l'objet</button
								>
								<button
									class="ws-btn ws-btn-sm ws-btn-secondary"
									onclick={clearActivePaint}
									disabled={activePaintedCount === 0}>Effacer</button
								>
							</div>
						{/if}
					</div>
				{/if}

				{#if showDropHint(activeSceneObjects.length, imports, activePlate?.objectIds ?? [])}
					<div class="pointer-events-none absolute inset-0 flex items-center justify-center p-6">
						<div
							class="ws-floating flex max-w-sm flex-col items-center gap-2 border-dashed px-6 py-5 text-center text-sm text-content-muted"
						>
							{#if queuedImports.length || imports.some(isPending)}
								{@const name = queuedImports[0] ?? imports.find(isPending)?.filename}
								<span
									class="inline-block size-6 animate-spin rounded-full border-2 border-primary border-t-transparent"
									aria-hidden="true"
								></span>
								<p role="status" data-testid="import-loading">
									Import de <span class="font-medium text-content">{name}</span>…
								</p>
							{:else}
								<IconFileUp class="h-6 w-6 text-primary" aria-hidden="true" />
								<p>
									Glissez un modèle ici (STL, OBJ, 3MF, STEP, AMF, SVG, DRC)<br />ou cliquez sur
									<span class="font-medium text-content">{$t('Import')}</span>.
								</p>
							{/if}
						</div>
					</div>
				{/if}

				{#if dragOver}
					<div
						class="pointer-events-none absolute inset-0 z-20 flex items-center justify-center border-2 border-dashed border-accent bg-primary/10 text-sm font-medium text-primary"
					>
						Déposez pour importer
					</div>
				{/if}

				{#if imports.some((i) => isPending(i) || i.status === 'failed')}
					<div class="absolute bottom-3 left-3 z-10 flex flex-col gap-1">
						{#each imports.filter((i) => isPending(i) || i.status === 'failed') as it (it.objectId)}
							<div
								class="flex items-center gap-1.5 rounded px-2 py-1 text-xs {it.status === 'failed'
									? 'bg-danger-soft text-danger-content'
									: 'bg-overlay text-content-muted'}"
								data-testid="import-status"
								data-status={it.status}
							>
								{#if it.status === 'failed'}
									<span aria-hidden="true">⚠</span>
								{:else}
									<span
										class="inline-block size-3 animate-spin rounded-full border-2 border-current border-t-transparent"
										aria-hidden="true"
									></span>
								{/if}
								<span>{it.filename} — {importLabel(it)}</span>
							</div>
						{/each}
					</div>
				{/if}
			{:else if layout.tab === 'preview'}
				<div
					class="flex h-full flex-col"
					data-testid="preview-view"
					data-layer-count={previewMeta?.layer_count}
				>
					<!-- Avertissements du tranchage (ex. scripts de post-traitement jamais
					     exécutés sur le serveur) : restent visibles avec l'aperçu. -->
					{#if session.phase !== 'slicing' && session.warnings.length}
						<div
							class="flex flex-col gap-1 border-b border-warning/40 bg-warning-soft px-4 py-2 text-sm text-warning-content"
							role="status"
							data-testid="slice-warnings"
						>
							{#each session.warnings as w (w.key)}
								<p data-key={w.key}>⚠ {w.message}</p>
							{/each}
						</div>
					{/if}
					{#if session.phase === 'slicing'}
						<div
							class="flex flex-1 flex-col items-center justify-center gap-3 p-6 text-content-muted"
							data-testid="slice-progress"
						>
							<p>Tranchage en cours… ({session.jobPhase})</p>
							<div class="h-2 w-64 overflow-hidden rounded bg-overlay">
								<div
									class="h-full bg-primary transition-all"
									style="width: {Math.round(session.progress * 100)}%"
								></div>
							</div>
							{#each session.warnings as w (w.key)}
								<p class="text-sm text-warning">⚠ {w.message}</p>
							{/each}
						</div>
					{:else if session.phase === 'error'}
						<div class="flex flex-1 items-center justify-center p-6">
							<div
								class="flex max-w-xl flex-col gap-3 rounded-lg border border-danger/40 bg-danger-soft px-5 py-4 text-sm text-danger-content shadow-sm"
								role="alert"
								data-testid="slice-error"
							>
								<p>
									{session.error && session.error !== 'échec du tranchage'
										? `Échec du tranchage : ${session.error}`
										: 'Échec du tranchage. Vérifiez l’imprimante, le filament et le profil de traitement, puis réessayez.'}
								</p>
								<div class="flex flex-wrap gap-2">
									<button
										type="button"
										class="ws-btn ws-btn-sm ws-btn-secondary"
										onclick={() => showTab('prepare')}>Retour à la préparation</button
									>
									{#if session.error?.includes('trop près des autres objets')}
										<button
											type="button"
											class="ws-btn ws-btn-sm ws-btn-primary"
											data-testid="slice-error-arrange"
											onclick={async () => {
												showTab('prepare');
												await arrangeActive();
											}}>Disposer les objets</button
										>
									{/if}
								</div>
							</div>
						</div>
					{:else if session.phase === 'preview' && previewGeometry && previewMeta}
						<!-- Attributs de test (contrat e2e) : segments rendus, plage, coloration. -->
						<div
							class="relative min-h-0 flex-1"
							data-testid="preview-canvas"
							data-visible-segments={previewGeometry.visibleCount}
							data-layer-low={layerRange.oneLayerMode ? layerRange.high : layerRange.low}
							data-layer-high={layerRange.high}
							data-coloration={coloration}
							data-colors={previewColors(previewGeometry).join(' ')}
							data-move-cursor={moveCursor.position}
							data-render-mode={effectiveRender}
							onwheel={onPreviewWheel}
						>
							{#await loadPreviewScene() then { default: PreviewView }}
								<PreviewView
									geometry={previewGeometry}
									{bed}
									overlay={previewOverlay}
									frame={previewFrame}
									fit={previewFit}
									frameKey={previewFrameKey}
									renderMode={effectiveRender}
									tool={previewTool?.position ?? null}
									view={previewNamedView}
									viewKey={previewViewKey}
									oncameradir={(forward) => (previewForward = forward)}
								/>
							{/await}
							{#if $appPreferences.show3dNavigator}
								<!-- Phase 38 — navigateur 3D aussi en Aperçu (Orca ViewManipulate, en bas
								     à gauche) ; à droite quand la fenêtre G-code occupe ce coin. -->
								<div
									class="absolute bottom-16 z-10 {showGcodeWindow && gcodeLines.length > 0
										? 'right-32'
										: 'left-3'}"
									data-testid="preview-view-cube"
								>
									<ViewCube
										forward={previewForward}
										onselect={(v) => {
											previewNamedView = v;
											previewViewKey++;
										}}
									/>
								</div>
							{/if}
							{#if showGcodeWindow && gcodeLines.length > 0}
								<div class="absolute bottom-16 left-3 z-20">
									<GcodeWindow lines={gcodeLines} layer={layerRange.high} />
								</div>
							{/if}

							{#if previewTool}
								<!-- Position de l'outil (fenêtre ToolPosition d'Orca, bas de la vue). -->
								<div
									class="absolute bottom-16 left-1/2 z-10 w-max max-w-[calc(100%-16rem)] -translate-x-1/2"
								>
									<ToolPositionCard tool={previewTool} {coloration} time={previewToolTime} />
								</div>
							{/if}
							<!-- Curseur horizontal intra-couche (←/→, Home/End). -->
							<div
								class="ws-floating absolute right-30 bottom-3 left-3 z-10 flex items-center gap-2 px-3 py-2 text-xs text-content-muted"
							>
								<span>Déplacements</span>
								<input
									type="range"
									class="min-w-0 flex-1"
									min="0"
									max={moveCursor.total}
									value={moveCursor.position}
									aria-label="Progression dans la couche"
									oninput={(e) => {
										moveCursor = { ...moveCursor, position: Number(e.currentTarget.value) };
										void refreshGeometry();
									}}
								/>
								<span class="tabular-nums">{moveCursor.position}/{moveCursor.total}</span>
								{#if previewToolTime}
									<!-- Temps estimé à la position de l'outil (simulation Orca). -->
									<span
										class="tabular-nums text-content"
										data-testid="preview-move-time"
										data-tip="Temps estimé écoulé à la position de l’outil"
										data-tip-side="top">{formatDuration(previewToolTime.time)}</span
									>
								{/if}
							</div>

							<!-- Sélecteur de coloration (T116, FR-041). -->
							<div
								class="ws-floating absolute left-3 top-3 flex items-center gap-2 p-2 text-xs text-content-muted"
							>
								<span>Couleur</span>
								<select
									class="ws-field ws-field-sm"
									bind:value={coloration}
									onchange={refreshGeometry}
									aria-label="Coloration de l'aperçu"
								>
									{#each COLORATIONS as c (c.id)}
										<option value={c.id}>{c.label}</option>
									{/each}
								</select>
								<span class="ml-1">Cadrer</span>
								<div class="ws-seg" role="group" aria-label="Cadrage de la vue">
									<button
										type="button"
										class="ws-seg-item {previewFit === 'objects' ? 'ws-seg-on' : ''}"
										aria-pressed={previewFit === 'objects'}
										data-testid="preview-fit-objects"
										data-tip="Zoom sur les objets imprimés"
										data-tip-side="bottom"
										onclick={() => setPreviewFit('objects')}>Objets</button
									>
									<button
										type="button"
										class="ws-seg-item {previewFit === 'bed' ? 'ws-seg-on' : ''}"
										aria-pressed={previewFit === 'bed'}
										data-testid="preview-fit-bed"
										data-tip="Vue de tout le plateau"
										data-tip-side="bottom"
										onclick={() => setPreviewFit('bed')}>Plateau</button
									>
								</div>
								<span class="ml-1">Rendu</span>
								<div class="ws-seg" role="group" aria-label="Rendu des extrusions">
									{#each [['auto', 'Auto'], ['volumes', 'Volumes'], ['lines', 'Lignes']] as [id, label] (id)}
										<button
											type="button"
											class="ws-seg-item {renderMode === id ? 'ws-seg-on' : ''}"
											aria-pressed={renderMode === id}
											data-testid={`preview-render-${id}`}
											data-tip={id === 'auto'
												? softwareGl
													? 'Rendu WebGL logiciel détecté (pas de GPU) : lignes'
													: `Volumes jusqu'à ${VOLUME_SEGMENT_LIMIT.toLocaleString('fr-FR')} segments, lignes au-delà`
												: id === 'volumes'
													? 'Cordons pleins éclairés (largeur × hauteur réelles)'
													: 'Lignes fines (rapide, gros fichiers)'}
											data-tip-side="bottom"
											onclick={() => (renderMode = id as RenderMode)}>{label}</button
										>
									{/each}
								</div>
							</div>

							<!-- Double curseur vertical de couches (T116, sliders.ts) : pleine hauteur,
							     numéro et Z des deux poignées, temps de couche, ± sur la poignée active. -->
							{#if previewMeta.layer_count > 1}
								<div
									class="ws-floating absolute top-3 right-3 bottom-16 z-10 flex w-24 flex-col items-center gap-1.5 p-2 text-xs text-content-muted"
									data-testid="preview-layer-panel"
								>
									<span class="tabular-nums"
										>{layerRange.oneLayerMode
											? `Couche ${layerRange.high + 1}`
											: `${layerRange.low + 1}–${layerRange.high + 1}`}/{previewMeta.layer_count}</span
									>
									{#if topLayerTime !== null}
										<!-- Temps (nominal) de la couche haute, comme l'info-bulle du curseur d'Orca. -->
										<span
											class="tabular-nums text-content-subtle"
											data-testid="preview-layer-time"
											data-tip="Temps nominal de la couche haute (sans accélérations)"
											data-tip-side="left"
											aria-label="Temps nominal de la couche haute : {formatDuration(topLayerTime)}"
											>{formatDuration(topLayerTime)}</span
										>
									{/if}
									<!-- Poignée haute : numéro + Z. -->
									<button
										class="ws-chip h-auto w-full flex-col gap-0 px-1 py-0.5 leading-tight {layerRange.active ===
										'high'
											? 'ws-chip-on'
											: ''}"
										onclick={() => (layerRange = setActiveThumb(layerRange, 'high'))}
										oncontextmenu={(e) => {
											e.preventDefault();
											layerRange = setActiveThumb(layerRange, 'high');
											openLayerMenu(e.clientX, e.clientY, layerRange.high);
										}}
										aria-label="Poignée haute"
										data-tip="Poignée haute (↑/↓ quand active ; clic droit : codes de la couche)"
										data-tip-side="left"
									>
										<span class="tabular-nums">▲ {layerRange.high + 1}</span>
										<span class="text-2xs tabular-nums opacity-80"
											>{(previewMeta.layers[layerRange.high]?.z ?? 0).toFixed(2)} mm</span
										>
									</button>
									<!-- Une seule piste, deux poignées (comme Orca) : deux curseurs natifs
									     superposés (clavier, aria et data-testid conservés), seules les
									     poignées captent la souris ; plage entre les poignées surlignée. -->
									<div class="preview-track relative min-h-0 w-6 flex-1 py-1">
										<div
											class="absolute inset-y-1 left-1/2 w-1.5 -translate-x-1/2 rounded-full bg-overlay"
											aria-hidden="true"
										></div>
										<div
											class="absolute left-1/2 w-1.5 -translate-x-1/2 rounded-full bg-primary"
											style:bottom="calc(0.25rem + {layerPct(
												layerRange.oneLayerMode ? layerRange.high : layerRange.low
											)} * (100% - 0.5rem))"
											style:top="calc(0.25rem + {1 - layerPct(layerRange.high)} * (100% - 0.5rem))"
											data-testid="layer-range-highlight"
											aria-hidden="true"
										></div>
										<!-- Repères d'Orca (IMSlider) : changement de couleur, pause, G-code perso. -->
										{#each previewLayerTicks as tick, i (i)}
											<div
												class="absolute -right-1 -left-1 h-0.5 rounded-full"
												style:bottom="calc(0.25rem + {layerPct(tick.layer)} * (100% - 0.5rem))"
												style:background-color={TICK_COLORS[tick.type] ?? '#9ca3af'}
												data-testid="layer-tick"
												data-type={tick.type}
												data-layer={tick.layer}
												aria-hidden="true"
											></div>
										{/each}
										{#each pendingLayerTicks as tick, i (i)}
											<div
												class="absolute -right-1.5 -left-1.5 h-0 border-t-2 border-dashed"
												style:bottom="calc(0.25rem + {layerPct(tick.layer)} * (100% - 0.5rem))"
												style:border-color={TICK_COLORS[tick.type] ?? '#9ca3af'}
												data-testid="layer-tick-pending"
												data-type={tick.type}
												data-layer={tick.layer}
												aria-hidden="true"
											></div>
										{/each}
										<input
											type="range"
											class="preview-vslider {layerRange.active === 'high' ? 'is-active' : ''}"
											data-testid="layer-slider"
											data-current-layer={layerRange.high}
											min="0"
											max={previewMeta.layer_count - 1}
											value={layerRange.high}
											onfocus={() => (layerRange = setActiveThumb(layerRange, 'high'))}
											oninput={(e) =>
												moveLayerThumb(Number(e.currentTarget.value) - layerRange.high)}
											aria-label="Couche haute"
										/>
										{#if !layerRange.oneLayerMode}
											<input
												type="range"
												class="preview-vslider {layerRange.active === 'low' ? 'is-active' : ''}"
												min="0"
												max={previewMeta.layer_count - 1}
												value={layerRange.low}
												oninput={(e) => {
													layerRange = setActiveThumb(layerRange, 'low');
													moveLayerThumb(Number(e.currentTarget.value) - layerRange.low);
												}}
												aria-label="Couche basse"
											/>
										{/if}
									</div>
									{#if !layerRange.oneLayerMode}
										<!-- Poignée basse : numéro + Z. -->
										<button
											class="ws-chip h-auto w-full flex-col gap-0 px-1 py-0.5 leading-tight {layerRange.active ===
											'low'
												? 'ws-chip-on'
												: ''}"
											onclick={() => (layerRange = setActiveThumb(layerRange, 'low'))}
											oncontextmenu={(e) => {
												e.preventDefault();
												layerRange = setActiveThumb(layerRange, 'low');
												openLayerMenu(e.clientX, e.clientY, layerRange.low);
											}}
											aria-label="Poignée basse"
											data-tip="Poignée basse (↑/↓ quand active ; clic droit : codes de la couche)"
											data-tip-side="left"
										>
											<span class="tabular-nums">▼ {layerRange.low + 1}</span>
											<span class="text-2xs tabular-nums opacity-80"
												>{(previewMeta.layers[layerRange.low]?.z ?? 0).toFixed(2)} mm</span
											>
										</button>
									{/if}
									<div class="flex w-full gap-1">
										<button
											type="button"
											class="ws-icon-btn ws-icon-btn-sm flex-1 border-border-strong"
											aria-label="Descendre la poignée active d'une couche"
											data-tip="Couche précédente (↓)"
											data-tip-side="left"
											onclick={() => moveLayerThumb(-1)}>−</button
										>
										<button
											type="button"
											class="ws-icon-btn ws-icon-btn-sm flex-1 border-border-strong"
											aria-label="Monter la poignée active d'une couche"
											data-tip="Couche suivante (↑)"
											data-tip-side="left"
											onclick={() => moveLayerThumb(1)}>+</button
										>
									</div>
									{#if session.gcodeId}
										<!-- Orca IMSlider : icône « + » de la poignée = changement de
										     couleur à cette couche ; ici un bouton explicite (et + / −). -->
										<button
											type="button"
											class="ws-chip w-full justify-center"
											data-testid="layer-color-change"
											data-active={codeAtHighLayer ? 'true' : 'false'}
											aria-keyshortcuts={codeAtHighLayer ? '-' : '+'}
											data-tip={codeAtHighLayer
												? 'Retirer le code de la couche ' + (layerRange.high + 1) + ' (−)'
												: 'Changement de couleur à la couche ' + (layerRange.high + 1) + ' (+)'}
											data-tip-side="left"
											onclick={() => editColorChangeKey(codeAtHighLayer ? '-' : '+')}
											>{codeAtHighLayer ? '− couleur' : '+ couleur'}</button
										>
										<!-- Phase 37 — menu d'IMSlider : pause, G-code perso, modèle,
										     changement de filament ; modifier / supprimer le code posé. -->
										<button
											type="button"
											class="ws-chip w-full justify-center"
											data-testid="layer-code-menu-open"
											aria-haspopup="menu"
											data-tip={'Codes de la couche ' +
												(activeThumbLayer() + 1) +
												' : pause, G-code, filament…'}
											data-tip-side="left"
											onclick={(e) => {
												const rect = e.currentTarget.getBoundingClientRect();
												openLayerMenu(rect.left, rect.bottom);
											}}>Codes…</button
										>
									{/if}
									<button
										class="ws-chip w-full justify-center {layerRange.oneLayerMode
											? 'ws-chip-on'
											: ''}"
										onclick={() => {
											layerRange = toggleOneLayerMode(layerRange);
											void refreshGeometry();
										}}
										data-tip="Mode une couche (L)"
										data-tip-side="left"
										aria-keyshortcuts="L">1 couche</button
									>
									{#if goToLayerOpen}
										<form
											class="flex w-full flex-col gap-1"
											data-testid="preview-goto-layer"
											onsubmit={(e) => {
												e.preventDefault();
												submitGoToLayer();
											}}
										>
											<!-- svelte-ignore a11y_autofocus -->
											<input
												class="ws-field ws-field-sm w-full text-center"
												type="number"
												min="1"
												max={previewMeta.layer_count}
												bind:value={goToLayerValue}
												aria-label="Aller à la couche (1 – {previewMeta.layer_count})"
												autofocus
												onkeydown={(e) => {
													e.stopPropagation();
													if (e.key === 'Escape') goToLayerOpen = false;
												}}
											/>
											<button type="submit" class="ws-btn ws-btn-sm ws-btn-primary w-full"
												>Aller</button
											>
										</form>
									{:else}
										<button
											type="button"
											class="ws-chip w-full justify-center"
											data-testid="preview-goto-layer-open"
											data-tip="Aller à la couche (Maj+G)"
											data-tip-side="left"
											aria-keyshortcuts="Shift+G"
											onclick={openGoToLayer}>Aller à…</button
										>
									{/if}
								</div>
							{/if}

							<!-- Légende + options d'affichage : panneau flottant repliable (coin haut
							     gauche, comme la légende d'Orca), la scène garde toute la hauteur. -->
							<div
								class="ws-floating absolute top-16 left-3 z-10 flex max-h-[calc(100%-9rem)] w-[26rem] max-w-[calc(100%-9rem)] flex-col overflow-hidden"
								data-testid="preview-legend-panel"
							>
								<button
									type="button"
									class="flex items-center justify-between px-3 py-1.5 text-left hover:bg-overlay"
									aria-expanded={legendOpen}
									aria-controls="preview-legend-body"
									onclick={() => (legendOpen = !legendOpen)}
								>
									<span class="ws-section-title">Légende</span>
									<span class="text-content-subtle" aria-hidden="true"
										>{legendOpen ? '▾' : '▸'}</span
									>
								</button>
								<div
									id="preview-legend-body"
									class="min-h-0 overflow-auto border-t border-border"
									hidden={!legendOpen}
								>
									<PreviewLegend
										legend={buildLegend(coloration, {
											ranges: { ...rangesFromMeta(previewMeta), flow: previewFlowRange },
											typesPresent: previewMeta.types_present,
											extrudersPresent: previewMeta.extruders_present,
											filamentColors: previewFilamentColors,
											track: previewTrack
										})}
										showTypes={coloration === 'type'}
										types={previewMeta.types_present}
										{hiddenTypes}
										ontoggletype={toggleType}
										options={previewOptionCounts}
										enabled={previewOptionSet}
										ontoggleoption={togglePreviewOption}
										loading={previewTrackLoading}
									/>
								</div>
							</div>
						</div>
					{:else}
						<div class="flex flex-1 items-center justify-center p-6 text-content-subtle">
							<!-- Tranchage terminé mais trajectoires encore en chargement : ne pas
							     inviter à trancher (message trompeur pendant 1 à 15 s en multi-filament). -->
							{#if session.phase === 'preview' && session.gcodeId}
								<p role="status" data-testid="preview-loading">Chargement de l'aperçu…</p>
							{:else}
								<p>Aperçu G-code : tranchez la scène pour l'afficher.</p>
							{/if}
						</div>
					{/if}
				</div>
			{:else if layout.tab === 'device'}
				<!-- Onglet Appareil (T117, US8) : imprimantes Moonraker + contrôles. -->
				<div class="h-full overflow-auto">
					<DevicePanel />
				</div>
			{:else}
				<!-- Onglet Projet (T117) : métadonnées du projet + renommage. -->
				<div class="h-full overflow-auto p-6">
					<div class="mx-auto flex w-full max-w-4xl flex-col gap-4">
						<h2 class="ws-dialog-title">Projet</h2>
						{#if data.project.has_thumbnail}
							<img
								src="/api/projects/{data.project.id}/thumbnail"
								alt="Vignette du projet"
								class="max-h-48 w-auto self-start rounded border border-border"
							/>
						{/if}
						<label class="flex flex-col gap-1">
							<span class="text-sm text-content-muted">Nom</span>
							<div class="flex gap-2">
								<input class="ws-field flex-1" bind:value={projectName} />
								<button
									class="ws-btn ws-btn-primary"
									disabled={projectName.trim() === '' || projectName === savedName}
									onclick={() => renameCurrentProject()}>Renommer</button
								>
							</div>
						</label>
						<dl class="grid grid-cols-2 gap-x-4 gap-y-1 text-sm">
							<dt class="text-content-muted">Identifiant</dt>
							<dd class="truncate font-mono text-xs text-content">{data.project.id}</dd>
							<dt class="text-content-muted">Version</dt>
							<dd class="tabular-nums text-content">{data.project.version}</dd>
							<dt class="text-content-muted">Créé le</dt>
							<dd class="text-content">{new Date(data.project.created_at).toLocaleString()}</dd>
							<dt class="text-content-muted">Modifié le</dt>
							<dd class="text-content">{new Date(data.project.updated_at).toLocaleString()}</dd>
						</dl>
						<!-- Fiche du modèle (ProjectPanel d'Orca : informations du modèle avant
						     les fichiers joints) — même panneau que l'onglet Projet de la colonne. -->
						<section class="flex flex-col gap-2" aria-labelledby="project-page-meta-title">
							<h3 id="project-page-meta-title" class="ws-section-title">Fiche du modèle</h3>
							<ProjectMetadataPanel projectId={data.project.id} />
						</section>
						<AuxiliariesPanel projectId={data.project.id} />
					</div>
				</div>
			{/if}
		</main>

		<!-- Zone droite : rail d'outils vertical (16 gizmos, T103). Actif en préparation. -->
		{#if layout.tab === 'prepare'}
			<aside
				aria-label="Outils"
				class="flex flex-col items-center border-l border-border bg-surface-raised p-2"
			>
				<ToolRail active={tools.active} onselect={selectTool} />
			</aside>
		{/if}
	</div>
</div>

<style>
	/* Curseur de couches vertical (T116). */
	/* Notification (phase 31) : courte entrée à chaque message, même répété. */
	.ws-notice-in {
		display: inline-block;
		animation: ws-notice-in 220ms ease-out;
	}
	@keyframes ws-notice-in {
		from {
			opacity: 0.2;
			transform: translateY(4px);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.ws-notice-in {
			animation: none;
		}
	}
	.preview-vslider {
		position: absolute;
		inset: 0;
		margin: 0;
		width: 100%;
		height: 100%;
		writing-mode: vertical-lr;
		direction: rtl;
		appearance: none;
		background: transparent;
		pointer-events: none;
	}
	.preview-vslider::-webkit-slider-runnable-track {
		background: transparent;
	}
	.preview-vslider::-moz-range-track {
		background: transparent;
	}
	.preview-vslider::-webkit-slider-thumb {
		appearance: none;
		pointer-events: auto;
		width: 1.125rem;
		height: 1.125rem;
		border-radius: 9999px;
		background: var(--ws-surface-raised);
		border: 2px solid var(--ws-primary);
		cursor: grab;
	}
	.preview-vslider::-moz-range-thumb {
		pointer-events: auto;
		width: 1rem;
		height: 1rem;
		border-radius: 9999px;
		background: var(--ws-surface-raised);
		border: 2px solid var(--ws-primary);
		cursor: grab;
	}
	.preview-vslider.is-active {
		z-index: 1;
	}
	.preview-vslider.is-active::-webkit-slider-thumb {
		background: var(--ws-primary);
	}
	.preview-vslider.is-active::-moz-range-thumb {
		background: var(--ws-primary);
	}
	.preview-vslider:focus-visible {
		outline: none;
	}
	.preview-vslider:focus-visible::-webkit-slider-thumb {
		box-shadow: 0 0 0 3px var(--ws-accent);
	}
</style>
