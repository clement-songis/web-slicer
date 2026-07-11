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
		oncut: (plane: CutPlane, connectorSpacing: number) => void;
		oncancel?: () => void;
	}

	let { min = [-100, -100, -100], max = [100, 100, 100], oncut, oncancel }: Props = $props();

	let axis = $state<Axis>('z');
	let offset = $state(0);
	let connectorSpacing = $state(0);

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
				class="rounded border border-border-strong px-2 py-0.5 {axis === a
					? 'bg-primary text-primary-content'
					: 'bg-surface-sunken'}"
				onclick={() => (axis = a)}>{a.toUpperCase()}</button
			>
		{/each}
	</div>

	<label class="flex items-center gap-2">
		<span class="w-20 text-content-muted">Décalage</span>
		<input
			type="range"
			min={bounds.lo}
			max={bounds.hi}
			step={0.5}
			bind:value={offset}
			class="flex-1"
		/>
		<input
			type="number"
			bind:value={offset}
			class="w-20 rounded border border-border-strong bg-surface-raised px-1 text-right"
		/>
	</label>

	<label class="flex items-center gap-2">
		<span class="w-20 text-content-muted">Connecteurs</span>
		<input
			type="number"
			min={0}
			step={1}
			bind:value={connectorSpacing}
			class="w-20 rounded border border-border-strong bg-surface-raised px-1 text-right"
			aria-label="Pas des connecteurs (mm, 0 = aucun)"
		/>
		<span class="text-content-subtle">mm (0 = aucun)</span>
	</label>

	<div class="flex gap-2">
		<button
			type="button"
			class="rounded bg-primary px-3 py-1 text-white hover:bg-primary-hover"
			onclick={() => oncut(plane(), connectorSpacing)}>Couper</button
		>
		<button
			type="button"
			class="rounded border border-border-strong px-3 py-1 hover:bg-overlay"
			onclick={() => oncancel?.()}>Annuler</button
		>
	</div>
</div>
