<script lang="ts">
	import { dialogFocus } from '#lib/ui/dialog-focus';
	// Dialogue de paramètres d'une calibration (Temp_Calibration_Dlg /
	// PA_Calibration_Dlg d'Orca) : plage, validation, puis création du projet
	// par l'appelant.
	import {
		DEFAULTS,
		pressureAdvanceTowerHeight,
		validatePressureAdvance,
		validateTemperature,
		type CalibrationKind,
		type PressureAdvanceParams,
		type TemperatureParams
	} from './calibration';
	import { fmtDec, locale, t } from '#lib/i18n';

	interface Props {
		open: boolean;
		kind: CalibrationKind;
		busy?: boolean;
		error?: string | null;
		onclose: () => void;
		onconfirm: (params: TemperatureParams | PressureAdvanceParams) => void;
	}

	let { open, kind, busy = false, error = null, onclose, onconfirm }: Props = $props();

	let tStart = $state(DEFAULTS.temperature.start);
	let tEnd = $state(DEFAULTS.temperature.end);
	let paStart = $state(DEFAULTS.pressure_advance.start);
	let paEnd = $state(DEFAULTS.pressure_advance.end);
	let paStep = $state(DEFAULTS.pressure_advance.step);
	let paMethod = $state<'tower' | 'line'>('tower');

	const isTemp = $derived(kind === 'temperature');
	const invalid = $derived.by(() => {
		void $t; // message traduit : recalcul au changement de langue
		return isTemp
			? validateTemperature({ start: tStart, end: tEnd })
			: validatePressureAdvance({ start: paStart, end: paEnd, step: paStep });
	});
	const summary = $derived(
		invalid
			? ''
			: isTemp
				? $t('{n} blocks, from {start} °C (bottom) to {end} °C (top).', {
						n: Math.round((tStart - tEnd) / 5) + 1,
						start: tStart,
						end: tEnd
					})
				: $t('{height} mm tower: PA = {start} + {step} × height (mm).', {
						height: pressureAdvanceTowerHeight({ start: paStart, end: paEnd, step: paStep }),
						start: fmtDec(paStart, 4, $locale),
						step: fmtDec(paStep, 4, $locale)
					})
	);

	function submit(event: SubmitEvent) {
		event.preventDefault();
		if (invalid || busy) return;
		onconfirm(
			isTemp
				? { start: tStart, end: tEnd }
				: { start: paStart, end: paEnd, step: paStep, method: paMethod }
		);
	}

	const FIELD = 'h-9 rounded-md border border-border-strong bg-surface-raised px-2';
</script>

<svelte:window onkeydown={(e) => open && e.key === 'Escape' && !busy && onclose()} />

{#if open}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-scrim p-4"
		role="presentation"
		onclick={() => !busy && onclose()}
	>
		<div
			class="ws-dialog w-full max-w-sm p-5"
			role="dialog"
			aria-modal="true"
			aria-labelledby="calib-title"
			data-testid="calib-dialog"
			tabindex="-1"
			use:dialogFocus
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.key !== 'Escape' && e.stopPropagation()}
		>
			<form class="flex flex-col gap-4" onsubmit={submit}>
				<h2 id="calib-title" class="ws-dialog-title">
					{isTemp ? $t('Temperature calibration') : $t('Pressure advance calibration')}
				</h2>
				<p class="text-sm text-content-muted">
					{#if isTemp}
						{$t(
							'Creates a “temperature tower” project (OrcaSlicer model): one 10 mm block per 5 °C step, the temperature changes at each block.'
						)}
					{:else}
						{$t(
							'Creates a “pressure advance tower” project (OrcaSlicer model): the PA increases by one step per millimeter of height.'
						)}
					{/if}
				</p>
				{#if isTemp}
					<div class="grid grid-cols-2 gap-3">
						<label class="flex flex-col gap-1 text-sm">
							<span class="text-content-muted">{$t('Start temperature (°C)')}</span>
							<input
								type="number"
								step="5"
								min="170"
								max="350"
								bind:value={tStart}
								data-testid="calib-start"
								class={FIELD}
								disabled={busy}
							/>
						</label>
						<label class="flex flex-col gap-1 text-sm">
							<span class="text-content-muted">{$t('End temperature (°C)')}</span>
							<input
								type="number"
								step="5"
								min="170"
								max="350"
								bind:value={tEnd}
								data-testid="calib-end"
								class={FIELD}
								disabled={busy}
							/>
						</label>
					</div>
				{:else}
					<!-- Méthodes d'Orca construites par le serveur : tour, lignes (le
					     motif reste dans « Pressure advance pattern »). -->
					<div class="ws-seg self-start" role="group" aria-label={$t('Method')}>
						{#each [['tower', 'Tower'], ['line', 'Lines']] as const as [m, label] (m)}
							<button
								type="button"
								class="ws-seg-item {paMethod === m ? 'ws-seg-on' : ''}"
								aria-pressed={paMethod === m}
								data-testid={`calib-pa-method-${m}`}
								disabled={busy}
								onclick={() => (paMethod = m)}
							>
								{$t(label)}
							</button>
						{/each}
					</div>
					<div class="grid grid-cols-3 gap-3">
						<label class="flex flex-col gap-1 text-sm">
							<span class="text-content-muted">{$t('Start PA')}</span>
							<input
								type="number"
								step="0.001"
								min="0"
								bind:value={paStart}
								data-testid="calib-start"
								class={FIELD}
								disabled={busy}
							/>
						</label>
						<label class="flex flex-col gap-1 text-sm">
							<span class="text-content-muted">{$t('End PA')}</span>
							<input
								type="number"
								step="0.001"
								min="0"
								bind:value={paEnd}
								data-testid="calib-end"
								class={FIELD}
								disabled={busy}
							/>
						</label>
						<label class="flex flex-col gap-1 text-sm">
							<span class="text-content-muted">{$t('Step||range')}</span>
							<input
								type="number"
								step="0.001"
								min="0.001"
								bind:value={paStep}
								data-testid="calib-step"
								class={FIELD}
								disabled={busy}
							/>
						</label>
					</div>
				{/if}
				{#if invalid}
					<p class="text-sm text-danger" role="alert">{invalid}</p>
				{:else}
					<p class="text-xs text-content-subtle">{summary}</p>
				{/if}
				{#if error}
					<p
						class="rounded-md bg-danger-soft px-3 py-2 text-sm text-danger-content"
						role="alert"
						data-testid="calib-error"
					>
						{error}
					</p>
				{/if}
				<div class="flex justify-end gap-2">
					<button
						type="button"
						class="ws-btn ws-btn-lg ws-btn-secondary"
						disabled={busy}
						onclick={onclose}
					>
						{$t('Cancel')}
					</button>
					<button
						type="submit"
						data-testid="calib-confirm"
						disabled={!!invalid || busy}
						class="ws-btn ws-btn-lg ws-btn-primary"
					>
						{busy ? $t('Creating…') : $t('Create project')}
					</button>
				</div>
			</form>
		</div>
	</div>
{/if}
