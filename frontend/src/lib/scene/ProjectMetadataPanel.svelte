<script lang="ts">
	// Onglet « Projet » (lot 34) : métadonnées 3MF d'Orca (onglet Projet de
	// ProjectPanel, éditeur DesignerPanel d'Auxiliary.cpp) — titre, auteur
	// (designer), licence (liste d'Orca : CC0, BY…), description et couverture
	// choisie parmi les fichiers joints « Model Pictures » ; GET/PUT
	// /api/projects/{id}/metadata. Lecture seule : auteur affiché (Copyright),
	// origin et profil de publication. Écrit à l'export 3MF du projet.
	import { ApiError } from '$lib/api/client';
	import { auxiliaryUrl, listAuxiliaries } from '$lib/api/auxiliaries';
	import { getProjectMetadata, putProjectMetadata } from '$lib/api/projects';
	import type { ProjectMetadataDto } from '$lib/api/types';
	import { t, tr } from '$lib/i18n';

	const PICTURES_FOLDER = 'Model Pictures';
	const LICENSES = ['', 'CC0', 'BY', 'BY-SA', 'BY-ND', 'BY-NC', 'BY-NC-SA', 'BY-NC-ND'];
	const LICENSE_LABELS: Record<string, string> = {
		'': '(none)',
		CC0: 'CC0 (public domain)',
		BY: 'CC BY',
		'BY-SA': 'CC BY-SA',
		'BY-ND': 'CC BY-ND',
		'BY-NC': 'CC BY-NC',
		'BY-NC-SA': 'CC BY-NC-SA',
		'BY-NC-ND': 'CC BY-NC-ND'
	};

	let { projectId }: { projectId: string } = $props();

	let meta = $state<ProjectMetadataDto | null>(null);
	let draft = $state<ProjectMetadataDto | null>(null);
	let pictures = $state<string[]>([]);
	let loadError = $state<string | null>(null);
	let saveMsg = $state<{ ok: boolean; text: string } | null>(null);
	let saving = $state(false);

	async function load() {
		loadError = null;
		try {
			const [dto, aux] = await Promise.all([
				getProjectMetadata(projectId),
				listAuxiliaries(projectId).catch(() => [])
			]);
			meta = dto;
			draft = { ...dto };
			pictures = aux
				.filter((e) => e.path.startsWith(PICTURES_FOLDER + '/'))
				.map((e) => e.path.slice(PICTURES_FOLDER.length + 1));
		} catch (e) {
			loadError = e instanceof ApiError ? e.message : tr('metadata unavailable');
		}
	}
	$effect(() => {
		void load();
	});

	const dirty = $derived(
		draft !== null &&
			meta !== null &&
			(draft.title !== meta.title ||
				draft.designer !== meta.designer ||
				draft.license !== meta.license ||
				draft.description !== meta.description ||
				draft.cover !== meta.cover)
	);

	async function save() {
		if (!draft) return;
		saving = true;
		saveMsg = null;
		try {
			const saved = await putProjectMetadata(projectId, draft);
			meta = saved;
			draft = { ...saved };
			saveMsg = { ok: true, text: tr('Metadata saved (written on 3MF export).') };
		} catch (e) {
			saveMsg = { ok: false, text: e instanceof ApiError ? e.message : tr('failed') };
		} finally {
			saving = false;
		}
	}
</script>

<div class="flex flex-col gap-3" data-testid="project-metadata">
	{#if loadError}
		<p class="text-sm text-danger" role="alert">{loadError}</p>
	{:else if draft === null}
		<p class="text-sm text-content-muted">{$t('Loading…')}</p>
	{:else}
		<label class="flex flex-col gap-1 text-sm">
			<span class="text-content-muted">{$t('Title')}</span>
			<input
				class="ws-field ws-field-sm"
				data-testid="project-metadata-title"
				bind:value={draft.title}
			/>
		</label>

		<label class="flex flex-col gap-1 text-sm">
			<span class="text-content-muted">{$t('Author')}</span>
			<input
				class="ws-field ws-field-sm"
				data-testid="project-metadata-designer"
				bind:value={draft.designer}
			/>
			{#if meta?.display_author && meta.display_author !== meta.designer}
				<span class="text-xs text-content-subtle">
					{$t('Displayed author: {author} (file Copyright)', {
						author: meta.display_author
					})}
				</span>
			{/if}
		</label>

		<label class="flex flex-col gap-1 text-sm">
			<span class="text-content-muted">{$t('License')}</span>
			<select
				class="ws-field ws-field-sm"
				data-testid="project-metadata-license"
				bind:value={draft.license}
			>
				{#each LICENSES as lic (lic)}
					<option value={lic}>{$t(LICENSE_LABELS[lic] ?? lic)}</option>
				{/each}
			</select>
		</label>

		<label class="flex flex-col gap-1 text-sm">
			<span class="text-content-muted">{$t('Description')}</span>
			<textarea
				class="ws-field ws-field-sm min-h-24"
				data-testid="project-metadata-description"
				bind:value={draft.description}></textarea>
		</label>

		<label class="flex flex-col gap-1 text-sm">
			<span class="text-content-muted">{$t('Cover (attached “Model Pictures” image)')}</span>
			<select
				class="ws-field ws-field-sm"
				data-testid="project-metadata-cover"
				bind:value={draft.cover}
			>
				<option value="">{$t('(none)')}</option>
				{#each pictures as pic (pic)}
					<option value={pic}>{pic}</option>
				{/each}
			</select>
		</label>
		{#if draft.cover}
			<img
				src={auxiliaryUrl(projectId, PICTURES_FOLDER, draft.cover)}
				alt={$t('Cover: {name}', { name: draft.cover })}
				class="max-h-40 rounded border border-border object-contain"
				data-testid="project-metadata-cover-preview"
			/>
		{/if}

		{#if pictures.length === 0}
			<p class="text-xs text-content-subtle">
				{$t(
					'No attached image: add a file to the “Model Pictures” folder (attachments panel) to choose a cover.'
				)}
			</p>
		{/if}

		{#if meta?.origin}
			<p class="text-xs text-content-subtle">
				{$t('Origin: {origin} (read-only)', { origin: meta.origin })}
			</p>
		{/if}
		{#if meta?.profile?.title}
			<p class="text-xs text-content-subtle">
				{$t('Published profile: {title} (read-only)', { title: meta.profile.title })}
			</p>
		{/if}

		<div class="flex items-center gap-3">
			<button
				type="button"
				class="ws-btn ws-btn-sm ws-btn-primary"
				data-testid="project-metadata-save"
				disabled={!dirty || saving}
				onclick={save}
			>
				{$t('Save')}
			</button>
			{#if saveMsg}
				<span
					class="text-xs {saveMsg.ok ? 'text-success' : 'text-danger'}"
					role={saveMsg.ok ? 'status' : 'alert'}
				>
					{saveMsg.text}
				</span>
			{/if}
		</div>
	{/if}
</div>
