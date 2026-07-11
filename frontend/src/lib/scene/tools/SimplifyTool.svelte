<script lang="ts">
	// Outil de simplification (T055) : décimation par regroupement de sommets.
	// Présentationnel : émet la taille de cellule au parent (qui applique
	// `simplifyGrid` et historise).
	interface Props {
		onsimplify: (cell: number) => void;
		oncancel?: () => void;
	}

	let { onsimplify, oncancel }: Props = $props();

	let cell = $state(1);
</script>

<div class="flex flex-col gap-2 text-sm">
	<label class="flex items-center gap-2">
		<span class="w-28 text-content-muted">Taille de grille</span>
		<input type="range" min={0.1} max={20} step={0.1} bind:value={cell} class="flex-1" />
		<input
			type="number"
			min={0.1}
			step={0.1}
			bind:value={cell}
			class="w-20 rounded border border-border-strong bg-surface-raised px-1 text-right"
			aria-label="Taille de cellule (mm)"
		/>
		<span class="text-content-subtle">mm</span>
	</label>

	<div class="flex gap-2">
		<button
			type="button"
			class="rounded bg-primary px-3 py-1 text-white hover:bg-primary-hover"
			onclick={() => onsimplify(cell)}>Simplifier</button
		>
		<button
			type="button"
			class="rounded border border-border-strong px-3 py-1 hover:bg-overlay"
			onclick={() => oncancel?.()}>Annuler</button
		>
	</div>
</div>
