<script lang="ts">
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { draftStore, type DraftRecord } from '$lib/stores/draft';
	import {
		Scene,
		ObjectList,
		PlateBar,
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
		PaintToolbar,
		TrianglePainting,
		type PaintChannel,
		ObjectTree,
		PlateSet,
		bedFromValues,
		serializeScene,
		saveScene,
		loadPreview as loadModelPreview,
		previewFromBuffer,
		uploadModel,
		fetchMesh,
		fetchModelFile,
		type SaveOutcome,
		type SceneObject,
		type SceneMesh,
		type Transform
	} from '$lib/scene';
	import { SettingsTabs } from '$lib/settings';
	import { PrinterSelect, PresetSelect } from '$lib/presets';
	import {
		PreviewScene,
		StatsPanel,
		LazyPreviewLoader,
		buildPreviewStats,
		type GcodeStats
	} from '$lib/preview';
	import type { PreviewGeometry } from '$lib/preview/geometry';
	import { sliceProject, listProjectModels } from '$lib/api/projects';
	import { fetchPreviewLayers, getPreviewMeta } from '$lib/api/preview';
	import { listPresets, createPreset, updatePreset, deletePreset } from '$lib/api/presets';
	import { subscribeEvents, type EventSubscription } from '$lib/queue/events';
	import type { PreviewMeta, PresetSummary, ServerEvent } from '$lib/api/types';
	import { ApiError } from '$lib/api/client';
	import { t } from '$lib/i18n';
	import {
		initialWorkspace,
		pick,
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
		findByModel,
		isAccepted,
		isPreviewable,
		isTransformTool,
		paintChannelOf,
		type EditorTool,
		initialLayout,
		setTab,
		EDITOR_DEFAULT_THEME,
		emptyActivePresets,
		parseActivePresets,
		serializeActivePresets,
		primaryFilament,
		setProcess,
		setFilament,
		type EditorTab,
		type ImportItem
	} from '$lib/editor';
	import type { DisplayMode } from '$lib/settings/filter';
	import type { PageData } from './$types';

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
	let paintTool = $state<'brush' | 'sphere' | 'fill'>('brush');
	let paintRadius = $state(2);
	let paintState = $state(1);
	// Document de peinture par objet (indexé par id) : créé paresseusement pour
	// l'objet sélectionné, sérialisable au format 3MF Orca via `serialize`.
	let paintings = $state<Record<string, TrianglePainting>>({});

	// Modèle de scène (mutations en place → proxysées par `$state`, réactives).
	let tree = $state(new ObjectTree());
	let plates = $state(new PlateSet());
	// Maillages rendus dans la scène 3D : peuplés à l'ouverture depuis les modèles
	// du projet (T092) puis par les imports (T089).
	let sceneObjects = $state<SceneObject[]>([]);

	// Plateau par défaut tant que le preset machine n'est pas résolu (le layout
	// et les dimensions réelles viennent des valeurs `printable_area` du preset).
	const bed = $derived(bedFromValues({}));

	// Presets actifs du projet (T098) : imprimante / filament / process. Peuplés
	// depuis le blob `active_presets` au montage (lecture non réactive de `data`),
	// modifiés par les sélecteurs, persistés au save.
	let activePresets = $state(emptyActivePresets());
	let machinePresets = $state<PresetSummary[]>([]);
	let filamentPresets = $state<PresetSummary[]>([]);
	let processPresets = $state<PresetSummary[]>([]);
	// Filament principal (mono-extrudeur v1) : liable par le sélecteur, resynchronisé
	// vers le tableau `filaments` de l'orchestrateur.
	let filamentSel = $state<string | null>(null);
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
	// Propage la sélection filament du composant vers l'état des presets actifs.
	$effect(() => {
		if (filamentSel !== primaryFilament(activePresets)) {
			activePresets = setFilament(activePresets, filamentSel);
		}
	});

	// Onglet de la colonne de configuration : liste d'objets/plateaux ou réglages.
	let configTab = $state<'objects' | 'settings'>('settings');
	let settingsMode = $state<DisplayMode>('simple');
	// Portée des réglages affichés (T100/T101/T102) : process / filament / machine,
	// chacun avec son propre jeu de valeurs éditées.
	let settingsScope = $state<'process' | 'filament' | 'machine'>('process');
	let processValues = $state<Record<string, unknown>>({});
	let filamentValues = $state<Record<string, unknown>>({});
	let machineValues = $state<Record<string, unknown>>({});
	let saveMessage = $state<string | null>(null);

	// Tranchage + aperçu G-code (T088) : machine à états pure `session` pilotée
	// par les événements WebSocket ; le G-code prêt est chargé paresseusement.
	let session = $state(prepareSession());
	let previewMeta = $state<PreviewMeta | null>(null);
	let previewGeometry = $state<PreviewGeometry | null>(null);
	let rawStats = $state<unknown>(null);
	let topLayer = $state(0);
	let loader: LazyPreviewLoader | null = null;

	const previewStats = $derived(rawStats ? buildPreviewStats(rawStats as GcodeStats) : null);
	const canSliceNow = $derived(tree.list().length > 0);

	// Import de modèles (T089) : file d'imports suivie + zone de dépôt.
	let imports = $state<ImportItem[]>([]);
	let importError = $state<string | null>(null);
	let dragOver = $state(false);
	let fileInput: HTMLInputElement | null = null;
	const ACCEPT = '.stl,.obj,.3mf,.oltp,.step,.stp,.amf,.svg,.drc';

	onMount(() => {
		let alive = true;
		// Thème sombre par défaut dans l'éditeur (parité OrcaSlicer) : on force
		// `data-theme` au niveau racine le temps de l'éditeur, sans toucher à la
		// préférence persistée de l'utilisateur (restaurée à la sortie). Le
		// `initTheme` du layout parent s'exécute juste après ce `onMount` enfant :
		// on réaffirme donc le thème via un `requestAnimationFrame` qui a le
		// dernier mot après le montage complet.
		const root = document.documentElement;
		const previousTheme = root.dataset.theme;
		const applyDark = () => (root.dataset.theme = EDITOR_DEFAULT_THEME);
		applyDark();
		const rafId = requestAnimationFrame(applyDark);

		// Presets actifs du projet (lecture non réactive de `data` au montage, T098).
		activePresets = parseActivePresets(data.project.active_presets);
		filamentSel = primaryFilament(activePresets);

		draftStore.pendingRestore(data.project.id, data.project.updated_at).then((d) => {
			if (alive) pendingDraft = d;
		});
		// Repeuple la scène depuis les modèles déjà rattachés au projet (T092).
		void loadProjectModels();
		// Charge les catalogues de presets pour les sélecteurs (T098).
		void loadPresets();
		// Flux d'événements du compte (T065) : progression et fin des jobs.
		const sub: EventSubscription = subscribeEvents({ onEvent });
		return () => {
			alive = false;
			sub.close();
			cancelAnimationFrame(rafId);
			if (previousTheme) root.dataset.theme = previousTheme;
			else delete root.dataset.theme;
		};
	});

	// Ouvre le projet : charge ses modèles, reconstruit l'arbre d'objets et les
	// maillages affichables. Les formats non décodés client (STEP/AMF/SVG/DRC)
	// restent en attente de conversion moteur (badge « conversion en cours »).
	async function loadProjectModels() {
		let models;
		try {
			models = await listProjectModels(data.project.id);
		} catch (e) {
			importError = e instanceof ApiError ? e.message : 'chargement des modèles impossible';
			return;
		}
		for (const model of models) {
			const objectId = tree.add(model.filename).id;
			if (plates.activeId) plates.assign(objectId, plates.activeId);
			// Suivi de l'import (permet la résolution via `model.converted`).
			imports = [
				...imports,
				markUploaded(startImport(objectId, model.filename), model.id, model.conversion_pending)
			];
			if (isPreviewable(model.filename)) {
				try {
					const mesh = previewFromBuffer(model.filename, await fetchModelFile(model.id));
					sceneObjects = [...sceneObjects, { id: objectId, mesh }];
				} catch {
					patchImport(objectId, (i) => markFailed(i, 'aperçu indisponible'));
				}
			}
		}
	}

	// Charge les catalogues de presets (T098). Filament/process sont filtrés par
	// compatibilité avec l'imprimante sélectionnée (FR-021, côté serveur).
	async function loadPresets() {
		const printerName = machinePresets.find((p) => p.id === activePresets.printer)?.name;
		try {
			const [machines, filaments, processes] = await Promise.all([
				listPresets('machine'),
				listPresets('filament', printerName),
				listPresets('process', printerName)
			]);
			machinePresets = machines;
			filamentPresets = filaments;
			processPresets = processes;
		} catch (e) {
			importError = e instanceof ApiError ? e.message : 'chargement des presets impossible';
		}
	}

	// Dérive un preset utilisateur (copie modifiable) puis le sélectionne.
	async function derivePreset(kind: 'filament' | 'process', base: PresetSummary) {
		try {
			const created = await createPreset({
				kind: base.kind,
				name: `${base.name} (copie)`,
				inherits: base.id,
				values: {}
			});
			await loadPresets();
			if (kind === 'filament') filamentSel = created.id;
			else activePresets = setProcess(activePresets, created.id);
		} catch (e) {
			importError = e instanceof ApiError ? e.message : 'dérivation du preset impossible';
		}
	}

	async function removePreset(kind: 'filament' | 'process', preset: PresetSummary) {
		try {
			await deletePreset(preset.id);
			if (kind === 'filament' && filamentSel === preset.id) filamentSel = null;
			if (kind === 'process' && activePresets.process === preset.id)
				activePresets = setProcess(activePresets, null);
			await loadPresets();
		} catch (e) {
			importError = e instanceof ApiError ? e.message : 'suppression du preset impossible';
		}
	}

	// Valeurs éditées du panneau de réglages correspondant au type d'un preset.
	function valuesForKind(kind: string): Record<string, unknown> {
		if (kind === 'filament') return filamentValues;
		if (kind === 'machine' || kind === 'machine_model') return machineValues;
		return processValues;
	}

	// Enregistre les valeurs éditées (du bon groupe de réglages) dans le preset
	// utilisateur courant.
	async function savePreset(preset: PresetSummary) {
		try {
			await updatePreset(preset.id, { values: valuesForKind(preset.kind) });
			await loadPresets();
			saveMessage = 'Preset enregistré.';
		} catch (e) {
			importError = e instanceof ApiError ? e.message : 'enregistrement du preset impossible';
		}
	}

	function onEvent(event: ServerEvent) {
		const wasPreview = layout.tab === 'preview';
		session = applyJobEvent(session, event);
		if (event.event === 'job.finished' && session.jobIds.includes(event.id)) {
			rawStats = event.stats;
		}
		// Bascule automatique vers l'aperçu au premier G-code produit.
		if (!wasPreview && session.phase === 'preview' && session.gcodeId) {
			layout = setTab(layout, 'preview');
			void loadPreview(session.gcodeId);
		}
		// Fin de conversion moteur (STEP…) : récupère le maillage et l'affiche.
		if (event.event === 'model.converted') {
			void resolveConversion(event.model_id);
		}
	}

	function patchImport(objectId: string, fn: (i: ImportItem) => ImportItem) {
		imports = imports.map((i) => (i.objectId === objectId ? fn(i) : i));
	}

	// Importe une liste de fichiers : aperçu immédiat + upload en tâche de fond.
	async function importFiles(files: File[]) {
		importError = null;
		for (const file of files) {
			if (!isAccepted(file.name)) {
				importError = `Format non supporté : ${file.name}`;
				continue;
			}
			await importOne(file);
		}
	}

	async function importOne(file: File) {
		const node = tree.add(file.name);
		const objectId = node.id;
		if (plates.activeId) plates.assign(objectId, plates.activeId);
		imports = [...imports, startImport(objectId, file.name)];

		// Aperçu client immédiat (STL/OBJ/3MF) pendant que l'upload part.
		if (isPreviewable(file.name)) {
			try {
				const mesh = await loadModelPreview(file);
				sceneObjects = [...sceneObjects, { id: objectId, mesh }];
			} catch {
				patchImport(objectId, (i) => markFailed(i, 'aperçu illisible'));
			}
		}

		// Upload en tâche de fond (T048) ; le STEP passe en conversion moteur.
		try {
			const model = await uploadModel(data.project.id, file);
			patchImport(objectId, (i) => markUploaded(i, model.id, model.conversion_pending));
		} catch (e) {
			patchImport(objectId, (i) =>
				markFailed(i, e instanceof ApiError ? e.message : 'échec de l’upload')
			);
		}
	}

	async function resolveConversion(modelId: string) {
		const item = findByModel(imports, modelId);
		if (!item) return;
		try {
			const mesh = await fetchMesh(modelId);
			sceneObjects = [...sceneObjects, { id: item.objectId, mesh }];
			patchImport(item.objectId, markConverted);
		} catch {
			patchImport(item.objectId, (i) => markFailed(i, 'maillage converti indisponible'));
		}
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
		ws = pick(ws, id, additive);
	}

	function selectTool(tool: EditorTool) {
		tools = setTool(tools, tool);
	}

	// Applique la transformation d'un objet manipulé par le gizmo (T103) : persiste
	// position/rotation/échelle dans l'état de scène (survit au re-rendu).
	function onTransform(id: string, transform: Transform) {
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
	// État local des outils sans persistance moteur.
	let assemblyExploded = $state(false);
	let assemblyFactor = $state(0.5);

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
	function toolNeedsEngine(feature: string) {
		saveMessage = `${feature} : opération moteur non disponible en v1 (backlog FFI).`;
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
	// Efface le canal actif de l'objet sélectionné (réaffecte l'index → réactif).
	function clearActivePaint() {
		if (!selId || !paintChannel) return;
		paintingFor(selId).clear(paintChannel);
		paintings = { ...paintings };
	}

	function showTab(tab: EditorTab) {
		layout = setTab(layout, tab);
	}

	// Lance le tranchage du plateau actif et affiche la progression dans l'aperçu.
	async function sliceActive() {
		const req = sliceRequestFor('plate', plates.plateIndex(plates.activeId ?? ''));
		try {
			const res = await sliceProject(data.project.id, req);
			session = startSlicing(
				res.jobs.map((j) => j.id),
				res.warnings
			);
			layout = setTab(layout, 'preview');
		} catch (e) {
			session = sliceFailed(e instanceof ApiError ? e.message : 'échec du lancement du tranchage');
			layout = setTab(layout, 'preview');
		}
	}

	// Charge la méta d'aperçu puis prépare le chargeur paresseux (T084).
	async function loadPreview(gcodeId: string) {
		previewMeta = await getPreviewMeta(gcodeId);
		loader = new LazyPreviewLoader({
			layerCount: previewMeta.layer_count,
			fetchRange: (from, to) => fetchPreviewLayers(gcodeId, from, to)
		});
		topLayer = Math.max(0, previewMeta.layer_count - 1);
		await refreshGeometry();
	}

	// Reconstruit la géométrie de lignes pour la fenêtre autour de `topLayer`.
	async function refreshGeometry() {
		if (!loader || !previewMeta) return;
		await loader.ensure(topLayer);
		previewGeometry = buildWindowGeometry(loader.window(topLayer), {
			coloration: 'type',
			ranges: rangesFromMeta(previewMeta)
		});
	}

	async function save(): Promise<SaveOutcome> {
		const scene = serializeScene(plates.serialize(), []);
		const outcome = await saveScene(
			data.project.id,
			Number(data.project.version),
			scene,
			serializeActivePresets(activePresets)
		);
		saveMessage =
			outcome.status === 'saved'
				? 'Projet sauvegardé.'
				: outcome.status === 'conflict'
					? 'Conflit : le projet a été modifié dans un autre onglet.'
					: outcome.message;
		return outcome;
	}

	// Onglets supérieurs : libellé de parité anglais → i18n.
	const TABS: { id: EditorTab; label: string }[] = [
		{ id: 'prepare', label: 'Prepare' },
		{ id: 'preview', label: 'Preview' },
		{ id: 'device', label: 'Device' },
		{ id: 'project', label: 'Project' }
	];
</script>

<div class="flex h-screen flex-col bg-surface text-content">
	<!-- Barre supérieure : onglets de vue (parité OrcaSlicer) + actions plateau. -->
	<header class="flex items-center justify-between border-b border-border px-4 py-2">
		<div class="flex items-center gap-4">
			<a
				href={resolve('/library')}
				class="text-sm text-primary hover:underline"
				title="Bibliothèque">←</a
			>
			<nav class="flex items-center gap-1" aria-label="Vues de l'éditeur">
				{#each TABS as tab (tab.id)}
					<button
						class="rounded px-3 py-1.5 text-sm font-medium {layout.tab === tab.id
							? 'bg-primary text-primary-content'
							: 'text-content-muted hover:bg-overlay'}"
						onclick={() => showTab(tab.id)}
						aria-current={layout.tab === tab.id ? 'page' : undefined}
					>
						{$t(tab.label)}
					</button>
				{/each}
			</nav>
			<span class="ml-2 text-sm text-content-subtle">{data.project.name}</span>
		</div>

		<div class="flex items-center gap-2">
			<button
				class="rounded border border-border-strong px-3 py-1 text-sm text-content-muted hover:bg-overlay"
				onclick={() => fileInput?.click()}
			>
				{$t('Import')}
			</button>
			<input
				bind:this={fileInput}
				type="file"
				multiple
				accept={ACCEPT}
				class="hidden"
				onchange={onFilePicked}
			/>
			<button
				class="rounded bg-success px-3 py-1 text-sm text-white hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-40"
				onclick={sliceActive}
				disabled={!canSliceNow}
				title={canSliceNow ? 'Trancher le plateau actif' : 'Ajoutez un objet à trancher'}
			>
				{$t('Slice plate')}
			</button>
			<SaveControls {save} />
			<!-- Export projet 3MF (FR-044) : ressource API backend, pas une route SvelteKit. -->
			<!-- eslint-disable svelte/no-navigation-without-resolve -->
			<a
				class="rounded bg-primary px-3 py-1.5 text-sm text-primary-content hover:bg-primary-hover"
				href="/api/projects/{data.project.id}/export/3mf"
				download
			>
				{$t('Export Generic 3MF…')}
			</a>
			<!-- eslint-enable svelte/no-navigation-without-resolve -->
		</div>
	</header>

	{#if pendingDraft}
		<div
			class="flex items-center justify-between gap-4 bg-warning-soft px-6 py-2 text-sm text-warning-content"
			role="status"
		>
			<span>Un brouillon local plus récent existe pour ce projet.</span>
			<button onclick={dismissDraft} class="font-medium hover:underline">Ignorer</button>
		</div>
	{/if}

	{#if saveMessage}
		<div class="bg-overlay px-6 py-2 text-sm text-content-muted">
			{saveMessage}
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
		<aside class="flex w-96 flex-col border-r border-border bg-surface-raised">
			<div class="flex border-b border-border">
				<button
					class="flex-1 px-3 py-2 text-sm {configTab === 'settings'
						? 'border-b-2 border-primary font-medium text-primary'
						: 'text-content-muted'}"
					onclick={() => (configTab = 'settings')}>{$t('Process')}</button
				>
				<button
					class="flex-1 px-3 py-2 text-sm {configTab === 'objects'
						? 'border-b-2 border-primary font-medium text-primary'
						: 'text-content-muted'}"
					onclick={() => (configTab = 'objects')}>Objets</button
				>
			</div>

			<div class="min-h-0 flex-1 overflow-auto p-3">
				{#if configTab === 'objects'}
					<PlateBar
						plates={plates.list()}
						activeId={plates.activeId}
						onselect={(id) => (plates.activeId = id)}
						onadd={() => plates.addPlate()}
						onremove={(id) => plates.removePlate(id)}
						ontype={(id, plateType) => plates.setPlateType(id, plateType)}
					/>
					<ObjectList
						{tree}
						selection={ws.selection}
						onselect={selectObject}
						ontogglelock={(id) => tree.setLocked(id, !tree.isLocked(id))}
						ontogglehide={(id) => tree.setHidden(id, !tree.isHidden(id))}
						onextruder={(id, extruder) => tree.setExtruder(id, extruder)}
						onduplicate={(id) => tree.duplicate(id)}
						ondelete={(id) => tree.remove(id)}
						ongroup={() => tree.group([...ws.selection])}
					/>
				{:else}
					<div class="flex flex-col gap-4">
						<!-- Sélecteurs Imprimante / Filament / Process (T098). -->
						<section class="flex flex-col gap-3 border-b border-border pb-3">
							<div class="flex flex-col gap-1">
								<span class="text-xs font-semibold tracking-wide text-content-subtle uppercase"
									>{$t('Printer')}</span
								>
								<PrinterSelect presets={machinePresets} bind:selectedId={activePresets.printer} />
							</div>
							<PresetSelect
								presets={filamentPresets}
								bind:selectedId={filamentSel}
								label={$t('Filament')}
								onderive={(p) => derivePreset('filament', p)}
								onsave={savePreset}
								ondelete={(p) => removePreset('filament', p)}
							/>
							<PresetSelect
								presets={processPresets}
								bind:selectedId={activePresets.process}
								label={$t('Process')}
								onderive={(p) => derivePreset('process', p)}
								onsave={savePreset}
								ondelete={(p) => removePreset('process', p)}
							/>
						</section>
						<!-- Sélecteur de portée des réglages (T100/T101/T102) : process / filament / machine. -->
						<div class="flex gap-1 rounded border border-border-strong p-0.5">
							{#each [{ id: 'process', label: 'Process' }, { id: 'filament', label: 'Filament' }, { id: 'machine', label: 'Printer' }] as scope (scope.id)}
								<button
									type="button"
									onclick={() => (settingsScope = scope.id as 'process' | 'filament' | 'machine')}
									class="flex-1 rounded px-2 py-1 text-xs whitespace-nowrap {settingsScope ===
									scope.id
										? 'bg-primary text-primary-content'
										: 'text-content-muted hover:bg-overlay'}"
								>
									{$t(scope.label)}
								</button>
							{/each}
						</div>
						{#if settingsScope === 'process'}
							<SettingsTabs kind="process" bind:mode={settingsMode} bind:values={processValues} />
						{:else if settingsScope === 'filament'}
							<SettingsTabs kind="filament" bind:mode={settingsMode} bind:values={filamentValues} />
						{:else}
							<SettingsTabs kind="machine" bind:mode={settingsMode} bind:values={machineValues} />
						{/if}
					</div>
				{/if}
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
		>
			{#if layout.tab === 'prepare'}
				<Scene
					{bed}
					objects={sceneObjects}
					bind:selection={ws.selection}
					{gizmoMode}
					ontransform={onTransform}
				/>

				<!-- Panneau de l'outil actif (T104) : flottant, agit sur l'objet sélectionné. -->
				{#if tools.active && !isTransformTool(tools.active)}
					<div
						class="absolute top-3 left-3 z-10 w-72 rounded border border-border bg-surface-raised/95 p-3 shadow-lg"
					>
						<div class="mb-2 flex items-center justify-between">
							<span class="text-xs font-semibold tracking-wide text-content-subtle uppercase">
								{$t('Tools')}
							</span>
							<button
								class="text-content-muted hover:text-content"
								onclick={closeTool}
								aria-label={$t('Close')}>✕</button
							>
						</div>
						{#if !activeObject && tools.active !== 'assembly'}
							<p class="text-sm text-content-subtle">Sélectionnez un objet.</p>
						{:else if tools.active === 'flatten'}
							<TransformPanel
								transform={activeTransform}
								onchange={(tr) => selId && onTransform(selId, tr)}
							/>
						{:else if tools.active === 'cut'}
							<CutTool
								min={activeBounds.min}
								max={activeBounds.max}
								oncut={() => toolNeedsEngine('Découpe')}
								oncancel={closeTool}
							/>
						{:else if tools.active === 'boolean'}
							<BooleanTool onapply={() => toolNeedsEngine('Booléen de maillage')} />
						{:else if tools.active === 'simplify'}
							<SimplifyTool
								onsimplify={() => toolNeedsEngine('Simplification')}
								oncancel={closeTool}
							/>
						{:else if tools.active === 'brim-ears'}
							<BrimEarsTool points={[]} onremove={() => {}} onclear={closeTool} />
						{:else if tools.active === 'measure'}
							<MeasureTool measurement={null} />
						{:else if tools.active === 'emboss'}
							<EmbossTool onsubmit={() => toolNeedsEngine('Emboss / texte')} />
						{:else if tools.active === 'svg'}
							<EmbossTool onsubmit={() => toolNeedsEngine('Forme SVG')} />
						{:else if tools.active === 'assembly'}
							<AssemblyView bind:exploded={assemblyExploded} bind:factor={assemblyFactor} />
						{:else if paintChannel}
							<!-- Barre de peinture (T105) : canal verrouillé sur l'outil du rail. -->
							<PaintToolbar
								channel={paintChannel}
								bind:tool={paintTool}
								bind:radius={paintRadius}
								bind:state={paintState}
							/>
							<div class="mt-3 flex items-center justify-between text-xs text-content-subtle">
								<span>{activePaintedCount} triangle(s) peint(s)</span>
								<button
									class="rounded border border-border-strong px-2 py-0.5 hover:bg-surface-sunken"
									onclick={clearActivePaint}
									disabled={activePaintedCount === 0}>Effacer</button
								>
							</div>
							<p class="mt-1 text-xs text-content-subtle">
								La pose au pinceau sur le maillage dépend du picking de scène (incrément
								scène↔peinture).
							</p>
						{/if}
					</div>
				{/if}

				{#if sceneObjects.length === 0}
					<div
						class="pointer-events-none absolute inset-0 flex items-center justify-center text-center text-sm text-content-subtle"
					>
						<p>
							Glissez un modèle ici (STL, OBJ, 3MF, STEP, AMF, SVG, DRC)<br />ou cliquez sur
							<span class="font-medium">{$t('Import')}</span>.
						</p>
					</div>
				{/if}

				{#if dragOver}
					<div
						class="pointer-events-none absolute inset-0 z-20 flex items-center justify-center border-2 border-dashed border-accent bg-primary/10 text-sm font-medium text-primary"
					>
						Déposez pour importer
					</div>
				{/if}

				{#if imports.some((i) => i.status === 'converting' || i.status === 'failed')}
					<div class="absolute bottom-3 left-3 z-10 flex flex-col gap-1">
						{#each imports.filter((i) => i.status === 'converting' || i.status === 'failed') as it (it.objectId)}
							<div
								class="rounded px-2 py-1 text-xs {it.status === 'failed'
									? 'bg-danger-soft text-danger-content'
									: 'bg-overlay text-content-muted'}"
							>
								{it.filename} —
								{it.status === 'failed' ? (it.error ?? 'échec') : 'conversion en cours…'}
							</div>
						{/each}
					</div>
				{/if}
			{:else if layout.tab === 'preview'}
				<div class="flex h-full flex-col">
					{#if session.phase === 'slicing'}
						<div
							class="flex flex-1 flex-col items-center justify-center gap-3 p-6 text-content-muted"
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
						<div class="flex flex-1 items-center justify-center p-6 text-danger">
							<p>Échec du tranchage : {session.error}</p>
						</div>
					{:else if session.phase === 'preview' && previewGeometry && previewMeta}
						<div class="relative min-h-0 flex-1">
							<PreviewScene geometry={previewGeometry} {bed} />
							{#if previewMeta.layer_count > 1}
								<div
									class="absolute right-3 top-3 flex flex-col items-center gap-2 rounded bg-surface-raised/80 p-2 text-xs text-content-muted"
								>
									<span>Couche {topLayer + 1}/{previewMeta.layer_count}</span>
									<input
										type="range"
										min="0"
										max={previewMeta.layer_count - 1}
										bind:value={topLayer}
										oninput={refreshGeometry}
									/>
								</div>
							{/if}
						</div>
						{#if previewStats}
							<div class="max-h-64 overflow-auto border-t border-border">
								<StatsPanel stats={previewStats} />
							</div>
						{/if}
					{:else}
						<div class="flex flex-1 items-center justify-center p-6 text-content-subtle">
							<p>Aperçu G-code : tranchez la scène pour l'afficher.</p>
						</div>
					{/if}
				</div>
			{:else}
				<!-- Onglets Appareil / Projet : contenu livré par T108 / T117. -->
				<div class="flex h-full items-center justify-center p-6 text-content-subtle">
					<p>{$t(layout.tab === 'device' ? 'Device' : 'Project')} — bientôt disponible.</p>
				</div>
			{/if}
		</main>

		<!-- Zone droite : rail d'outils vertical (16 gizmos, T103). Actif en préparation. -->
		{#if layout.tab === 'prepare'}
			<aside class="flex flex-col items-center border-l border-border bg-surface-raised p-2">
				<ToolRail active={tools.active} onselect={selectTool} />
			</aside>
		{/if}
	</div>
</div>
