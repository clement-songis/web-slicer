<script lang="ts" module>
	export type SimplifyToolParam =
		{ mode: 'ratio'; decimateRatio: number } | { mode: 'detail'; level: number };
</script>

<script lang="ts">
	// Outil « Simplifier » (T055) : paramètres de la simplification quadrique
	// serveur d'Orca (`its_quadric_edge_collapse`, `POST /api/models/{id}/simplify`).
	// Deux modes comme Orca : ratio de décimation (% de triangles retirés) ou
	// niveau de détail. Présentationnel : la page valide (`simplifyRequest`),
	// appelle l'API et recharge le maillage de toutes les instances.
	import { SIMPLIFY_DETAIL_LEVELS } from '../repair';

	interface Props {
		onsimplify: (param: SimplifyToolParam) => void;
		oncancel?: () => void;
		/** Simplification en cours (bouton désactivé). */
		busy?: boolean;
		/** Triangles du maillage courant (information). */
		triangles?: number | null;
	}

	let { onsimplify, oncancel, busy = false, triangles = null }: Props = $props();

	let mode = $state<'ratio' | 'detail'>('detail');
	let ratio = $state(50);
	let level = $state(2);

	function submit() {
		onsimplify(mode === 'ratio' ? { mode, decimateRatio: ratio } : { mode, level });
	}
</script>

<div class="flex flex-col gap-2 text-sm" data-testid="simplify-tool">
	{#if triangles !== null}
		<p class="text-content-subtle">Maillage actuel : {triangles} triangles</p>
	{/if}
	<div class="flex gap-3">
		<label class="flex items-center gap-1">
			<input type="radio" bind:group={mode} value="detail" data-testid="simplify-mode-detail" />
			Niveau de détail
		</label>
		<label class="flex items-center gap-1">
			<input type="radio" bind:group={mode} value="ratio" data-testid="simplify-mode-ratio" />
			Ratio de décimation
		</label>
	</div>

	{#if mode === 'detail'}
		<label class="flex items-center gap-2">
			<span class="w-28 text-content-muted">Détail</span>
			<select bind:value={level} class="ws-field ws-field-sm flex-1" data-testid="simplify-level">
				{#each SIMPLIFY_DETAIL_LEVELS as label, i (label)}
					<option value={i}>{label}</option>
				{/each}
			</select>
		</label>
	{:else}
		<label class="flex items-center gap-2">
			<span class="w-28 text-content-muted">Triangles retirés</span>
			<input type="range" min={1} max={99} step={1} bind:value={ratio} class="flex-1" />
			<input
				type="number"
				min={1}
				max={99}
				step={1}
				bind:value={ratio}
				class="ws-field ws-field-sm w-16 text-right"
				aria-label="Ratio de décimation (%)"
				data-testid="simplify-ratio"
			/>
			<span class="text-content-subtle">%</span>
		</label>
	{/if}

	<div class="flex gap-2">
		<button
			type="button"
			class="ws-btn ws-btn-primary"
			disabled={busy}
			data-testid="simplify-apply"
			onclick={submit}>{busy ? 'Simplification…' : 'Simplifier'}</button
		>
		<button type="button" class="ws-btn ws-btn-secondary" onclick={() => oncancel?.()}
			>Annuler</button
		>
	</div>
</div>
