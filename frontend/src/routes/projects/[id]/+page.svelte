<script lang="ts">
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { draftStore, type DraftRecord } from '$lib/stores/draft';
	import {
		Scene,
		ObjectList,
		PlateBar,
		SaveControls,
		GizmoToolbar,
		ObjectTree,
		PlateSet,
		bedFromValues,
		serializeScene,
		saveScene,
		type SaveOutcome,
		type SceneObject
	} from '$lib/scene';
	import { SettingsTabs } from '$lib/settings';
	import {
		PreviewScene,
		StatsPanel,
		LazyPreviewLoader,
		buildPreviewStats,
		type GcodeStats
	} from '$lib/preview';
	import type { PreviewGeometry } from '$lib/preview/geometry';
	import { sliceProject } from '$lib/api/projects';
	import { fetchPreviewLayers, getPreviewMeta } from '$lib/api/preview';
	import { subscribeEvents, type EventSubscription } from '$lib/queue/events';
	import type { PreviewMeta, ServerEvent } from '$lib/api/types';
	import { ApiError } from '$lib/api/client';
	import {
		initialWorkspace,
		pick,
		setGizmoMode,
		setPanel,
		type EditorPanel,
		prepareSession,
		startSlicing,
		sliceFailed,
		applyJobEvent,
		buildWindowGeometry,
		rangesFromMeta,
		sliceRequestFor
	} from '$lib/editor';
	import type { GizmoMode } from '$lib/scene/gizmos/types';
	import type { DisplayMode } from '$lib/settings/filter';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	// Brouillon local plus récent que la version serveur → proposer la
	// restauration après une fermeture accidentelle (l'éditeur le consommera).
	let pendingDraft = $state<DraftRecord | null>(null);

	// État de disposition (orchestrateur pur `lib/editor`) : panneau actif, mode
	// gizmo, sélection partagée scène↔liste.
	let ws = $state(initialWorkspace());

	// Modèle de scène (mutations en place → proxysées par `$state`, réactives).
	let tree = $state(new ObjectTree());
	let plates = $state(new PlateSet());
	// Maillages rendus dans la scène 3D : peuplés par l'import de modèles (T051,
	// câblé au prochain incrément) — vide à l'ouverture d'un projet neuf.
	let sceneObjects = $state<SceneObject[]>([]);

	// Plateau par défaut tant que le preset machine n'est pas résolu (le layout
	// et les dimensions réelles viennent des valeurs `printable_area` du preset).
	const bed = $derived(bedFromValues({}));

	// Onglet latéral : liste d'objets / plateaux, ou arbre de réglages.
	let sidebarTab = $state<'objects' | 'settings'>('objects');
	let settingsMode = $state<DisplayMode>('simple');
	let settingsValues = $state<Record<string, unknown>>({});
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

	onMount(() => {
		let alive = true;
		draftStore.pendingRestore(data.project.id, data.project.updated_at).then((d) => {
			if (alive) pendingDraft = d;
		});
		// Flux d'événements du compte (T065) : progression et fin des jobs.
		const sub: EventSubscription = subscribeEvents({ onEvent });
		return () => {
			alive = false;
			sub.close();
		};
	});

	function onEvent(event: ServerEvent) {
		const wasPreview = session.phase === 'preview';
		session = applyJobEvent(session, event);
		if (event.event === 'job.finished' && session.jobIds.includes(event.id)) {
			rawStats = event.stats;
		}
		// Bascule automatique vers l'aperçu au premier G-code produit.
		if (!wasPreview && session.phase === 'preview' && session.gcodeId) {
			ws = setPanel(ws, 'preview');
			void loadPreview(session.gcodeId);
		}
	}

	async function dismissDraft() {
		await draftStore.clear(data.project.id);
		pendingDraft = null;
	}

	function selectObject(id: string, additive: boolean) {
		ws = pick(ws, id, additive);
	}

	function changeGizmo(mode: GizmoMode) {
		ws = setGizmoMode(ws, mode);
	}

	function showPanel(panel: EditorPanel) {
		ws = setPanel(ws, panel);
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
			ws = setPanel(ws, 'preview');
		} catch (e) {
			session = sliceFailed(e instanceof ApiError ? e.message : 'échec du lancement du tranchage');
			ws = setPanel(ws, 'preview');
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
			data.project.active_presets
		);
		saveMessage =
			outcome.status === 'saved'
				? 'Projet sauvegardé.'
				: outcome.status === 'conflict'
					? 'Conflit : le projet a été modifié dans un autre onglet.'
					: outcome.message;
		return outcome;
	}
</script>

<div class="flex h-screen flex-col">
	<header
		class="flex items-center justify-between border-b border-gray-200 px-6 py-3 dark:border-gray-700"
	>
		<div class="flex items-center gap-4">
			<a href={resolve('/library')} class="text-sm text-blue-600 hover:underline">← Bibliothèque</a>
			<h1 class="text-lg font-semibold text-gray-900 dark:text-gray-100">{data.project.name}</h1>
		</div>
		<div class="flex items-center gap-3">
			<div class="flex overflow-hidden rounded border border-gray-300 dark:border-gray-600">
				<button
					class="px-3 py-1 text-sm {ws.panel === 'prepare'
						? 'bg-blue-600 text-white'
						: 'text-gray-700 dark:text-gray-200'}"
					onclick={() => showPanel('prepare')}>Préparer</button
				>
				<button
					class="px-3 py-1 text-sm {ws.panel === 'preview'
						? 'bg-blue-600 text-white'
						: 'text-gray-700 dark:text-gray-200'}"
					onclick={() => showPanel('preview')}>Aperçu</button
				>
			</div>
			<button
				class="rounded bg-emerald-600 px-3 py-1 text-sm text-white hover:bg-emerald-500 disabled:cursor-not-allowed disabled:opacity-40"
				onclick={sliceActive}
				disabled={!canSliceNow}
				title={canSliceNow ? 'Trancher le plateau actif' : 'Ajoutez un objet à trancher'}
			>
				Trancher
			</button>
			<SaveControls {save} />
			<!-- Export projet 3MF (FR-044) : ressource API backend, pas une route SvelteKit. -->
			<!-- eslint-disable svelte/no-navigation-without-resolve -->
			<a
				class="rounded bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-500"
				href="/api/projects/{data.project.id}/export/3mf"
				download
			>
				Exporter 3MF
			</a>
			<!-- eslint-enable svelte/no-navigation-without-resolve -->
		</div>
	</header>

	{#if pendingDraft}
		<div
			class="flex items-center justify-between gap-4 bg-amber-50 px-6 py-2 text-sm text-amber-900 dark:bg-amber-950 dark:text-amber-200"
			role="status"
		>
			<span>Un brouillon local plus récent existe pour ce projet.</span>
			<button onclick={dismissDraft} class="font-medium hover:underline">Ignorer</button>
		</div>
	{/if}

	{#if saveMessage}
		<div class="bg-gray-100 px-6 py-2 text-sm text-gray-700 dark:bg-gray-800 dark:text-gray-200">
			{saveMessage}
		</div>
	{/if}

	<div class="flex min-h-0 flex-1">
		<!-- Zone centrale : scène 3D (Préparer) ou aperçu G-code (Aperçu, T088). -->
		<main class="relative min-w-0 flex-1 bg-gray-50 dark:bg-gray-900">
			{#if ws.panel === 'prepare'}
				<div class="absolute left-3 top-3 z-10">
					<GizmoToolbar mode={ws.gizmoMode} onmode={changeGizmo} />
				</div>
				<Scene {bed} objects={sceneObjects} bind:selection={ws.selection} />
			{:else}
				<div class="flex h-full flex-col">
					{#if session.phase === 'slicing'}
						<div
							class="flex flex-1 flex-col items-center justify-center gap-3 p-6 text-gray-600 dark:text-gray-300"
						>
							<p>Tranchage en cours… ({session.jobPhase})</p>
							<div class="h-2 w-64 overflow-hidden rounded bg-gray-200 dark:bg-gray-700">
								<div
									class="h-full bg-blue-600 transition-all"
									style="width: {Math.round(session.progress * 100)}%"
								></div>
							</div>
							{#each session.warnings as w (w.key)}
								<p class="text-sm text-amber-600 dark:text-amber-400">⚠ {w.message}</p>
							{/each}
						</div>
					{:else if session.phase === 'error'}
						<div class="flex flex-1 items-center justify-center p-6 text-red-600 dark:text-red-400">
							<p>Échec du tranchage : {session.error}</p>
						</div>
					{:else if session.phase === 'preview' && previewGeometry && previewMeta}
						<div class="relative min-h-0 flex-1">
							<PreviewScene geometry={previewGeometry} {bed} />
							{#if previewMeta.layer_count > 1}
								<div
									class="absolute right-3 top-3 flex flex-col items-center gap-2 rounded bg-white/80 p-2 text-xs text-gray-700 dark:bg-gray-900/80 dark:text-gray-200"
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
							<div class="max-h-64 overflow-auto border-t border-gray-200 dark:border-gray-700">
								<StatsPanel stats={previewStats} />
							</div>
						{/if}
					{:else}
						<div
							class="flex flex-1 items-center justify-center p-6 text-gray-500 dark:text-gray-400"
						>
							<p>Aperçu G-code : tranchez la scène pour l'afficher.</p>
						</div>
					{/if}
				</div>
			{/if}
		</main>

		<!-- Panneau latéral : objets/plateaux ou réglages. -->
		<aside
			class="flex w-96 flex-col border-l border-gray-200 bg-white dark:border-gray-700 dark:bg-gray-950"
		>
			<div class="flex border-b border-gray-200 dark:border-gray-700">
				<button
					class="flex-1 px-3 py-2 text-sm {sidebarTab === 'objects'
						? 'border-b-2 border-blue-600 font-medium text-blue-600'
						: 'text-gray-600 dark:text-gray-300'}"
					onclick={() => (sidebarTab = 'objects')}>Objets</button
				>
				<button
					class="flex-1 px-3 py-2 text-sm {sidebarTab === 'settings'
						? 'border-b-2 border-blue-600 font-medium text-blue-600'
						: 'text-gray-600 dark:text-gray-300'}"
					onclick={() => (sidebarTab = 'settings')}>Réglages</button
				>
			</div>

			<div class="min-h-0 flex-1 overflow-auto">
				{#if sidebarTab === 'objects'}
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
					<SettingsTabs bind:mode={settingsMode} bind:values={settingsValues} />
				{/if}
			</div>
		</aside>
	</div>
</div>
