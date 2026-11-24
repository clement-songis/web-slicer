<script lang="ts">
	// Avertissement « G-code modifié » / « Presets personnalisés » d'un 3MF ouvert
	// comme projet (lot 34, validate_presets d'Orca) : items en cause, case « Ne
	// plus afficher » (no_warn_when_modified_gcodes, enregistrée dans les
	// préférences du compte — le serveur ne renvoie plus l'avertissement). Si le
	// rapport porte des métadonnées, un renvoi vers l'onglet Projet est proposé.
	import Dialog from '#lib/ui/Dialog.svelte';
	import { gcodeItemLabel, warningContent } from '#lib/projects/open-3mf';
	import { t } from '#lib/i18n';
	import type { ImportReport } from '#lib/api/types';

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
	label={$t('Opened project warning')}
	title={content?.title ?? $t('Project opened')}
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
				{@const label = warning.kind === 'modified_gcodes' ? gcodeItemLabel(item) : null}
				<li class="text-xs" data-key={item}>
					{#if label}
						<span class="text-content">{$t(label)}</span>
						<span class="font-mono text-content-muted">({item})</span>
					{:else}
						<span class="font-mono">{item}</span>
					{/if}
				</li>
			{/each}
		</ul>
		<label class="flex items-center gap-2 text-sm">
			<input type="checkbox" bind:checked={ignore} data-testid="import-report-ignore" />
			<span>{$t('Don’t show this warning again')}</span>
		</label>
	{:else}
		<p class="text-sm text-content">
			{$t('3MF project opened.')}
			{report?.load === 'geometry'
				? ' ' + $t('Geometry only imported (without the file settings).')
				: ''}
		</p>
	{/if}

	{#if report?.has_metadata}
		<p class="mt-3 text-xs text-content-muted" data-testid="import-report-metadata">
			{$t('This 3MF contains metadata (title, author, license…).')}
			{#if onshowmetadata}
				<button type="button" class="ws-btn ws-btn-sm ws-btn-ghost" onclick={onshowmetadata}>
					{$t('See the Project tab')}
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
