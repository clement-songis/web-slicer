<script lang="ts">
	// Carte « Résultat de la calibration » d'un projet de calibration : la mesure
	// relevée sur l'impression (hauteur de la tour, K imprimé, palier de
	// température) devient la valeur à appliquer au filament ou à enregistrer
	// dans l'historique PA — sans calcul à la main (Orca : wiki + saisie manuelle).
	import { kAtHeight, type CalibrationReading } from './calibration-result';
	import { parseK } from './pa-history';
	import { t } from '$lib/i18n';

	interface Props {
		reading: CalibrationReading;
		/** Applique K au filament actif (pressure_advance). */
		onapplyk: (k: number) => void;
		/** Ouvre l'historique PA prérempli avec K. */
		onrecordk: (k: number) => void;
		/** Applique la température au filament actif (nozzle_temperature). */
		onapplytemp: (t: number) => void;
	}

	let { reading, onapplyk, onrecordk, onapplytemp }: Props = $props();

	let draft = $state('');
	let tempDraft = $state('');
	const k = $derived.by(() => {
		if (reading.kind === 'pa_tower') return kAtHeight(reading, Number(draft.replace(',', '.')));
		if (reading.kind === 'pa_direct') return parseK(draft);
		return null;
	});
	const temp = $derived(tempDraft === '' ? null : Number(tempDraft));
</script>

<section
	class="mb-3 rounded-lg border border-border bg-overlay p-3 text-sm"
	data-testid="calib-result"
	aria-label={$t('Calibration result')}
>
	<h3 class="mb-1 text-xs font-semibold tracking-wide text-content-muted uppercase">
		{$t('Calibration result')}
	</h3>
	{#if reading.kind === 'temp_tower'}
		<label class="flex flex-col gap-1">
			<span class="text-xs text-content-subtle"
				>{$t('After printing: the cleanest step (from bottom to top, 10 mm each).')}</span
			>
			<select class="ws-field ws-field-sm" bind:value={tempDraft} data-testid="calib-result-temp">
				<option value="">{$t('Choose…')}</option>
				{#each reading.temperatures as temperature, i (i)}
					<option value={String(temperature)}
						>{$t('Step {n} — {temp} °C', { n: i + 1, temp: temperature })}</option
					>
				{/each}
			</select>
		</label>
		<div class="mt-2 flex justify-end">
			<button
				type="button"
				class="ws-btn ws-btn-sm ws-btn-primary"
				disabled={temp === null}
				data-testid="calib-result-apply"
				onclick={() => temp !== null && onapplytemp(temp)}>{$t('Apply to filament')}</button
			>
		</div>
	{:else}
		<label class="flex flex-col gap-1">
			<span class="text-xs text-content-subtle">
				{reading.kind === 'pa_tower'
					? $t('After printing: height (mm) of the cleanest area, from 0 to {max}.', {
							max: Math.round(reading.maxHeight)
						})
					: $t('After printing: K value printed next to the best test ({start} to {end}).', {
							start: reading.start,
							end: reading.end
						})}
			</span>
			<span class="flex items-center gap-2">
				<input
					class="ws-field ws-field-sm w-24 text-right"
					inputmode="decimal"
					bind:value={draft}
					placeholder={reading.kind === 'pa_tower' ? 'mm' : 'K'}
					aria-label={reading.kind === 'pa_tower' ? $t('Measured height (mm)') : $t('Measured K')}
					data-testid="calib-result-input"
				/>
				<span class="text-content-muted" data-testid="calib-result-k"
					>{k === null ? '' : `K = ${k}`}</span
				>
			</span>
		</label>
		<div class="mt-2 flex flex-wrap justify-end gap-2">
			<button
				type="button"
				class="ws-btn ws-btn-sm ws-btn-secondary"
				disabled={k === null}
				data-testid="calib-result-record"
				onclick={() => k !== null && onrecordk(k)}>{$t('Save to history…')}</button
			>
			<button
				type="button"
				class="ws-btn ws-btn-sm ws-btn-primary"
				disabled={k === null}
				data-testid="calib-result-apply"
				onclick={() => k !== null && onapplyk(k)}>{$t('Apply to filament')}</button
			>
		</div>
	{/if}
</section>
