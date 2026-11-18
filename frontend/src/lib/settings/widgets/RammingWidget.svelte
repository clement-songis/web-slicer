<script lang="ts">
	// filament_ramming_parameters : résumé (durée, volume, largeur) + bouton
	// ouvrant l'éditeur de courbe (RammingDialog, parité Orca « Ramming
	// settings… »). La chaîne exacte est conservée tant que rien ne change.
	import { t } from '$lib/i18n';
	import RammingDialog from '../special/RammingDialog.svelte';
	import { parseRamming, rammingTime, rammingVolume } from '../special/ramming';
	import type { WidgetProps } from './types';

	let {
		def,
		value = $bindable(),
		disabled = false,
		index
	}: WidgetProps<string[] | string> = $props();

	const raw = $derived(Array.isArray(value) ? (value[0] ?? '') : (value ?? ''));
	const summary = $derived.by(() => {
		const r = parseRamming(raw);
		const time = rammingTime(r);
		return {
			time,
			volume: Math.round(rammingVolume(r.points, time)),
			lineWidth: r.lineWidth,
			spacing: r.spacing
		};
	});
	let open = $state(false);

	function apply(next: string) {
		if (Array.isArray(value)) {
			const copy = [...value];
			copy[0] = next;
			value = copy;
		} else value = [next];
		open = false;
	}
	const fmt = (n: number) => n.toLocaleString('fr-FR', { maximumFractionDigits: 2 });
</script>

<div class="flex items-center gap-2" data-index={index}>
	<span
		class="min-w-0 flex-1 truncate text-xs text-content-subtle tabular-nums"
		data-testid="ramming-summary"
	>
		{fmt(summary.time)} s · {summary.volume} mm³ · {summary.lineWidth} %
	</span>
	<button
		type="button"
		class="ws-btn ws-btn-sm ws-btn-secondary shrink-0"
		data-testid="ramming-open"
		aria-label={`${$t(def.label)} : personnaliser`}
		{disabled}
		onclick={() => (open = true)}>Personnaliser…</button
	>
</div>
{#if open}
	<RammingDialog open value={raw} onclose={() => (open = false)} onapply={apply} />
{/if}
