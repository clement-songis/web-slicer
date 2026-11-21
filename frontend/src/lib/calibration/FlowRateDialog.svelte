<script lang="ts">
	import { dialogFocus } from '$lib/ui/dialog-focus';
	// FlowRateCalibrationDialog d'Orca : quatre types de test et motif de surface.
	import type { FlowPass, FlowPattern } from './flowrate';
	import { t } from '$lib/i18n';

	interface Props {
		open: boolean;
		busy?: boolean;
		error?: string | null;
		onclose: () => void;
		onconfirm: (pass: FlowPass, linear: boolean, pattern: FlowPattern) => void;
	}
	let { open, busy = false, error = null, onclose, onconfirm }: Props = $props();
	let type = $state(2);
	let pattern = $state<FlowPattern>('archimedeanchords');
	const types = ['Pass 1 (Coarse)', 'Pass 2 (Fine)', 'YOLO (Recommended)', 'YOLO (Perfectionist)'];
	function submit(event: SubmitEvent) {
		event.preventDefault();
		if (!busy) onconfirm(((type % 2) + 1) as FlowPass, type >= 2, pattern);
	}
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
			aria-label={$t('Flow Rate Calibration')}
			data-testid="flow-calib-dialog"
			tabindex="-1"
			use:dialogFocus
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.key === 'Escape' && !busy && onclose()}
		>
			<form class="flex flex-col gap-4" onsubmit={submit}>
				<h2 class="ws-dialog-title">{$t('Flow Ratio Calibration')}</h2>
				<fieldset class="flex flex-col gap-2">
					<legend class="mb-2 text-sm font-medium">{$t('Calibration Test Type')}</legend>
					{#each types as label, i (label)}
						<label class="flex items-center gap-2 text-sm">
							<input
								type="radio"
								name="flow-type"
								value={i}
								bind:group={type}
								data-testid={`flow-type-${i}`}
							/>
							{$t(label)}
						</label>
					{/each}
				</fieldset>
				<label class="flex items-center justify-between gap-3 text-sm">
					<span>{$t('Top surface pattern')}</span>
					<select bind:value={pattern} data-testid="flow-pattern" class="ws-field">
						<option value="archimedeanchords">{$t('Archimedean Chords')}</option>
						<option value="monotonic">{$t('Monotonic')}</option>
					</select>
				</label>
				<a
					class="text-sm text-accent underline"
					href="https://www.orcaslicer.com/wiki/flow_ratio_calib"
					target="_blank"
					rel="noreferrer">{$t('Wiki Guide')}</a
				>
				{#if error}<p class="text-sm text-danger" role="alert">{error}</p>{/if}
				<div class="flex justify-end gap-2">
					<button type="button" class="ws-btn ws-btn-secondary" disabled={busy} onclick={onclose}
						>{$t('Cancel')}</button
					>
					<button
						type="submit"
						data-testid="flow-calib-confirm"
						class="ws-btn ws-btn-primary"
						disabled={busy}>{busy ? $t('Creating…') : $t('OK')}</button
					>
				</div>
			</form>
		</div>
	</div>
{/if}
