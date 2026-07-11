<script lang="ts">
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
	import type { ProjectResponse } from '$lib/api/types';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	// Semé une fois depuis le `load` client (pas d'invalidation ensuite) : la
	// liste est ensuite gérée localement au fil des opérations CRUD.
	let projects = $state<ProjectResponse[]>([]);
	onMount(() => {
		projects = data.projects;
	});

	let newName = $state('');
	let renamingId = $state<string | null>(null);
	let renameValue = $state('');
	let error = $state<string | null>(null);
	let busy = $state(false);

	/** Réordonne par date de mise à jour décroissante (comme le backend). */
	function sorted(list: ProjectResponse[]): ProjectResponse[] {
		return [...list].sort((a, b) => b.updated_at.localeCompare(a.updated_at));
	}

	function formatDate(rfc3339: string): string {
		const d = new Date(rfc3339);
		return Number.isNaN(d.getTime()) ? rfc3339 : d.toLocaleString();
	}

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

	function create() {
		const name = newName.trim();
		if (!name) return;
		return run(async () => {
			const project = await createProject({ name });
			projects = sorted([project, ...projects]);
			newName = '';
		});
	}

	// Import (T090) : un `.3mf` projet ou un modèle 3D → nouveau projet, puis on
	// ouvre l'éditeur dessus.
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
			projects = sorted([copy, ...projects]);
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
			projects = sorted(projects.map((p) => (p.id === id ? updated : p)));
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

<header
	class="flex items-center justify-between border-b border-gray-200 px-6 py-4 dark:border-gray-700"
>
	<h1 class="text-lg font-semibold text-gray-900 dark:text-gray-100">Ma bibliothèque</h1>
	<div class="flex items-center gap-4 text-sm">
		<ThemeToggle />
		<span class="text-gray-600 dark:text-gray-400">{data.user.email}</span>
		<button onclick={signOut} class="text-blue-600 hover:underline">Déconnexion</button>
	</div>
</header>

<main class="mx-auto max-w-4xl p-6">
	<form class="mb-6 flex gap-2" onsubmit={(e) => (e.preventDefault(), create())}>
		<input
			type="text"
			bind:value={newName}
			placeholder="Nom du nouveau projet"
			class="flex-1 rounded border border-gray-300 px-3 py-2 dark:border-gray-600 dark:bg-gray-700 dark:text-gray-100"
		/>
		<button
			type="submit"
			disabled={busy || !newName.trim()}
			class="rounded bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-700 disabled:opacity-50"
		>
			Nouveau projet
		</button>
		<button
			type="button"
			disabled={busy}
			onclick={() => importInput?.click()}
			class="rounded border border-gray-300 px-4 py-2 font-medium text-gray-700 hover:bg-gray-100 disabled:opacity-50 dark:border-gray-600 dark:text-gray-200 dark:hover:bg-gray-800"
			title="Importer un projet .3mf ou un modèle 3D"
		>
			Importer…
		</button>
		<input
			bind:this={importInput}
			type="file"
			accept={IMPORT_ACCEPT}
			class="hidden"
			onchange={importFile}
		/>
	</form>

	{#if error}
		<p class="mb-4 text-sm text-red-600" role="alert">{error}</p>
	{/if}

	{#if projects.length === 0}
		<p class="text-gray-500 dark:text-gray-400">Aucun projet pour le moment.</p>
	{:else}
		<ul class="grid gap-3 sm:grid-cols-2">
			{#each projects as project (project.id)}
				<li class="rounded-lg border border-gray-200 p-4 dark:border-gray-700">
					{#if renamingId === project.id}
						<input
							type="text"
							bind:value={renameValue}
							onblur={() => commitRename(project.id)}
							onkeydown={(e) => e.key === 'Enter' && commitRename(project.id)}
							class="mb-2 w-full rounded border border-gray-300 px-2 py-1 dark:border-gray-600 dark:bg-gray-700 dark:text-gray-100"
						/>
					{:else}
						<a
							href={resolve('/projects/[id]', { id: project.id })}
							class="block truncate font-medium text-gray-900 hover:text-blue-600 dark:text-gray-100"
						>
							{project.name}
						</a>
					{/if}
					<p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
						Modifié le {formatDate(project.updated_at)}
					</p>
					<div class="mt-3 flex gap-3 text-sm">
						<a
							href={resolve('/projects/[id]', { id: project.id })}
							class="text-blue-600 hover:underline">Ouvrir</a
						>
						<button
							onclick={() => startRename(project)}
							disabled={busy}
							class="text-gray-600 hover:underline dark:text-gray-300"
						>
							Renommer
						</button>
						<button
							onclick={() => duplicate(project.id)}
							disabled={busy}
							class="text-gray-600 hover:underline dark:text-gray-300"
						>
							Dupliquer
						</button>
						<button
							onclick={() => remove(project.id)}
							disabled={busy}
							class="text-red-600 hover:underline"
						>
							Supprimer
						</button>
					</div>
				</li>
			{/each}
		</ul>
	{/if}
</main>
