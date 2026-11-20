<script lang="ts">
	// Avertissement « G-code modifié » / « Presets personnalisés » d'un 3MF ouvert
	// comme projet (lot 34, validate_presets d'Orca) : items en cause, case « Ne
	// plus afficher » (no_warn_when_modified_gcodes, enregistrée dans les
	// préférences du compte — le serveur ne renvoie plus l'avertissement). Si le
	// rapport porte des métadonnées, un renvoi vers l'onglet Projet est proposé.
	import Dialog from '$lib/ui/Dialog.svelte';
	import { warningContent } from '$lib/projects/open-3mf';
	import type { ImportReport } from '$lib/api/types';

	interface Props {
		report: ImportReport | null;
		/** Enregistre la case « Ne plus afficher » (préférences du compte). */
		onignore: () => void;
		/** Ouvre l'onglet Projet (métadonnées présentes). */
		onshowmetadata?: () => void;
		onclose: () => void;
	}

	let { report, onignore, onshowmetadata, onclose }: Props = $props();

	let ignore = $state(false);
	$effect(() => {
		if (report) ignore = false;
	});
	const warning = $derived(report?.gcode_warning ?? null);
	const content = $derived(warning ? warningContent(warning) : null);
	// La case « Ne plus afficher » s'applique à la fermeture, quel que soit le
	// chemin (OK, Échap, voile) — comme la boîte d'Orca (show_dsa_button).
	function close() {
		if (ignore) onignore();
		onclose();
	}
</script>

<Dialog
	open={report !== null}
	label="Avertissement du projet ouvert"
	title={content?.title ?? 'Projet ouvert'}
	size="sm"
	data-testid="import-report"
	onclose={close}
>
	{#if warning && content}
		<p class="text-sm text-content">{content.body}</p>
		<ul
			class="my-3 max-h-40 list-inside list-disc overflow-auto rounded bg-surface-sunken p-2 text-sm"
			data-testid="import-report-items"
		>
			{#each warning.items as item (item)}
				<li class="font-mono text-xs">{item}</li>
			{/each}
		</ul>
		<label class="flex items-center gap-2 text-sm">
			<input type="checkbox" bind:checked={ignore} data-testid="import-report-ignore" />
			<span>Ne plus afficher cet avertissement</span>
		</label>
	{:else}
		<p class="text-sm text-content">
			Projet 3MF ouvert.
			{report?.load === 'geometry'
				? ' Géométrie seule importée (sans les réglages du fichier).'
				: ''}
		</p>
	{/if}

	{#if report?.has_metadata}
		<p class="mt-3 text-xs text-content-muted" data-testid="import-report-metadata">
			Ce 3MF contient des métadonnées (titre, auteur, licence…).
			{#if onshowmetadata}
				<button type="button" class="ws-btn ws-btn-sm ws-btn-ghost" onclick={onshowmetadata}>
					Voir l'onglet Projet
				</button>
			{/if}
		</p>
	{/if}

	{#snippet footer()}
		<button
			type="button"
			class="ws-btn ws-btn-primary"
			data-testid="import-report-ok"
			onclick={close}
		>
			OK
		</button>
	{/snippet}
</Dialog>
