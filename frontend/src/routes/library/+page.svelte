<script lang="ts">
	// Accueil / bibliothèque (T118, US6 ; parité orca-home.png) : barre latérale
	// (compte + Récent + OrcaCloud), deux cartes d'entrée (Nouveau projet / Ouvrir
	// un projet 3mf) et la grille « Récemment ouvert » des vignettes de projet.
	// La logique testable est pure (`lib/library/library.ts`) ; ce composant
	// orchestre transport (API projets) + disposition.
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { ApiError } from '$lib/api/client';
	import {
		createProject,
		deleteProject,
		duplicateProject,
		renameProject,
		importProject
	} from '$lib/api/projects';
	import { isAccepted } from '$lib/editor';
	import AppSidebar from '$lib/nav/AppSidebar.svelte';
	import { sortByUpdated, formatDate, thumbnailUrl } from '$lib/library/library';
	import type { ProjectResponse } from '$lib/api/types';
	import type { PageData } from './$types';
	import IconNew from '~icons/lucide/file-plus';
	import IconOpen from '~icons/lucide/download';
	import IconTrash from '~icons/lucide/trash-2';

	let { data }: { data: PageData } = $props();

	// Semé une fois depuis le `load` client (pas d'invalidation ensuite) : la
	// liste est ensuite gérée localement au fil des opérations CRUD.
	let projects = $state<ProjectResponse[]>([]);
	onMount(() => {
		projects = sortByUpdated(data.projects);
	});

	let renamingId = $state<string | null>(null);
	let renameValue = $state('');
	let error = $state<string | null>(null);
	let busy = $state(false);
	// Mode suppression (pastille « Supprimer » de la parité) : révèle les
	// boutons de suppression sur chaque vignette.
	let removeMode = $state(false);

	async function run(action: () => Promise<void>) {
		busy = true;
		error = null;
		try {
			await action();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Erreur inattendue';
		} finally {
			busy = false;
		}
	}

	// « Nouveau projet » : crée un projet par défaut et ouvre l'éditeur dessus.
	function createAndOpen() {
		return run(async () => {
			const project = await createProject({ name: 'Sans titre' });
			await goto(resolve('/projects/[id]', { id: project.id }));
		});
	}

	// « Ouvrir un projet » (T090) : un `.3mf` projet ou un modèle 3D → nouveau
	// projet, puis on ouvre l'éditeur dessus.
	let importInput: HTMLInputElement | null = null;
	const IMPORT_ACCEPT = '.stl,.obj,.3mf,.oltp,.step,.stp,.amf,.svg,.drc';

	function importFile(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		input.value = '';
		if (!file) return;
		if (!isAccepted(file.name)) {
			error = `Format non supporté : ${file.name}`;
			return;
		}
		return run(async () => {
			const project = await importProject(file);
			await goto(resolve('/projects/[id]', { id: project.id }));
		});
	}

	function duplicate(id: string) {
		return run(async () => {
			const copy = await duplicateProject(id);
			projects = sortByUpdated([copy, ...projects]);
		});
	}

	function remove(id: string) {
		return run(async () => {
			await deleteProject(id);
			projects = projects.filter((p) => p.id !== id);
		});
	}

	function startRename(project: ProjectResponse) {
		renamingId = project.id;
		renameValue = project.name;
	}

	function commitRename(id: string) {
		const name = renameValue.trim();
		if (!name) {
			renamingId = null;
			return;
		}
		return run(async () => {
			const updated = await renameProject(id, name);
			projects = sortByUpdated(projects.map((p) => (p.id === id ? updated : p)));
			renamingId = null;
		});
	}
</script>

<svelte:head><title>Récent — Web-Slicer</title></svelte:head>

<div class="flex h-screen bg-surface text-content">
	<AppSidebar email={data.user.email} active="recent" />

	<!-- Zone principale -->
	<main class="flex-1 overflow-auto p-8">
		<h1 class="ws-sr-only">Bibliothèque de projets</h1>
		<!-- Deux cartes d'entrée -->
		<div class="flex flex-wrap gap-4">
			<button
				onclick={createAndOpen}
				data-testid="new-project"
				disabled={busy}
				class="flex w-72 items-center gap-4 rounded-lg border border-border bg-surface-raised p-5 text-left hover:border-primary disabled:opacity-50"
			>
				<span
					class="flex h-14 w-14 shrink-0 items-center justify-center rounded-lg border border-primary text-primary"
				>
					<IconNew class="h-7 w-7" />
				</span>
				<span class="flex flex-col">
					<span class="font-medium text-content">Nouveau projet</span>
					<span class="text-sm text-content-muted">Créer un nouveau projet</span>
				</span>
			</button>
			<button
				onclick={() => importInput?.click()}
				disabled={busy}
				class="flex w-72 items-center gap-4 rounded-lg border border-border bg-surface-raised p-5 text-left hover:border-primary disabled:opacity-50"
			>
				<span
					class="flex h-14 w-14 shrink-0 items-center justify-center rounded-lg border border-primary text-primary"
				>
					<IconOpen class="h-7 w-7" />
				</span>
				<span class="flex flex-col">
					<span class="font-medium text-content">Ouvrir un projet</span>
					<span class="text-sm text-content-muted">Projet .3mf ou modèle 3D</span>
				</span>
			</button>
			<input
				bind:this={importInput}
				type="file"
				accept={IMPORT_ACCEPT}
				class="hidden"
				onchange={importFile}
			/>
		</div>

		{#if error}
			<p class="mt-4 text-sm text-danger" role="alert">{error}</p>
		{/if}

		<!-- Récemment ouvert -->
		<div class="mt-8 flex items-center gap-3">
			<h2 class="ws-section-title">Récemment ouvert</h2>
			{#if projects.length}
				<button
					onclick={() => (removeMode = !removeMode)}
					class="ws-btn ws-btn-sm {removeMode ? 'ws-btn-danger' : 'ws-btn-ghost'}"
					aria-pressed={removeMode}
				>
					<IconTrash class="h-3.5 w-3.5" /> Supprimer
				</button>
			{/if}
		</div>

		{#if projects.length === 0}
			<p class="ws-empty mt-4 rounded-lg border border-dashed border-border">
				Aucun projet pour le moment.
			</p>
		{:else}
			<ul class="mt-4 grid grid-cols-[repeat(auto-fill,minmax(11rem,1fr))] gap-5">
				{#each projects as project (project.id)}
					{@const thumb = thumbnailUrl(project)}
					<li class="group relative flex flex-col gap-2">
						<a
							href={resolve('/projects/[id]', { id: project.id })}
							class="block aspect-square overflow-hidden rounded-lg border border-border bg-overlay hover:border-primary"
							aria-label="Ouvrir {project.name}"
						>
							{#if thumb}
								<img src={thumb} alt="" class="h-full w-full object-cover" />
							{:else}
								<span class="flex h-full w-full items-center justify-center text-content-subtle">
									<IconNew class="h-10 w-10 opacity-40" />
								</span>
							{/if}
						</a>
						{#if removeMode}
							<button
								onclick={() => remove(project.id)}
								disabled={busy}
								title="Supprimer {project.name}"
								class="ws-btn ws-btn-sm ws-btn-danger absolute right-2 top-2 w-7 justify-center"
							>
								<IconTrash class="h-4 w-4" />
							</button>
						{/if}
						{#if renamingId === project.id}
							<!-- svelte-ignore a11y_autofocus -->
							<input
								type="text"
								bind:value={renameValue}
								autofocus
								onblur={() => commitRename(project.id)}
								onkeydown={(e) => e.key === 'Enter' && commitRename(project.id)}
								class="ws-field w-full"
							/>
						{:else}
							<!-- Le nom ouvre le projet comme la vignette (clic attendu) ; renommer
							     passe par l'action « Renommer » (un seul lien par carte). -->
							<button
								type="button"
								onclick={() => goto(resolve('/projects/[id]', { id: project.id }))}
								class="truncate text-left text-sm font-medium text-content hover:underline"
								title="Ouvrir {project.name}"
							>
								{project.name}
							</button>
						{/if}
						<p class="text-xs text-content-subtle">Modifié le {formatDate(project.updated_at)}</p>
						<div class="flex gap-3 text-xs text-content-muted">
							<!-- Actions toujours lisibles (contraste du texte secondaire, pas d'opacité
							     réduite) ; l'accent au survol signale qu'elles sont cliquables. -->
							<button
								onclick={() => startRename(project)}
								disabled={busy}
								class="hover:text-primary hover:underline"
							>
								Renommer
							</button>
							<button
								onclick={() => duplicate(project.id)}
								disabled={busy}
								class="hover:text-primary hover:underline"
							>
								Dupliquer
							</button>
						</div>
					</li>
				{/each}
			</ul>
		{/if}
	</main>
</div>
