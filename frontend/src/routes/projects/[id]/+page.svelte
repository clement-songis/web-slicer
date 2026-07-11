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
	import { initialWorkspace, pick, setGizmoMode, setPanel, type EditorPanel } from '$lib/editor';
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

	onMount(async () => {
		pendingDraft = await draftStore.pendingRestore(data.project.id, data.project.updated_at);
	});

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
				<div class="flex h-full items-center justify-center p-6 text-gray-500 dark:text-gray-400">
					<p>Aperçu G-code : tranchez la scène pour l'afficher (branchement T088).</p>
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
