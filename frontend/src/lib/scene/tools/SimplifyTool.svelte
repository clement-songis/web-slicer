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
		<span class="w-28 text-slate-300">Taille de grille</span>
		<input type="range" min={0.1} max={20} step={0.1} bind:value={cell} class="flex-1" />
		<input
			type="number"
			min={0.1}
			step={0.1}
			bind:value={cell}
			class="w-20 rounded border border-slate-600 bg-slate-900 px-1 text-right"
			aria-label="Taille de cellule (mm)"
		/>
		<span class="text-slate-500">mm</span>
	</label>

	<div class="flex gap-2">
		<button
			type="button"
			class="rounded bg-sky-600 px-3 py-1 text-white hover:bg-sky-500"
			onclick={() => onsimplify(cell)}>Simplifier</button
		>
		<button
			type="button"
			class="rounded border border-slate-600 px-3 py-1 hover:bg-slate-700"
			onclick={() => oncancel?.()}>Annuler</button
		>
	</div>
</div>
