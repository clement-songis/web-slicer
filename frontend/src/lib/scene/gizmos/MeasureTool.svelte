<script lang="ts">
	// Outil de mesure (T057) : affiche la distance, les composantes et l'angle de
	// la mesure courante. Présentationnel — le gizmo raycaste les points/facettes
	// et fournit les résultats via `measure.ts`.
	// Audit 41 T2 : mode « Face » (Orca GLGizmoMeasure : plans → distance entre
	// faces parallèles, sinon angle) en plus du point à point ; Suppr recommence.
	import type { FeatureMeasurement, PointMeasurement } from './measure';
	import { fmtNum, locale, t } from '$lib/i18n';

	export type MeasureMode = 'point' | 'face';

	interface Props {
		measurement?: PointMeasurement | null;
		/** Angle mesuré entre deux facettes (degrés), le cas échéant. */
		angle?: number | null;
		/** Mesure de faces (mode Face) : distance entre plans parallèles ou angle. */
		feature?: FeatureMeasurement | null;
		mode?: MeasureMode;
		onmode?: (mode: MeasureMode) => void;
		/** Facteur d'affichage des longueurs (pouces : 1/25,4) et unité. */
		lengthFactor?: number;
		lengthUnit?: string;
		onclear?: () => void;
	}

	let {
		measurement = null,
		angle = null,
		feature = null,
		mode = 'point',
		onmode,
		lengthFactor = 1,
		lengthUnit = 'mm',
		onclear
	}: Props = $props();
	const digits = $derived(lengthFactor === 1 ? 2 : 3);
	const len = (v: number) => fmtNum(v * lengthFactor, digits, $locale);
	const shownAngle = $derived(feature?.angle ?? angle);
</script>

<div class="flex flex-col gap-2 text-sm" data-testid="measure-tool">
	{#if onmode}
		<div class="ws-seg self-start" role="radiogroup" aria-label={$t('Measure')}>
			{#each [['point', $t('Point to point')], ['face', $t('Face to face')]] as [value, label] (value)}
				<button
					type="button"
					role="radio"
					class="ws-seg-item {mode === value ? 'ws-seg-on' : ''}"
					aria-checked={mode === value}
					data-testid={`measure-mode-${value}`}
					onclick={() => onmode?.(value as MeasureMode)}>{label}</button
				>
			{/each}
		</div>
	{/if}

	{#if measurement && !feature}
		<dl class="grid grid-cols-2 gap-x-4 gap-y-1">
			<dt class="text-content-muted">{$t('Distance')}</dt>
			<dd data-testid="measure-distance">{len(measurement.distance)} {lengthUnit}</dd>
			<dt class="text-content-muted">ΔX / ΔY / ΔZ</dt>
			<dd data-testid="measure-delta">{measurement.delta.map((d) => len(d)).join(' / ')}</dd>
		</dl>
	{/if}

	{#if feature?.distance != null}
		<dl class="grid grid-cols-2 gap-x-4 gap-y-1">
			<dt class="text-content-muted">{$t('Distance')}</dt>
			<dd data-testid="measure-distance">{len(feature.distance)} {lengthUnit}</dd>
		</dl>
	{/if}

	{#if shownAngle !== null && shownAngle !== undefined}
		<div class="flex justify-between">
			<span class="text-content-muted">{$t('Angle')}</span>
			<span data-testid="measure-angle">{fmtNum(shownAngle, 1, $locale)}°</span>
		</div>
	{/if}

	{#if !measurement && !feature && (angle === null || angle === undefined)}
		<p class="text-content-subtle">
			{mode === 'face'
				? $t('Click two faces: distance if parallel, otherwise angle.')
				: $t('Click two points or two facets to measure.')}
		</p>
	{/if}
	<p class="text-2xs text-content-subtle">{$t('Delete: restart selection')}</p>

	<button type="button" class="ws-btn ws-btn-secondary self-start" onclick={() => onclear?.()}
		>{$t('Clear')}</button
	>
</div>
