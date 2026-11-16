<script lang="ts">
	import { dialogFocus } from '$lib/ui/dialog-focus';
	// Dialogue des calibrations « mode moteur » (Retraction_Test_Dlg,
	// VFA_Test_Dlg, MaxVolumetricSpeed_Test_Dlg, PA_Calibration_Dlg (motif),
	// Input_Shaping_Freq/Damp_Test_Dlg, Cornering_Test_Dlg d'Orca) : début, fin,
	// pas (sauf interpolation continue) et valeur fixe de l'input shaping.
	import {
		ENGINE_CALIB_DEFAULTS,
		ENGINE_CALIB_LABELS,
		ENGINE_CALIB_UNITS,
		FIXED_VALUE_KINDS,
		FIXED_VALUE_LABELS,
		STEPLESS_KINDS,
		validateRange,
		type EngineCalibKind,
		type RangeParams
	} from './engine-calib';

	interface Props {
		open: boolean;
		kind: EngineCalibKind;
		busy?: boolean;
		error?: string | null;
		onclose: () => void;
		onconfirm: (params: RangeParams) => void;
	}

	let { open, kind, busy = false, error = null, onclose, onconfirm }: Props = $props();

	let start = $state(0);
	let end = $state(0);
	let step = $state(0);
	let fixed = $state(0);
	// Valeurs par défaut d'Orca à chaque ouverture / changement de calibration.
	$effect(() => {
		if (open) {
			const defaults = ENGINE_CALIB_DEFAULTS[kind];
			({ start, end, step } = defaults);
			fixed = defaults.fixed ?? 0;
		}
	});

	const stepless = $derived(STEPLESS_KINDS.includes(kind));
	const hasFixed = $derived(FIXED_VALUE_KINDS.includes(kind));
	const fields = $derived(
		stepless
			? [
					['Début', 'start'],
					['Fin', 'end']
				]
			: [
					['Début', 'start'],
					['Fin', 'end'],
					['Pas', 'step']
				]
	);
	const params = $derived({
		start,
		end,
		step: stepless ? 1 : step,
		...(hasFixed ? { fixed } : {})
	});
	const invalid = $derived(validateRange(params, kind));

	function submit(event: SubmitEvent) {
		event.preventDefault();
		if (!invalid && !busy) onconfirm(params);
	}

	const FIELD = 'h-9 w-24 rounded-md border border-border-strong bg-surface-raised px-2';
</script>

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
			aria-label="Calibration {ENGINE_CALIB_LABELS[kind]}"
			data-testid="engine-calib-dialog"
			tabindex="-1"
			use:dialogFocus
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.key === 'Escape' && !busy && onclose()}
		>
			<form class="flex flex-col gap-3" onsubmit={submit}>
				<h2 class="ws-dialog-title">Calibration : {ENGINE_CALIB_LABELS[kind]}</h2>
				{#each fields as [label, key] (key)}
					<label class="flex items-center justify-between gap-2 text-sm">
						<span>{label}{ENGINE_CALIB_UNITS[kind] ? ` (${ENGINE_CALIB_UNITS[kind]})` : ''}</span>
						{#if key === 'start'}
							<input
								class={FIELD}
								type="number"
								step="any"
								data-testid="calib-start"
								bind:value={start}
							/>
						{:else if key === 'end'}
							<input
								class={FIELD}
								type="number"
								step="any"
								data-testid="calib-end"
								bind:value={end}
							/>
						{:else}
							<input
								class={FIELD}
								type="number"
								step="any"
								data-testid="calib-step"
								bind:value={step}
							/>
						{/if}
					</label>
				{/each}
				{#if hasFixed}
					<label class="flex items-center justify-between gap-2 text-sm">
						<span>{FIXED_VALUE_LABELS[kind]}</span>
						<input
							class={FIELD}
							type="number"
							step="any"
							data-testid="calib-fixed"
							bind:value={fixed}
						/>
					</label>
				{/if}
				{#if invalid}<p class="text-sm text-danger">{invalid}</p>{/if}
				{#if error}<p class="text-sm text-danger" data-testid="engine-calib-error">{error}</p>{/if}
				<div class="flex justify-end gap-2">
					<button type="button" class="ws-btn ws-btn-secondary" disabled={busy} onclick={onclose}
						>Annuler</button
					>
					<button
						type="submit"
						data-testid="engine-calib-confirm"
						class="ws-btn ws-btn-primary"
						disabled={!!invalid || busy}>{busy ? 'Création…' : 'OK'}</button
					>
				</div>
			</form>
		</div>
	</div>
{/if}
