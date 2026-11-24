<script lang="ts">
	// « Ouvrir un projet 3MF » (lot 34) : ProjectDropDialog d'Orca 2.4.1
	// (Plater.cpp) — « Veuillez sélectionner une action », nom du fichier,
	// radio « Ouvrir comme projet » / « Importer la géométrie seule » (choix par
	// défaut : géométrie, comme Orca), OK / Annuler. Plus intuitif qu'Orca : la
	// case « Se souvenir de mon choix » (masquée chez Orca) reste offerte et
	// chaque option explique ce qu'elle charge.
	import Dialog from '#lib/ui/Dialog.svelte';
	import { t } from '#lib/i18n';
	import type { ImportLoadMode } from '#lib/api/types';

	interface Props {
		open: boolean;
		filename: string;
		onconfirm: (mode: ImportLoadMode, remember: boolean) => void;
		oncancel: () => void;
	}

	let { open, filename, onconfirm, oncancel }: Props = $props();

	// Orca : m_action = 2 → « Import geometry only » présélectionné.
	let mode = $state<ImportLoadMode>('geometry');
	let remember = $state(false);
	$effect(() => {
		if (open) {
			mode = 'geometry';
			remember = false;
		}
	});
</script>

<Dialog
	{open}
	label={$t('Open a 3MF project')}
	title={$t('Open a 3MF project')}
	size="sm"
	data-testid="open3mf-dialog"
	onclose={oncancel}
>
	<p class="text-sm text-content">{$t('Choose an action for')}</p>
	<p class="mb-3 truncate text-sm font-medium" data-testid="open3mf-filename">{filename}</p>

	<fieldset class="flex flex-col gap-2">
		<legend class="sr-only">{$t('Action')}</legend>
		<label
			class="flex cursor-pointer items-start gap-2 rounded border border-border p-2 {mode ===
			'project'
				? 'border-accent bg-overlay'
				: ''}"
		>
			<input
				type="radio"
				name="open3mf-mode"
				value="project"
				bind:group={mode}
				data-testid="open3mf-project"
				class="mt-0.5"
			/>
			<span>
				<span class="block text-sm font-medium">{$t('Open as project')}</span>
				<span class="block text-xs text-content-subtle">
					{$t('Objects, plates, settings and presets, attached files and 3MF metadata.')}
				</span>
			</span>
		</label>
		<label
			class="flex cursor-pointer items-start gap-2 rounded border border-border p-2 {mode ===
			'geometry'
				? 'border-accent bg-overlay'
				: ''}"
		>
			<input
				type="radio"
				name="open3mf-mode"
				value="geometry"
				bind:group={mode}
				data-testid="open3mf-geometry"
				class="mt-0.5"
			/>
			<span>
				<span class="block text-sm font-medium">{$t('Import geometry only')}</span>
				<span class="block text-xs text-content-subtle">
					{$t('Objects and their transformations, without settings, presets or metadata.')}
				</span>
			</span>
		</label>
	</fieldset>

	<label class="mt-3 flex items-center gap-2 text-sm">
		<input type="checkbox" bind:checked={remember} data-testid="open3mf-remember" />
		<span>{$t('Remember my choice')}</span>
		<span class="text-xs text-content-subtle">{$t('(can be changed in Preferences)')}</span>
	</label>

	{#snippet footer()}
		<button
			type="button"
			class="ws-btn ws-btn-secondary"
			data-testid="open3mf-cancel"
			onclick={oncancel}
		>
			{$t('Cancel')}
		</button>
		<button
			type="button"
			class="ws-btn ws-btn-primary"
			data-testid="open3mf-ok"
			onclick={() => onconfirm(mode, remember)}
		>
			{mode === 'project' ? $t('Open as project') : $t('Import geometry')}
		</button>
	{/snippet}
</Dialog>
