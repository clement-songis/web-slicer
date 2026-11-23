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
		parseNumberList,
		SHAPER_TYPES,
		STEPLESS_KINDS,
		TEST_MODEL_KINDS,
		testModels,
		validateRange,
		type EngineCalibKind,
		type RangeParams
	} from './engine-calib';
	import { t } from '$lib/i18n';

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
	// Input shaping : plage / fréquence Y, modèle de tour, type de shaper.
	let startY = $state(0);
	let endY = $state(0);
	let fixedY = $state(0);
	let testModel = $state(0);
	let shaperType = $state<string>('Default');
	// Motif PA : numéros, vitesses et accélérations (vides = valeurs du process).
	let printNumbers = $state(true);
	let speedsRaw = $state('');
	let accelsRaw = $state('');
	// Valeurs par défaut d'Orca à chaque ouverture / changement de calibration.
	$effect(() => {
		if (open) {
			const defaults = ENGINE_CALIB_DEFAULTS[kind];
			({ start, end, step } = defaults);
			fixed = defaults.fixed ?? 0;
			startY = defaults.start;
			endY = defaults.end;
			fixedY = defaults.fixed ?? 0;
			testModel = 0;
			shaperType = 'Default';
			printNumbers = true;
			speedsRaw = '';
			accelsRaw = '';
		}
	});

	const stepless = $derived(STEPLESS_KINDS.includes(kind));
	const hasFixed = $derived(FIXED_VALUE_KINDS.includes(kind));
	const fields = $derived(
		stepless
			? [
					['Start||range', 'start'],
					['End||range', 'end']
				]
			: kind === 'pa_pattern'
				? [
						// Orca PA_Calibration_Dlg : « Start PA / End PA / PA step ».
						['Start PA (K)', 'start'],
						['End PA (K)', 'end'],
						['PA step', 'step']
					]
				: [
						['Start||range', 'start'],
						['End||range', 'end'],
						['Step||range', 'step']
					]
	);
	const hasModel = $derived(TEST_MODEL_KINDS.includes(kind));
	const isShaping = $derived(kind === 'input_shaping_freq' || kind === 'input_shaping_damp');
	const speeds = $derived(speedsRaw.trim() ? parseNumberList(speedsRaw) : []);
	const accels = $derived(accelsRaw.trim() ? parseNumberList(accelsRaw) : []);
	const params = $derived<RangeParams>({
		start,
		end,
		step: stepless ? 1 : step,
		...(hasFixed ? { fixed } : {}),
		...(kind === 'input_shaping_freq' ? { startY, endY } : {}),
		...(kind === 'input_shaping_damp' ? { fixedY } : {}),
		...(hasModel ? { testModel } : {}),
		...(isShaping ? { shaperType } : {}),
		...(kind === 'pa_pattern'
			? { printNumbers, speeds: speeds ?? [], accelerations: accels ?? [] }
			: {})
	});
	const invalid = $derived.by(() => {
		if (kind === 'pa_pattern' && (speeds === null || accels === null))
			return $t('speeds / accelerations: positive numbers separated by commas');
		void $t; // message traduit : recalcul au changement de langue
		return validateRange(params, kind);
	});

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
			aria-label={$t('Calibration {name}', { name: $t(ENGINE_CALIB_LABELS[kind]) })}
			data-testid="engine-calib-dialog"
			tabindex="-1"
			use:dialogFocus
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.key === 'Escape' && !busy && onclose()}
		>
			<form class="flex flex-col gap-3" onsubmit={submit}>
				<h2 class="ws-dialog-title">
					{$t('Calibration: {name}', { name: $t(ENGINE_CALIB_LABELS[kind]) })}
				</h2>
				{#each fields as [label, key] (key)}
					<label class="flex items-center justify-between gap-2 text-sm">
						<span>
							{$t(label)}{ENGINE_CALIB_UNITS[kind] ? ` (${ENGINE_CALIB_UNITS[kind]})` : ''}
						</span>
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
						<span>{$t(FIXED_VALUE_LABELS[kind] ?? '')}</span>
						<input
							class={FIELD}
							type="number"
							step="any"
							data-testid="calib-fixed"
							bind:value={fixed}
						/>
					</label>
				{/if}
				{#if kind === 'input_shaping_freq'}
					<label class="flex items-center justify-between gap-2 text-sm">
						<span>{$t('Y start (Hz)')}</span>
						<input
							class={FIELD}
							type="number"
							step="any"
							data-testid="calib-start-y"
							bind:value={startY}
						/>
					</label>
					<label class="flex items-center justify-between gap-2 text-sm">
						<span>{$t('Y end (Hz)')}</span>
						<input
							class={FIELD}
							type="number"
							step="any"
							data-testid="calib-end-y"
							bind:value={endY}
						/>
					</label>
				{:else if kind === 'input_shaping_damp'}
					<label class="flex items-center justify-between gap-2 text-sm">
						<span>{$t('Y frequency (Hz)')}</span>
						<input
							class={FIELD}
							type="number"
							step="any"
							data-testid="calib-fixed-y"
							bind:value={fixedY}
						/>
					</label>
				{/if}
				{#if hasModel}
					<label class="flex items-center justify-between gap-2 text-sm">
						<span>{$t('Test model')}</span>
						<select class="{FIELD} w-36" data-testid="calib-test-model" bind:value={testModel}>
							{#each testModels(kind) as m (m.id)}<option value={m.id}>{$t(m.label)}</option>{/each}
						</select>
					</label>
				{/if}
				{#if isShaping}
					<label class="flex items-center justify-between gap-2 text-sm">
						<span>{$t('Shaper type')}</span>
						<select class="{FIELD} w-36" data-testid="calib-shaper-type" bind:value={shaperType}>
							{#each SHAPER_TYPES as s (s)}<option value={s}>
									{s === 'Default' ? $t('Default') : s}
								</option>{/each}
						</select>
					</label>
				{/if}
				{#if kind === 'pa_pattern'}
					<label class="flex items-center gap-2 text-sm">
						<input type="checkbox" data-testid="calib-print-numbers" bind:checked={printNumbers} />
						{$t('Print numbers')}
					</label>
					<label class="flex flex-col gap-1 text-sm">
						<span>{$t('Accelerations (mm/s², optional)')}</span>
						<input
							class="{FIELD} w-full"
							data-testid="calib-accels"
							placeholder={$t('e.g. {values}', { values: '1000, 3000' })}
							bind:value={accelsRaw}
						/>
					</label>
					<label class="flex flex-col gap-1 text-sm">
						<span>{$t('Speeds (mm/s, optional)')}</span>
						<input
							class="{FIELD} w-full"
							data-testid="calib-speeds"
							placeholder={$t('e.g. {values}', { values: '100, 200' })}
							bind:value={speedsRaw}
						/>
					</label>
				{/if}
				{#if invalid}<p class="text-sm text-danger">{invalid}</p>{/if}
				{#if error}<p class="text-sm text-danger" data-testid="engine-calib-error">{error}</p>{/if}
				<div class="flex justify-end gap-2">
					<button type="button" class="ws-btn ws-btn-secondary" disabled={busy} onclick={onclose}>
						{$t('Cancel')}
					</button>
					<button
						type="submit"
						data-testid="engine-calib-confirm"
						class="ws-btn ws-btn-primary"
						disabled={!!invalid || busy}
					>
						{busy ? $t('Creating…') : $t('OK')}
					</button>
				</div>
			</form>
		</div>
	</div>
{/if}
