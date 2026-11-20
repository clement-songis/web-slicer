<script lang="ts">
	// « Ouvrir un projet 3MF » (lot 34) : ProjectDropDialog d'Orca 2.4.1
	// (Plater.cpp) — « Veuillez sélectionner une action », nom du fichier,
	// radio « Ouvrir comme projet » / « Importer la géométrie seule » (choix par
	// défaut : géométrie, comme Orca), OK / Annuler. Plus intuitif qu'Orca : la
	// case « Se souvenir de mon choix » (masquée chez Orca) reste offerte et
	// chaque option explique ce qu'elle charge.
	import Dialog from '$lib/ui/Dialog.svelte';
	import type { ImportLoadMode } from '$lib/api/types';

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
	label="Ouvrir un projet 3MF"
	title="Ouvrir un projet 3MF"
	size="sm"
	data-testid="open3mf-dialog"
	onclose={oncancel}
>
	<p class="text-sm text-content">Choisissez une action pour</p>
	<p class="mb-3 truncate text-sm font-medium" data-testid="open3mf-filename">{filename}</p>

	<fieldset class="flex flex-col gap-2">
		<legend class="sr-only">Action</legend>
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
				<span class="block text-sm font-medium">Ouvrir comme projet</span>
				<span class="block text-xs text-content-subtle"
					>Objets, plateaux, réglages et presets, fichiers joints et métadonnées du 3MF.</span
				>
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
				<span class="block text-sm font-medium">Importer la géométrie seule</span>
				<span class="block text-xs text-content-subtle"
					>Les objets et leurs transformations, sans réglages, presets ni métadonnées.</span
				>
			</span>
		</label>
	</fieldset>

	<label class="mt-3 flex items-center gap-2 text-sm">
		<input type="checkbox" bind:checked={remember} data-testid="open3mf-remember" />
		<span>Se souvenir de mon choix</span>
		<span class="text-xs text-content-subtle">(modifiable dans les Préférences)</span>
	</label>

	{#snippet footer()}
		<button
			type="button"
			class="ws-btn ws-btn-secondary"
			data-testid="open3mf-cancel"
			onclick={oncancel}
		>
			Annuler
		</button>
		<button
			type="button"
			class="ws-btn ws-btn-primary"
			data-testid="open3mf-ok"
			onclick={() => onconfirm(mode, remember)}
		>
			{mode === 'project' ? 'Ouvrir comme projet' : 'Importer la géométrie'}
		</button>
	{/snippet}
</Dialog>
