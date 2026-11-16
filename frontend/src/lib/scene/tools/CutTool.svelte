<script lang="ts">
	// Outil de coupe (T055) : plan positionnable par axe + décalage, avec
	// connecteurs optionnels. Présentationnel : émet le plan calculé au parent
	// (qui applique `splitByPlane` et historise).
	import type { CutPlane } from './cut';

	type Axis = 'x' | 'y' | 'z';

	interface Props {
		/** Emprise du modèle (mm) pour borner le curseur de décalage. */
		min?: [number, number, number];
		max?: [number, number, number];
		oncut: (
			plane: CutPlane,
			connectorSpacing: number,
			keep: { upper: boolean; lower: boolean }
		) => void;
		oncancel?: () => void;
		/** Coupe serveur en cours : bouton désactivé. */
		busy?: boolean;
	}

	let {
		min = [-100, -100, -100],
		max = [100, 100, 100],
		oncut,
		oncancel,
		busy = false
	}: Props = $props();

	let axis = $state<Axis>('z');
	let offset = $state(0);
	let connectorSpacing = $state(0);
	// Moitiés conservées (attributs KeepUpper / KeepLower de `Cut` d'Orca).
	let keepUpper = $state(true);
	let keepLower = $state(true);

	const axisIndex: Record<Axis, number> = { x: 0, y: 1, z: 2 };
	const bounds = $derived({ lo: min[axisIndex[axis]], hi: max[axisIndex[axis]] });

	function plane(): CutPlane {
		const normal: [number, number, number] = [0, 0, 0];
		normal[axisIndex[axis]] = 1;
		const point: [number, number, number] = [0, 0, 0];
		point[axisIndex[axis]] = offset;
		return { point, normal };
	}
</script>

<div class="flex flex-col gap-2 text-sm">
	<div class="flex items-center gap-2">
		<span class="text-content-muted">Axe</span>
		{#each ['x', 'y', 'z'] as const as a (a)}
			<button
				type="button"
				class="ws-chip {axis === a ? 'ws-chip-on' : ''}"
				onclick={() => (axis = a)}>{a.toUpperCase()}</button
			>
		{/each}
	</div>

	<label class="flex items-center gap-2">
		<span class="w-20 text-content-muted">Décalage</span>
		<input
			type="range"
			data-testid="cut-offset"
			min={bounds.lo}
			max={bounds.hi}
			step={0.5}
			bind:value={offset}
			class="flex-1"
		/>
		<input
			type="number"
			data-testid="cut-offset-input"
			bind:value={offset}
			class="ws-field ws-field-sm w-20 text-right"
		/>
	</label>

	<label class="flex items-center gap-2">
		<span class="w-20 text-content-muted">Connecteurs</span>
		<input
			type="number"
			min={0}
			step={1}
			bind:value={connectorSpacing}
			class="ws-field ws-field-sm w-20 text-right"
			aria-label="Pas des connecteurs (mm, 0 = aucun)"
		/>
		<span class="text-content-subtle">mm (0 = aucun)</span>
	</label>

	<div class="flex items-center gap-3">
		<label class="flex items-center gap-1">
			<input type="checkbox" data-testid="cut-keep-upper" bind:checked={keepUpper} />
			<span>Garder le haut</span>
		</label>
		<label class="flex items-center gap-1">
			<input type="checkbox" data-testid="cut-keep-lower" bind:checked={keepLower} />
			<span>Garder le bas</span>
		</label>
	</div>

	<div class="flex gap-2">
		<button
			type="button"
			data-testid="cut-apply"
			class="ws-btn ws-btn-primary"
			disabled={busy || (!keepUpper && !keepLower)}
			onclick={() => oncut(plane(), connectorSpacing, { upper: keepUpper, lower: keepLower })}
			>Couper</button
		>
		<button type="button" class="ws-btn ws-btn-secondary" onclick={() => oncancel?.()}
			>Annuler</button
		>
	</div>
</div>
