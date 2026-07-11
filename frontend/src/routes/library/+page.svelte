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
	import { ThemeToggle } from '$lib/theme';
	import { logout } from '$lib/api/session';
	import { sortByUpdated, formatDate, thumbnailUrl } from '$lib/library/library';
	import type { ProjectResponse } from '$lib/api/types';
	import type { PageData } from './$types';
	import IconNew from '~icons/lucide/file-plus';
	import IconOpen from '~icons/lucide/download';
	import IconClock from '~icons/lucide/clock';
	import IconExternal from '~icons/lucide/external-link';
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

	async function signOut() {
		await run(async () => {
			await logout();
			await goto(resolve('/login'));
		});
	}
</script>

<div class="flex h-screen bg-surface text-content">
	<!-- Barre latérale : compte + navigation (parité orca-home.png) -->
	<aside class="flex w-64 shrink-0 flex-col border-r border-border bg-surface-raised">
		<div class="flex flex-col items-center gap-2 border-b border-border px-4 py-6">
			<span class="text-sm font-semibold text-content">Mon compte</span>
			<div
				class="flex h-16 w-16 items-center justify-center rounded-xl bg-overlay text-2xl font-semibold text-content-muted"
				aria-hidden="true"
			>
				{data.user.email.charAt(0).toUpperCase()}
			</div>
			<span class="max-w-full truncate text-xs text-content-muted" title={data.user.email}>
				{data.user.email}
			</span>
			<button onclick={signOut} disabled={busy} class="text-xs text-primary hover:underline">
				Déconnexion
			</button>
		</div>
		<nav class="flex flex-col gap-1 p-2 text-sm">
			<span
				class="flex items-center gap-2 rounded bg-primary/10 px-3 py-2 font-medium text-primary"
			>
				<IconClock class="h-4 w-4" /> Récent
			</span>
			<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
			<a
				href="https://github.com/SoftFever/OrcaSlicer"
				target="_blank"
				rel="noreferrer"
				class="flex items-center justify-between rounded px-3 py-2 text-content-muted hover:bg-overlay"
			>
				<span>OrcaSlicer</span>
				<IconExternal class="h-3.5 w-3.5" />
			</a>
		</nav>
		<div class="mt-auto flex items-center justify-between border-t border-border px-3 py-3">
			<span class="text-xs text-content-subtle">Web-Slicer</span>
			<ThemeToggle />
		</div>
	</aside>

	<!-- Zone principale -->
	<main class="flex-1 overflow-auto p-8">
		<!-- Deux cartes d'entrée -->
		<div class="flex flex-wrap gap-4">
			<button
				onclick={createAndOpen}
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
					<span class="text-sm text-content-muted">3mf</span>
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
			<h2 class="text-base font-semibold text-content">Récemment ouvert</h2>
			{#if projects.length}
				<button
					onclick={() => (removeMode = !removeMode)}
					class="flex items-center gap-1 rounded-full px-3 py-1 text-xs {removeMode
						? 'bg-danger-soft text-danger-content'
						: 'bg-overlay text-content-muted hover:bg-overlay/70'}"
				>
					<IconTrash class="h-3.5 w-3.5" /> Supprimer
				</button>
			{/if}
		</div>

		{#if projects.length === 0}
			<p class="mt-4 text-content-subtle">Aucun projet pour le moment.</p>
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
								class="absolute right-2 top-2 flex h-7 w-7 items-center justify-center rounded-full bg-danger text-white shadow hover:bg-danger/90"
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
								class="w-full rounded border border-border-strong bg-surface-raised px-2 py-1 text-sm text-content"
							/>
						{:else}
							<button
								ondblclick={() => startRename(project)}
								class="truncate text-left text-sm font-medium text-content"
								title={project.name}
							>
								{project.name}
							</button>
						{/if}
						<p class="text-xs text-content-subtle">Modifié le {formatDate(project.updated_at)}</p>
						<div class="flex gap-3 text-xs text-content-muted opacity-0 group-hover:opacity-100">
							<button onclick={() => startRename(project)} disabled={busy} class="hover:underline">
								Renommer
							</button>
							<button onclick={() => duplicate(project.id)} disabled={busy} class="hover:underline">
								Dupliquer
							</button>
						</div>
					</li>
				{/each}
			</ul>
		{/if}
	</main>
</div>
