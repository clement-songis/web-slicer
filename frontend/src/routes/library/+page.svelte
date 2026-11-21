<script lang="ts">
	// Accueil / bibliothèque (T118, US6 ; parité orca-home.png) : barre latérale
	// (compte + Récent + OrcaCloud), deux cartes d'entrée (Nouveau projet / Ouvrir
	// un projet 3mf) et la grille « Récemment ouvert » des vignettes de projet.
	// La logique testable est pure (`lib/library/library.ts`) ; ce composant
	// orchestre transport (API projets) + disposition.
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { t, tr } from '$lib/i18n';
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
	import { appPreferences, currentPreferences, savePreferences } from '$lib/settings/preferences';
	import OpenProjectDialog from '$lib/projects/OpenProjectDialog.svelte';
	import {
		behaviourFromChoice,
		decideLoadMode,
		isProjectArchive,
		stashImportReport
	} from '$lib/projects/open-3mf';
	import AppSidebar from '$lib/nav/AppSidebar.svelte';
	import {
		sortByUpdated,
		formatDate,
		thumbnailUrl,
		filterProjects,
		limitRecent,
		relativeTime,
		sortProjects,
		type ProjectSort
	} from '$lib/library/library';
	import {
		isNameConflict,
		nameSuggestion,
		nameTakenMessage,
		uniqueProjectName
	} from '$lib/library/names';
	import type { ProjectResponse } from '$lib/api/types';
	import type { PageData } from './$types';
	import IconNew from '~icons/lucide/file-plus';
	import IconOpen from '~icons/lucide/download';
	import IconTrash from '~icons/lucide/trash-2';
	import IconSearch from '~icons/lucide/search';
	import IconFolder from '~icons/lucide/folder-open';

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

	// Recherche et tri (mémorisé dans le navigateur) ; la grille affiche la vue
	// filtrée, les opérations CRUD agissent toujours sur `projects`.
	const SORT_KEY = 'web-slicer.library-sort';
	let query = $state('');
	let sort = $state<ProjectSort>('updated');
	onMount(() => {
		const saved = localStorage.getItem(SORT_KEY);
		if (saved === 'name' || saved === 'created' || saved === 'updated') sort = saved;
	});
	const filtered = $derived(filterProjects(sortProjects(projects, sort), query));
	// Préférence « Nombre de projets récents » (Orca max_recent_count) : bornée à la
	// vue récente (tri par modification, sans recherche) ; « Tout afficher » lève la borne.
	let showAll = $state(false);
	const recent = $derived(
		limitRecent(
			filtered,
			$appPreferences.maxRecent,
			showAll || sort !== 'updated' || !!query.trim()
		)
	);
	const shown = $derived(recent.items);
	let now = $state(Date.now());
	onMount(() => {
		const timer = setInterval(() => (now = Date.now()), 60_000);
		return () => clearInterval(timer);
	});

	async function run(action: () => Promise<void>) {
		busy = true;
		error = null;
		try {
			await action();
		} catch (err) {
			error = err instanceof ApiError ? err.message : tr('Unexpected error');
		} finally {
			busy = false;
		}
	}

	// « Nouveau projet » : crée un projet par défaut et ouvre l'éditeur dessus.
	function createAndOpen() {
		return run(async () => {
			// Nom unique par compte (phase 32) : « Sans titre », « Sans titre (2) »…
			// Un projet créé ailleurs entre-temps (autre onglet) → nouvel essai.
			const taken = projects.map((p) => p.name);
			for (let attempt = 0; ; attempt++) {
				const name = uniqueProjectName(tr('Untitled'), taken);
				try {
					const project = await createProject({ name });
					await goto(resolve('/projects/[id]', { id: project.id }));
					return;
				} catch (err) {
					if (!isNameConflict(err) || attempt >= 4) throw err;
					taken.push(name);
				}
			}
		});
	}

	// « Ouvrir un projet » (T090) : un `.3mf` projet ou un modèle 3D → nouveau
	// projet, puis on ouvre l'éditeur dessus.
	let importInput: HTMLInputElement | null = null;
	const IMPORT_ACCEPT = '.stl,.obj,.3mf,.oltp,.step,.stp,.amf,.svg,.drc';

	// Ouverture d'un 3MF (lot 34) : « Ouvrir comme projet / Importer la géométrie
	// seule » selon project_load_behaviour. Depuis la bibliothèque il n'y a rien
	// en cours : « Demander si utile » ouvre donc comme projet sans question
	// (Orca : plateau vide → pas de ProjectDropDialog).
	let open3mf = $state<File | null>(null);

	function importFile(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		input.value = '';
		if (!file) return;
		if (!isAccepted(file.name)) {
			error = tr('Unsupported format: {name}', { name: file.name });
			return;
		}
		if (isProjectArchive(file.name)) {
			const decision = decideLoadMode(currentPreferences().projectLoadBehaviour, false);
			if (decision === 'ask') {
				open3mf = file;
				return;
			}
			return run(() => openProjectWithLoad(file, decision));
		}
		return run(() => openProjectWithLoad(file));
	}

	async function openProjectWithLoad(file: File, load?: 'project' | 'geometry') {
		open3mf = null;
		const project = await importProject(file, load);
		if (project.import) stashImportReport(project.id, project.import);
		await goto(resolve('/projects/[id]', { id: project.id }));
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
			try {
				const updated = await renameProject(id, name);
				projects = sortByUpdated(projects.map((p) => (p.id === id ? updated : p)));
				renamingId = null;
			} catch (err) {
				// Nom déjà pris : message clair, le champ reste ouvert et reçoit le
				// nom libre proposé par le serveur (lot 32, details.suggestion).
				if (isNameConflict(err)) {
					const suggestion = nameSuggestion(err);
					if (suggestion) renameValue = suggestion;
					throw new ApiError(409, 'conflict', nameTakenMessage(name, suggestion));
				}
				throw err;
			}
		});
	}
</script>

<svelte:head><title>{$t('Recent')} — Web-Slicer</title></svelte:head>

<div class="flex h-screen bg-surface text-content">
	<AppSidebar email={data.user.email} active="recent" />

	<!-- Zone principale -->
	<main class="flex-1 overflow-auto p-8">
		<h1 class="ws-sr-only">{$t('Project library')}</h1>
		<!-- Deux cartes d'entrée -->
		<div class="flex flex-wrap gap-4">
			<button
				onclick={createAndOpen}
				data-testid="new-project"
				disabled={busy}
				class="flex w-80 items-center gap-4 rounded-lg border border-border bg-surface-raised p-5 text-left transition-colors hover:border-primary focus-visible:ring-2 focus-visible:ring-accent focus-visible:outline-none disabled:opacity-50"
			>
				<span
					class="flex h-14 w-14 shrink-0 items-center justify-center rounded-lg border border-primary text-primary"
				>
					<IconNew class="h-7 w-7" />
				</span>
				<span class="flex flex-col">
					<span class="font-medium text-content">{$t('New Project')}</span>
					<span class="text-sm text-content-muted">{$t('Create a new project')}</span>
				</span>
			</button>
			<button
				onclick={() => importInput?.click()}
				disabled={busy}
				class="flex w-80 items-center gap-4 rounded-lg border border-border bg-surface-raised p-5 text-left transition-colors hover:border-primary focus-visible:ring-2 focus-visible:ring-accent focus-visible:outline-none disabled:opacity-50"
			>
				<span
					class="flex h-14 w-14 shrink-0 items-center justify-center rounded-lg border border-primary text-primary"
				>
					<IconOpen class="h-7 w-7" />
				</span>
				<span class="flex flex-col">
					<span class="font-medium text-content">{$t('Open Project')}</span>
					<span class="text-sm text-content-muted">{$t('.3mf project or 3D model')}</span>
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

		<!-- Récemment ouvert : recherche, tri, mode suppression. -->
		<div class="mt-8 flex flex-wrap items-center gap-3">
			<h2 class="ws-section-title">{$t('Recently opened')}</h2>
			{#if projects.length}
				<span class="ws-badge tabular-nums" data-testid="library-count"
					>{query.trim() ? `${shown.length} / ${projects.length}` : projects.length}</span
				>
				<button
					onclick={() => (removeMode = !removeMode)}
					class="ws-btn ws-btn-sm {removeMode ? 'ws-btn-danger' : 'ws-btn-ghost'}"
					aria-pressed={removeMode}
				>
					<IconTrash class="h-3.5 w-3.5" />
					{$t('Delete')}
				</button>
				<div class="ml-auto flex items-center gap-2">
					<label class="relative">
						<span class="ws-sr-only">{$t('Search projects')}</span>
						<IconSearch
							class="pointer-events-none absolute top-1/2 left-2 h-3.5 w-3.5 -translate-y-1/2 text-content-subtle"
							aria-hidden="true"
						/>
						<input
							type="search"
							class="ws-field ws-field-sm w-56 pl-7"
							placeholder={$t('Search projects…')}
							bind:value={query}
							data-testid="library-search"
						/>
					</label>
					<label class="flex items-center gap-1.5 text-xs text-content-muted">
						{$t('Sort')}
						<select
							class="ws-field ws-field-sm"
							bind:value={sort}
							onchange={() => localStorage.setItem(SORT_KEY, sort)}
							data-testid="library-sort"
						>
							<option value="updated">{$t('Recently modified')}</option>
							<option value="created">{$t('Recently created')}</option>
							<option value="name">{$t('Name (A → Z)')}</option>
						</select>
					</label>
				</div>
			{/if}
		</div>

		{#if projects.length === 0}
			<div
				class="ws-empty mt-4 rounded-lg border border-dashed border-border"
				data-testid="library-empty"
			>
				<IconFolder class="h-8 w-8 text-content-subtle" aria-hidden="true" />
				<p class="font-medium text-content">{$t('No projects yet')}</p>
				<p>
					{$t(
						'Create a project or open a .3mf / .stl file: it will appear here with its thumbnail.'
					)}
				</p>
			</div>
		{:else if filtered.length === 0}
			<div
				class="ws-empty mt-4 rounded-lg border border-dashed border-border"
				data-testid="library-no-match"
			>
				<p>{$t('No project matches “{query}”.', { query: query.trim() })}</p>
				<button type="button" class="ws-btn ws-btn-sm ws-btn-secondary" onclick={() => (query = '')}
					>{$t('Clear search')}</button
				>
			</div>
		{:else}
			<ul class="mt-4 grid grid-cols-[repeat(auto-fill,minmax(11rem,1fr))] gap-5">
				{#each shown as project (project.id)}
					{@const thumb = thumbnailUrl(project)}
					<li class="group relative flex flex-col gap-2">
						<a
							href={resolve('/projects/[id]', { id: project.id })}
							class="block aspect-square overflow-hidden rounded-lg border border-border bg-overlay hover:border-primary"
							aria-label={$t('Open {name}', { name: project.name })}
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
								title={$t('Delete {name}', { name: project.name })}
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
								title={$t('Open {name}', { name: project.name })}
							>
								{project.name}
							</button>
						{/if}
						<p class="text-xs text-content-subtle">
							{$t('Modified||project')}
							<time datetime={project.updated_at} title={formatDate(project.updated_at)}
								>{relativeTime(project.updated_at, now)}</time
							>
						</p>
						<div class="flex gap-3 text-xs text-content-muted">
							<!-- Actions toujours lisibles (contraste du texte secondaire, pas d'opacité
							     réduite) ; l'accent au survol signale qu'elles sont cliquables. -->
							<button
								onclick={() => startRename(project)}
								disabled={busy}
								class="hover:text-primary hover:underline"
							>
								{$t('Rename')}
							</button>
							<button
								onclick={() => duplicate(project.id)}
								disabled={busy}
								class="hover:text-primary hover:underline"
							>
								{$t('Duplicate')}
							</button>
						</div>
					</li>
				{/each}
			</ul>
			{#if recent.hidden > 0}
				<div class="mt-4 flex items-center gap-3 text-sm text-content-muted">
					<span data-testid="library-hidden"
						>{$t(
							recent.hidden > 1
								? '{n} older projects hidden (“Recent projects shown” preference).'
								: '{n} older project hidden (“Recent projects shown” preference).',
							{ n: recent.hidden }
						)}</span
					>
					<button
						type="button"
						class="ws-btn ws-btn-sm ws-btn-secondary"
						data-testid="library-show-all"
						onclick={() => (showAll = true)}>{$t('Show all')}</button
					>
				</div>
			{/if}
		{/if}
	</main>
</div>

<!-- Ouverture d'un 3MF : projet ou géométrie seule (lot 34). -->
<OpenProjectDialog
	open={open3mf !== null}
	filename={open3mf?.name ?? ''}
	onconfirm={(mode, remember) => {
		if (remember)
			savePreferences({ ...currentPreferences(), projectLoadBehaviour: behaviourFromChoice(mode) });
		const file = open3mf;
		if (file) void run(() => openProjectWithLoad(file, mode));
	}}
	oncancel={() => (open3mf = null)}
/>
