<script lang="ts">
	// Vue d'assemblage (T058, `assembly_view` + gizmo Assembly) : bascule
	// l'éclaté des pièces et règle le facteur d'écartement. Présentationnel —
	// le parent applique `explode` aux positions de rendu (sans altérer les
	// transformations enregistrées).
	interface Props {
		exploded?: boolean;
		factor?: number;
		/** Mode Orca `GLGizmoAssembly` : vue éclatée, face à face, point à point. */
		mode?: 'explode' | 'face' | 'point';
		/** Nombre d'entités déjà choisies (0 ou 1) en mode face/point. */
		picks?: number;
		onchange?: (exploded: boolean, factor: number) => void;
	}

	let {
		exploded = $bindable(false),
		factor = $bindable(0.5),
		mode = $bindable('explode'),
		picks = 0,
		onchange
	}: Props = $props();

	function emit() {
		onchange?.(exploded, factor);
	}
</script>

<div class="flex flex-col gap-2 text-sm">
	<label class="flex items-center gap-2">
		<span class="text-content-muted">Mode</span>
		<select bind:value={mode} data-testid="assembly-mode" class="ws-field ws-field-sm">
			<option value="explode">Vue éclatée</option>
			<option value="face">Assemblage face à face</option>
			<option value="point">Assemblage point à point</option>
		</select>
	</label>
	{#if mode !== 'explode'}
		<p class="text-content-subtle" data-testid="assembly-hint" data-picks={picks}>
			{picks === 0
				? `Cliquez ${mode === 'face' ? 'une face' : 'un point'} de l’objet fixe.`
				: `Cliquez ${mode === 'face' ? 'une face' : 'un point'} de l’objet à déplacer.`}
		</p>
	{/if}
	<label class="flex items-center gap-2">
		<input type="checkbox" bind:checked={exploded} onchange={emit} />
		<span class="text-content-muted">Vue éclatée</span>
	</label>

	{#if exploded}
		<label class="flex items-center gap-2">
			<span class="w-20 text-content-muted">Écartement</span>
			<input
				type="range"
				min={0}
				max={3}
				step={0.1}
				bind:value={factor}
				oninput={emit}
				class="flex-1"
			/>
			<span class="w-8 text-right text-content-muted">{factor.toFixed(1)}</span>
		</label>
	{/if}
</div>
