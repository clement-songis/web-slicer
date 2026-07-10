<script lang="ts">
	// Outil d'opérations booléennes (T055) : union / différence / intersection
	// sur la sélection. Le calcul CSG fidèle est délégué au moteur libslic3r via
	// le worker FFI (non câblé — phase P5) : le composant choisit l'opération et
	// l'émet, mais reste désactivé tant que l'endpoint moteur n'est pas exposé.
	export type BooleanOp = 'union' | 'difference' | 'intersection';

	interface Props {
		onapply: (op: BooleanOp) => void;
		/** Vrai quand l'endpoint booléen moteur est disponible (phase P5). */
		available?: boolean;
	}

	let { onapply, available = false }: Props = $props();

	let op = $state<BooleanOp>('union');
	const ops: BooleanOp[] = ['union', 'difference', 'intersection'];
	const labels: Record<BooleanOp, string> = {
		union: 'Union',
		difference: 'Différence',
		intersection: 'Intersection'
	};
</script>

<div class="flex flex-col gap-2 text-sm">
	<label class="flex items-center gap-2">
		<span class="text-slate-300">Opération</span>
		<select bind:value={op} class="rounded border border-slate-600 bg-slate-900 px-1 py-0.5">
			{#each ops as key (key)}
				<option value={key}>{labels[key]}</option>
			{/each}
		</select>
	</label>

	<button
		type="button"
		class="self-start rounded bg-sky-600 px-3 py-1 text-white hover:bg-sky-500 disabled:opacity-50"
		disabled={!available}
		onclick={() => onapply(op)}>Appliquer</button
	>

	{#if !available}
		<p class="text-slate-500">Nécessite le moteur (à venir).</p>
	{/if}
</div>
