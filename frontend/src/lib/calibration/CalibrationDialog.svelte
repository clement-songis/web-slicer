<script lang="ts">
	import { dialogFocus } from '$lib/ui/dialog-focus';
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

	const isTemp = $derived(kind === 'temperature');
	const invalid = $derived(
		isTemp
			? validateTemperature({ start: tStart, end: tEnd })
			: validatePressureAdvance({ start: paStart, end: paEnd, step: paStep })
	);
	const summary = $derived(
		invalid
			? ''
			: isTemp
				? `${Math.round((tStart - tEnd) / 5) + 1} blocs, de ${tStart} °C (bas) à ${tEnd} °C (haut).`
				: `Tour de ${pressureAdvanceTowerHeight({ start: paStart, end: paEnd, step: paStep })} mm : PA = ${paStart} + ${paStep} × hauteur (mm).`
	);

	function submit(event: SubmitEvent) {
		event.preventDefault();
		if (invalid || busy) return;
		onconfirm(isTemp ? { start: tStart, end: tEnd } : { start: paStart, end: paEnd, step: paStep });
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
					{isTemp ? 'Calibration de la température' : 'Calibration du pressure advance'}
				</h2>
				<p class="text-sm text-content-muted">
					{#if isTemp}
						Crée un projet « tour de température » (modèle OrcaSlicer) : un bloc de 10 mm par palier
						de 5 °C, la température change à chaque bloc.
					{:else}
						Crée un projet « tour de pressure advance » (modèle OrcaSlicer) : le PA augmente d'un
						pas par millimètre de hauteur.
					{/if}
				</p>
				{#if isTemp}
					<div class="grid grid-cols-2 gap-3">
						<label class="flex flex-col gap-1 text-sm">
							<span class="text-content-muted">Température de départ (°C)</span>
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
							<span class="text-content-muted">Température de fin (°C)</span>
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
					<div class="grid grid-cols-3 gap-3">
						<label class="flex flex-col gap-1 text-sm">
							<span class="text-content-muted">PA de départ</span>
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
							<span class="text-content-muted">PA de fin</span>
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
							<span class="text-content-muted">Pas</span>
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
						onclick={onclose}>Annuler</button
					>
					<button
						type="submit"
						data-testid="calib-confirm"
						disabled={!!invalid || busy}
						class="ws-btn ws-btn-lg ws-btn-primary"
					>
						{busy ? 'Création…' : 'Créer le projet'}
					</button>
				</div>
			</form>
		</div>
	</div>
{/if}
