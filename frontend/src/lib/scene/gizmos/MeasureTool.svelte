<script lang="ts">
	// Outil de mesure (T057) : affiche la distance, les composantes et l'angle de
	// la mesure courante. Présentationnel — le gizmo raycaste les points/facettes
	// et fournit les résultats via `measure.ts`.
	import type { PointMeasurement } from './measure';

	interface Props {
		measurement?: PointMeasurement | null;
		/** Angle mesuré entre deux facettes (degrés), le cas échéant. */
		angle?: number | null;
		onclear?: () => void;
	}

	let { measurement = null, angle = null, onclear }: Props = $props();
</script>

<div class="flex flex-col gap-2 text-sm">
	{#if measurement}
		<dl class="grid grid-cols-2 gap-x-4 gap-y-1">
			<dt class="text-content-muted">Distance</dt>
			<dd>{measurement.distance.toFixed(2)} mm</dd>
			<dt class="text-content-muted">ΔX / ΔY / ΔZ</dt>
			<dd>{measurement.delta.map((d) => d.toFixed(2)).join(' / ')}</dd>
		</dl>
	{/if}

	{#if angle !== null}
		<div class="flex justify-between">
			<span class="text-content-muted">Angle</span>
			<span>{angle.toFixed(1)}°</span>
		</div>
	{/if}

	{#if !measurement && angle === null}
		<p class="text-content-subtle">Cliquez deux points ou deux facettes à mesurer.</p>
	{/if}

	<button
		type="button"
		class="self-start rounded border border-border-strong px-3 py-1 hover:bg-overlay"
		onclick={() => onclear?.()}>Effacer</button
	>
</div>
