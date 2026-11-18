<script lang="ts" module>
	// Dossier affiché par projet : conservé si le panneau est remonté (changement de
	// langue, retour sur l'onglet Projet).
	import { SvelteMap } from 'svelte/reactivity';
	const lastFolder = new SvelteMap<string, string>();
</script>

<script lang="ts">
	// Fichiers joints au projet (Orca : onglet « Fichiers auxiliaires » du
	// panneau Projet, dossier Auxiliaries/ du 3MF), intégré à l'onglet Projet.
	// Quatre dossiers d'Orca à
	// gauche (compteurs), contenu du dossier à droite : vignettes des images,
	// téléversement (bouton ou glisser-déposer), ouverture, suppression
	// confirmée. Les fichiers partent avec l'export 3MF du projet.
	import { onMount, untrack } from 'svelte';
	import {
		auxiliaryUrl,
		deleteAuxiliary,
		listAuxiliaries,
		putAuxiliary
	} from '$lib/api/auxiliaries';
	import { ApiError } from '$lib/api/client';
	import {
		AUX_FOLDERS,
		AUX_MAX_BYTES,
		formatSize,
		groupAuxiliaries,
		isPreviewableImage,
		safeAuxName,
		uniqueAuxName,
		type AuxFile,
		type AuxFolder
	} from './auxiliaries';
	import IconPlus from '~icons/lucide/plus';
	import IconX from '~icons/lucide/x';
	import IconExternal from '~icons/lucide/external-link';

	interface Props {
		projectId: string;
		/** Nombre total de fichiers (après chaque rechargement de la liste). */
		oncount?: (n: number) => void;
	}

	let { projectId, oncount }: Props = $props();

	let folder = $state<AuxFolder>(
		untrack(() => lastFolder.get(projectId) as AuxFolder | undefined) ?? 'Model Pictures'
	);
	$effect(() => {
		lastFolder.set(projectId, folder);
	});
	let files = $state<Record<AuxFolder, AuxFile[]>>(groupAuxiliaries([]));
	let loading = $state(false);
	let busy = $state(0);
	let error = $state<string | null>(null);
	let notice = $state<string | null>(null);
	let confirmDelete = $state<string | null>(null);
	let dragOver = $state(false);
	let input = $state<HTMLInputElement | null>(null);
	// Contournement du cache des vignettes après remplacement d'un fichier.
	let stamp = $state(0);

	const current = $derived(files[folder]);
	const meta = $derived(AUX_FOLDERS.find((f) => f.id === folder)!);
	const total = $derived(Object.values(files).reduce((n, f) => n + f.length, 0));

	async function refresh() {
		loading = true;
		try {
			files = groupAuxiliaries(await listAuxiliaries(projectId));
			oncount?.(Object.values(files).reduce((n, f) => n + f.length, 0));
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Liste des fichiers indisponible.';
		} finally {
			loading = false;
		}
	}
	onMount(() => {
		void refresh();
	});

	async function upload(list: FileList | File[] | null) {
		if (!list || !list.length) return;
		error = null;
		notice = null;
		const target = folder;
		const taken = files[target].map((f) => f.name);
		let added = 0;
		for (const file of Array.from(list)) {
			if (file.size > AUX_MAX_BYTES) {
				error = `« ${file.name} » dépasse 50 Mo.`;
				continue;
			}
			const name = uniqueAuxName(safeAuxName(file.name), taken);
			taken.push(name);
			busy++;
			try {
				await putAuxiliary(projectId, target, name, file);
				added++;
			} catch (e) {
				error = `« ${file.name} » : ${e instanceof ApiError ? e.message : 'échec du téléversement'}`;
			} finally {
				busy--;
			}
		}
		stamp++;
		await refresh();
		if (added) notice = `${added} fichier${added > 1 ? 's' : ''} ajouté${added > 1 ? 's' : ''}.`;
	}

	async function remove(file: AuxFile) {
		confirmDelete = null;
		error = null;
		busy++;
		try {
			await deleteAuxiliary(projectId, file.folder, file.name);
			notice = `« ${file.name} » supprimé.`;
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Suppression impossible.';
		} finally {
			busy--;
		}
		await refresh();
	}

	function onDrop(e: DragEvent) {
		e.preventDefault();
		dragOver = false;
		void upload(e.dataTransfer?.files ?? null);
	}
</script>

<!-- Liens vers les fichiers servis par l'API (pas des routes SvelteKit). -->
<!-- eslint-disable svelte/no-navigation-without-resolve -->
<section class="flex flex-col gap-3" aria-labelledby="aux-title" data-testid="aux-panel">
	<header>
		<h3 id="aux-title" class="ws-section-title">Fichiers joints au projet</h3>
		<p class="text-xs text-content-subtle">
			Photos, nomenclature, guide d'assemblage… enregistrés avec le projet et inclus dans l'export
			3MF (dossier Auxiliaries, comme Orca).
		</p>
	</header>
	<div class="flex min-h-72 gap-4 rounded-md border border-border bg-surface-raised p-3">
		<nav class="flex w-48 shrink-0 flex-col gap-1" aria-label="Dossiers">
			{#each AUX_FOLDERS as f (f.id)}
				<button
					type="button"
					class="flex items-center justify-between rounded-md px-3 py-2 text-left text-sm {folder ===
					f.id
						? 'bg-primary/10 font-medium text-primary-text'
						: 'text-content-muted hover:bg-overlay'}"
					aria-current={folder === f.id ? 'true' : undefined}
					data-testid="aux-folder"
					data-folder={f.id}
					onclick={() => {
						folder = f.id;
						confirmDelete = null;
					}}
				>
					<span>{f.label}</span>
					<span class="ws-badge tabular-nums" data-testid="aux-folder-count"
						>{files[f.id].length}</span
					>
				</button>
			{/each}
			<p class="mt-auto px-3 text-xs text-content-subtle" data-testid="aux-total">
				{total} fichier{total > 1 ? 's' : ''} au total
			</p>
		</nav>
		<div
			role="region"
			class="flex min-w-0 flex-1 flex-col gap-3 rounded-md border-2 border-dashed p-3 {dragOver
				? 'border-primary bg-primary/5'
				: 'border-transparent'}"
			aria-label={meta.label}
			ondragover={(e) => {
				e.preventDefault();
				dragOver = true;
			}}
			ondragleave={() => (dragOver = false)}
			ondrop={onDrop}
			data-testid="aux-dropzone"
		>
			<div class="flex items-start justify-between gap-3">
				<div>
					<h3 class="text-sm font-medium">{meta.label}</h3>
					<p class="text-xs text-content-subtle">{meta.hint} Glissez des fichiers ici ou :</p>
				</div>
				<button
					type="button"
					class="ws-btn ws-btn-sm ws-btn-primary shrink-0"
					data-testid="aux-add"
					disabled={busy > 0}
					onclick={() => input?.click()}
					><IconPlus class="h-3.5 w-3.5" aria-hidden="true" />Ajouter des fichiers…</button
				>
				<input
					bind:this={input}
					type="file"
					multiple
					accept={meta.accept || undefined}
					class="hidden"
					data-testid="aux-input"
					onchange={(e) => {
						void upload(e.currentTarget.files);
						e.currentTarget.value = '';
					}}
				/>
			</div>
			{#if error}
				<p class="text-xs text-danger-text" role="alert" data-testid="aux-error">{error}</p>
			{:else if notice}
				<p class="text-xs text-success-text" role="status" data-testid="aux-notice">{notice}</p>
			{/if}
			{#if busy > 0}
				<p class="text-xs text-content-subtle" role="status">Téléversement…</p>
			{/if}
			{#if current.length === 0}
				<div
					class="flex flex-1 items-center justify-center text-sm text-content-subtle"
					data-testid="aux-empty"
				>
					{loading ? 'Chargement…' : 'Aucun fichier dans ce dossier.'}
				</div>
			{:else}
				<ul
					class="grid grid-cols-[repeat(auto-fill,minmax(9.5rem,1fr))] gap-3"
					data-testid="aux-files"
				>
					{#each current as file (file.name)}
						{@const url = auxiliaryUrl(projectId, file.folder, file.name)}
						<li
							class="group relative flex flex-col overflow-hidden rounded-md border border-border bg-surface-raised"
							data-testid="aux-file"
							data-name={file.name}
						>
							<a
								href={url}
								target="_blank"
								rel="noopener"
								class="block aspect-[4/3] bg-surface-sunken"
								title="Ouvrir « {file.name} »"
							>
								{#if isPreviewableImage(file.name)}
									<img
										src={`${url}?v=${stamp}`}
										alt={file.name}
										loading="lazy"
										class="h-full w-full object-cover"
										data-testid="aux-thumb"
									/>
								{:else}
									<span
										class="flex h-full items-center justify-center font-mono text-xs uppercase text-content-subtle"
									>
										{file.name.includes('.') ? file.name.split('.').pop() : 'fichier'}
									</span>
								{/if}
							</a>
							<div class="flex items-center gap-1 px-2 py-1.5">
								<span class="min-w-0 flex-1">
									<span class="block truncate text-xs" title={file.name}>{file.name}</span>
									<span class="block text-2xs text-content-subtle tabular-nums"
										>{formatSize(file.size)}</span
									>
								</span>
								<a
									href={url}
									target="_blank"
									rel="noopener"
									class="ws-icon-btn ws-icon-btn-xs"
									aria-label={`Ouvrir ${file.name}`}
									><IconExternal class="h-3.5 w-3.5" aria-hidden="true" /></a
								>
								<button
									type="button"
									class="ws-icon-btn ws-icon-btn-xs text-danger-text"
									aria-label={`Supprimer ${file.name}`}
									data-testid="aux-delete"
									disabled={busy > 0}
									onclick={() => (confirmDelete = file.name)}
									><IconX class="h-3.5 w-3.5" aria-hidden="true" /></button
								>
							</div>
							{#if confirmDelete === file.name}
								<div
									class="absolute inset-0 flex flex-col items-center justify-center gap-2 bg-surface-raised/95 p-2 text-center text-xs"
									role="alertdialog"
									aria-label={`Supprimer ${file.name} ?`}
								>
									<span>Supprimer « {file.name} » ?</span>
									<span class="flex gap-1">
										<button
											type="button"
											class="ws-btn ws-btn-sm ws-btn-danger"
											data-testid="aux-delete-confirm"
											onclick={() => remove(file)}>Supprimer</button
										>
										<button
											type="button"
											class="ws-btn ws-btn-sm ws-btn-secondary"
											onclick={() => (confirmDelete = null)}>Annuler</button
										>
									</span>
								</div>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	</div>
</section>
<!-- eslint-enable svelte/no-navigation-without-resolve -->
