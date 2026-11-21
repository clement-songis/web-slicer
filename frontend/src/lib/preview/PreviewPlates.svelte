<script lang="ts">
	// Plateaux dans l'aperçu (phase 31) : Orca liste les plateaux à gauche de
	// l'aperçu (vignettes PartPlateList) et affiche « Statistiques de tous les
	// plateaux » après « Trancher tout » (GCodeViewer::render_all_plates_stats).
	// Ici : une puce par plateau (✓ tranché, clic = aperçu de ce plateau) et les
	// totaux temps / filament / coût des plateaux tranchés.
	import { formatMoney } from '$lib/settings/preferences';
	import { locale, numberLocale, t } from '$lib/i18n';
	import { formatDuration } from './stats';
	import type { PlatesTotals } from './plates-summary';
	import type { PlatesStatsResponse } from './engine-data';
	import FilamentTable from './FilamentTable.svelte';

	interface Props {
		plates: { id: string; name: string; objects: number }[];
		activeId: string | null;
		sliced: readonly string[];
		totals: PlatesTotals | null;
		/** Statistiques serveur de tous les plateaux (POST /gcodes/plates-stats, lot 31). */
		all?: PlatesStatsResponse['all'] | null;
		currency?: string;
		onselect: (id: string) => void;
		/** « Trancher tout » proposé tant qu'un plateau non vide n'est pas tranché. */
		onsliceall?: () => void;
	}
	let {
		plates,
		activeId,
		sliced,
		totals,
		all = null,
		currency = '€',
		onselect,
		onsliceall
	}: Props = $props();
	let detail = $state(false);
	const pending = $derived(plates.filter((p) => p.objects > 0 && !sliced.includes(p.id)).length);
</script>

<section
	class="mb-4 flex flex-col gap-2 border-b border-border pb-4"
	data-testid="preview-plates"
	aria-label={$t('Plates')}
>
	<h2 class="ws-section-title">{$t('Plates')}</h2>
	<div class="flex flex-wrap gap-1.5" role="group" aria-label={$t('Plate preview')}>
		{#each plates as plate (plate.id)}
			{@const done = sliced.includes(plate.id)}
			<button
				type="button"
				class="ws-chip h-7 gap-1.5 {plate.id === activeId ? 'ws-chip-on' : ''}"
				aria-pressed={plate.id === activeId}
				data-testid="preview-plate"
				data-plate-id={plate.id}
				data-sliced={done ? 'true' : 'false'}
				data-tip={done
					? $t('Sliced: show its preview')
					: plate.objects
						? $t('Not sliced yet')
						: $t('Empty plate')}
				data-tip-side="bottom"
				onclick={() => onselect(plate.id)}
			>
				{plate.name}
				{#if done}<span class="text-success-text" aria-label={$t('sliced')}>✓</span>{/if}
			</button>
		{/each}
	</div>
	{#if all && all.plate_count > 1}
		<!-- render_all_plates_stats : temps sommés, filament cumulé par filament
		     (modèle / support / purge / tour), coût figé des tranchages. -->
		<div class="flex flex-col gap-1" data-testid="preview-plates-all">
			<p class="text-xs text-content-muted tabular-nums" data-testid="preview-plates-totals">
				<span class="text-content">{$t('All plates ({n})', { n: all.plate_count })}</span> ·
				{formatDuration(all.time[0])} ·
				{all.total.total.g.toLocaleString(numberLocale($locale), { maximumFractionDigits: 2 })} g{#if all.total_cost > 0}
					· {formatMoney(all.total_cost, currency)}{/if}
			</p>
			<button
				type="button"
				class="ws-link self-start text-xs"
				aria-expanded={detail}
				data-testid="preview-plates-detail"
				onclick={() => (detail = !detail)}
				>{detail ? $t('Hide the per-filament detail') : $t('Per-filament detail')}</button
			>
			{#if detail}
				<FilamentTable table={all} testid="preview-plates-filaments" />
			{/if}
		</div>
	{:else if totals && totals.plates > 1}
		<p class="text-xs text-content-muted tabular-nums" data-testid="preview-plates-totals">
			<span class="text-content">{$t('All plates ({n})', { n: totals.plates })}</span> ·
			{formatDuration(totals.timeSeconds)} ·
			{totals.grams.toLocaleString(numberLocale($locale), { maximumFractionDigits: 2 })} g{#if totals.cost > 0}
				· {formatMoney(totals.cost, currency)}{/if}
		</p>
	{/if}
	{#if pending > 0 && onsliceall}
		<button
			type="button"
			class="ws-link self-start text-xs"
			data-testid="preview-plates-slice-all"
			onclick={onsliceall}
			>{$t(pending > 1 ? 'Slice all ({n} plates left)' : 'Slice all ({n} plate left)', {
				n: pending
			})}</button
		>
	{/if}
</section>
