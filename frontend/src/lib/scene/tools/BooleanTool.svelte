<script lang="ts" module>
	export type BooleanOp = 'union' | 'difference' | 'intersection';
</script>

<script lang="ts">
	// Outil d'opérations booléennes (T055) : union / différence / intersection
	// entre deux objets sélectionnés (A = premier sélectionné, B = second). Le
	// calcul est fait par mcut (`MeshBoolean::mcut::make_boolean`) dans le worker
	// moteur via `POST /api/projects/{id}/boolean` ; le composant choisit
	// l'opération et l'émet.
	import { t } from '$lib/i18n';

	interface Props {
		onapply: (op: BooleanOp) => void;
		/** Vrai quand l'opération est applicable (deux objets prêts sélectionnés). */
		available?: boolean;
		/** Raison affichée quand l'opération n'est pas applicable. */
		hint?: string;
		busy?: boolean;
	}

	let { onapply, available = false, hint, busy = false }: Props = $props();

	let op = $state<BooleanOp>('union');
	const ops: BooleanOp[] = ['union', 'difference', 'intersection'];
	const labels: Record<BooleanOp, string> = {
		union: 'Union',
		difference: 'Difference',
		intersection: 'Intersection'
	};
</script>

<div class="flex flex-col gap-2 text-sm">
	<label class="flex items-center gap-2">
		<span class="text-content-muted">{$t('Operation')}</span>
		<select data-testid="boolean-op" bind:value={op} class="ws-field ws-field-sm">
			{#each ops as key (key)}
				<option value={key}>{$t(labels[key])}</option>
			{/each}
		</select>
	</label>

	<button
		type="button"
		class="ws-btn ws-btn-primary self-start"
		data-testid="boolean-apply"
		disabled={!available || busy}
		onclick={() => onapply(op)}
	>
		{$t('Apply')}
	</button>

	{#if !available}
		<p class="text-content-subtle">{hint ?? $t('Select two objects (A then B).')}</p>
	{/if}
</div>
