<script lang="ts">
	// Audit 40 A2 : preset modifié et non enregistré, signalé visiblement à côté
	// du sélecteur (Orca : suffixe « (modified) » du combo, Preset::suffix_modified,
	// et icône d'enregistrement). Pastille plutôt que suffixe dans la liste : lisible
	// même quand le nom du preset est tronqué, et le nom reste intact pour la recherche.
	import { t } from '$lib/i18n';

	let { testid, onclick }: { testid?: string; onclick?: () => void } = $props();
	const title = $derived(
		$t('Modified values are not saved in the preset. Use “Save as” to keep them as a preset.')
	);
</script>

{#if onclick}
	<button
		type="button"
		class="ws-modified-badge inline-flex shrink-0 items-center gap-1 rounded-full bg-warning-soft px-1.5 py-0.5 text-[11px] leading-none font-medium text-warning-content hover:brightness-95"
		{title}
		data-testid={testid}
		{onclick}
	>
		<span class="size-1.5 rounded-full bg-warning" aria-hidden="true"></span>{$t('(modified)')}
	</button>
{:else}
	<span
		class="ws-modified-badge inline-flex shrink-0 items-center gap-1 rounded-full bg-warning-soft px-1.5 py-0.5 text-[11px] leading-none font-medium text-warning-content"
		{title}
		data-testid={testid}
	>
		<span class="size-1.5 rounded-full bg-warning" aria-hidden="true"></span>{$t('(modified)')}
	</span>
{/if}
