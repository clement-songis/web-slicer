<script lang="ts">
	// Carte « Résultat de la calibration » d'un projet de calibration : la mesure
	// relevée sur l'impression (hauteur de la tour, K imprimé, palier de
	// température) devient la valeur à appliquer au filament ou à enregistrer
	// dans l'historique PA — sans calcul à la main (Orca : wiki + saisie manuelle).
	import { kAtHeight, type CalibrationReading } from './calibration-result';
	import { parseK } from './pa-history';

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
	aria-label="Résultat de la calibration"
>
	<h3 class="mb-1 text-xs font-semibold tracking-wide text-content-muted uppercase">
		Résultat de la calibration
	</h3>
	{#if reading.kind === 'temp_tower'}
		<label class="flex flex-col gap-1">
			<span class="text-xs text-content-subtle"
				>Après impression : palier le plus net (du bas vers le haut, 10 mm chacun).</span
			>
			<select class="ws-field ws-field-sm" bind:value={tempDraft} data-testid="calib-result-temp">
				<option value="">Choisir…</option>
				{#each reading.temperatures as t, i (i)}
					<option value={String(t)}>Palier {i + 1} — {t} °C</option>
				{/each}
			</select>
		</label>
		<div class="mt-2 flex justify-end">
			<button
				type="button"
				class="ws-btn ws-btn-sm ws-btn-primary"
				disabled={temp === null}
				data-testid="calib-result-apply"
				onclick={() => temp !== null && onapplytemp(temp)}>Appliquer au filament</button
			>
		</div>
	{:else}
		<label class="flex flex-col gap-1">
			<span class="text-xs text-content-subtle">
				{reading.kind === 'pa_tower'
					? `Après impression : hauteur (mm) de la zone la plus nette, de 0 à ${Math.round(reading.maxHeight)}.`
					: `Après impression : valeur K imprimée à côté du meilleur essai (${reading.start} à ${reading.end}).`}
			</span>
			<span class="flex items-center gap-2">
				<input
					class="ws-field ws-field-sm w-24 text-right"
					inputmode="decimal"
					bind:value={draft}
					placeholder={reading.kind === 'pa_tower' ? 'mm' : 'K'}
					aria-label={reading.kind === 'pa_tower' ? 'Hauteur mesurée (mm)' : 'K relevé'}
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
				onclick={() => k !== null && onrecordk(k)}>Enregistrer dans l'historique…</button
			>
			<button
				type="button"
				class="ws-btn ws-btn-sm ws-btn-primary"
				disabled={k === null}
				data-testid="calib-result-apply"
				onclick={() => k !== null && onapplyk(k)}>Appliquer au filament</button
			>
		</div>
	{/if}
</section>
