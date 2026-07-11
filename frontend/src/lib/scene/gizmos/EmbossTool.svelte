<script lang="ts">
	// Outil texte/SVG en relief (T057, Emboss & SVG). Saisit et valide les
	// paramètres du volume ; la génération du maillage est une opération moteur
	// (FFI, phase P5) — le bouton reste désactivé tant que l'endpoint n'existe pas.
	import { defaultEmbossParams, validateEmboss, type EmbossParams } from './emboss';

	interface Props {
		onsubmit: (params: EmbossParams) => void;
		/** Vrai quand l'endpoint de création de volume moteur est disponible (P5). */
		available?: boolean;
	}

	let { onsubmit, available = false }: Props = $props();

	let params = $state<EmbossParams>(defaultEmbossParams());
	const error = $derived(validateEmboss(params));
</script>

<div class="flex flex-col gap-2 text-sm">
	<div class="flex items-center gap-2">
		<span class="text-content-muted">Source</span>
		<button
			type="button"
			class="rounded border border-border-strong px-2 py-0.5 {params.source === 'text'
				? 'bg-primary text-primary-content'
				: 'bg-surface-sunken'}"
			onclick={() => (params.source = 'text')}>Texte</button
		>
		<button
			type="button"
			class="rounded border border-border-strong px-2 py-0.5 {params.source === 'svg'
				? 'bg-primary text-primary-content'
				: 'bg-surface-sunken'}"
			onclick={() => (params.source = 'svg')}>SVG</button
		>
	</div>

	{#if params.source === 'text'}
		<label class="flex flex-col gap-1">
			<span class="text-content-muted">Texte</span>
			<input
				bind:value={params.text}
				class="rounded border border-border-strong bg-surface-raised px-1 py-0.5"
			/>
		</label>
		<label class="flex items-center justify-between gap-2">
			<span class="text-content-muted">Taille de police (mm)</span>
			<input
				type="number"
				min={0.1}
				step={0.5}
				bind:value={params.fontSize}
				class="w-20 rounded border border-border-strong bg-surface-raised px-1 text-right"
			/>
		</label>
	{:else}
		<label class="flex flex-col gap-1">
			<span class="text-content-muted">SVG</span>
			<textarea
				bind:value={params.svg}
				rows="3"
				class="rounded border border-border-strong bg-surface-raised px-1 py-0.5 font-mono"
			></textarea>
		</label>
	{/if}

	<label class="flex items-center justify-between gap-2">
		<span class="text-content-muted">Profondeur (mm)</span>
		<input
			type="number"
			min={0.1}
			step={0.1}
			bind:value={params.depth}
			class="w-20 rounded border border-border-strong bg-surface-raised px-1 text-right"
		/>
	</label>

	<label class="flex items-center gap-2">
		<input type="checkbox" bind:checked={params.embossed} />
		<span class="text-content-muted">En relief (décoché : en creux)</span>
	</label>

	{#if error}
		<p class="text-warning">{error}</p>
	{/if}

	<button
		type="button"
		class="self-start rounded bg-primary px-3 py-1 text-white hover:bg-primary-hover disabled:opacity-50"
		disabled={!available || error !== null}
		onclick={() => onsubmit(params)}>Créer le volume</button
	>
	{#if !available}
		<p class="text-content-subtle">La création de volume nécessite le moteur (à venir).</p>
	{/if}
</div>
