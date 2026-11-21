<script lang="ts">
	// Outil de mesure (T057) : affiche la distance, les composantes et l'angle de
	// la mesure courante. Présentationnel — le gizmo raycaste les points/facettes
	// et fournit les résultats via `measure.ts`.
	import type { PointMeasurement } from './measure';
	import { fmtNum, locale, t } from '$lib/i18n';

	interface Props {
		measurement?: PointMeasurement | null;
		/** Angle mesuré entre deux facettes (degrés), le cas échéant. */
		angle?: number | null;
		onclear?: () => void;
	}

	let { measurement = null, angle = null, onclear }: Props = $props();
</script>

<div class="flex flex-col gap-2 text-sm" data-testid="measure-tool">
	{#if measurement}
		<dl class="grid grid-cols-2 gap-x-4 gap-y-1">
			<dt class="text-content-muted">{$t('Distance')}</dt>
			<dd data-testid="measure-distance">{fmtNum(measurement.distance, 2, $locale)} mm</dd>
			<dt class="text-content-muted">ΔX / ΔY / ΔZ</dt>
			<dd>{measurement.delta.map((d) => d.toFixed(2)).join(' / ')}</dd>
		</dl>
	{/if}

	{#if angle !== null}
		<div class="flex justify-between">
			<span class="text-content-muted">{$t('Angle')}</span>
			<span>{angle.toFixed(1)}°</span>
		</div>
	{/if}

	{#if !measurement && angle === null}
		<p class="text-content-subtle">{$t('Click two points or two facets to measure.')}</p>
	{/if}

	<button type="button" class="ws-btn ws-btn-secondary self-start" onclick={() => onclear?.()}
		>{$t('Clear')}</button
	>
</div>
