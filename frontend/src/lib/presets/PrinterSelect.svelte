<script lang="ts">
	// Sélecteur d'imprimante (T046) : cascade vendeur → modèle → buse, avec une
	// vignette de couverture (placeholder si absente).
	import type { PresetSummary } from '../api/types';
	import { groupPrinters } from './catalog';

	interface Props {
		/** Presets machine à proposer. */
		presets: PresetSummary[];
		/** Id du preset machine sélectionné (liable). */
		selectedId?: string | null;
		/** URL de couverture par modèle (facultatif). */
		covers?: Record<string, string>;
	}

	let { presets, selectedId = $bindable(null), covers = {} }: Props = $props();

	const tree = $derived(groupPrinters(presets));

	// Sélection courante déduite de l'id (survit au rechargement de la liste).
	const selected = $derived(presets.find((p) => p.id === selectedId) ?? null);

	let vendor = $state('');
	let model = $state('');

	// Aligne vendeur/modèle sur la sélection externe.
	$effect(() => {
		if (!selected) return;
		for (const v of tree) {
			for (const m of v.models) {
				if (m.nozzles.some((n) => n.preset.id === selected.id)) {
					vendor = v.vendor;
					model = m.model;
					return;
				}
			}
		}
	});

	const vendorNode = $derived(tree.find((v) => v.vendor === vendor) ?? null);
	const modelNode = $derived(vendorNode?.models.find((m) => m.model === model) ?? null);

	const FIELD =
		'w-full rounded border border-gray-300 px-2 py-1 text-sm dark:border-gray-600 dark:bg-gray-700 dark:text-gray-100';
</script>

<div class="flex gap-3">
	<div
		class="flex h-20 w-20 shrink-0 items-center justify-center overflow-hidden rounded border border-gray-200 bg-gray-50 dark:border-gray-700 dark:bg-gray-800"
	>
		{#if model && covers[model]}
			<img src={covers[model]} alt={model} class="h-full w-full object-contain" />
		{:else}
			<span class="text-xs text-gray-400">Aperçu</span>
		{/if}
	</div>

	<div class="flex flex-1 flex-col gap-2">
		<select
			bind:value={vendor}
			onchange={() => {
				model = '';
				selectedId = null;
			}}
			aria-label="Vendeur"
			class={FIELD}
		>
			<option value="" disabled>Vendeur…</option>
			{#each tree as v (v.vendor)}
				<option value={v.vendor}>{v.vendor}</option>
			{/each}
		</select>

		<select
			bind:value={model}
			onchange={() => (selectedId = null)}
			disabled={!vendorNode}
			aria-label="Modèle"
			class={FIELD}
		>
			<option value="" disabled>Modèle…</option>
			{#each vendorNode?.models ?? [] as m (m.model)}
				<option value={m.model}>{m.model}</option>
			{/each}
		</select>

		<select bind:value={selectedId} disabled={!modelNode} aria-label="Buse" class={FIELD}>
			<option value={null} disabled>Buse…</option>
			{#each modelNode?.nozzles ?? [] as n (n.preset.id)}
				<option value={n.preset.id}>{n.nozzle} mm</option>
			{/each}
		</select>
	</div>
</div>
