<script lang="ts">
	// Volumes de purge du projet (Orca : « Flushing volumes », config projet) :
	// matrice départ → arrivée par paire de filaments, en-têtes aux couleurs des
	// slots, multiplicateur global. Brouillon local appliqué par « Appliquer ».
	import { untrack } from 'svelte';
	import Dialog from '$lib/ui/Dialog.svelte';
	import { swatchStyle } from '$lib/ui/color';
	import { t, tr } from '$lib/i18n';
	import { DEFAULT_FLUSH, resizeFlushMatrix } from './multi-filament';
	import { appPreferences, savePreferences, type AutoFlush } from '$lib/settings/preferences';

	interface Props {
		open: boolean;
		/** Couleur CSS de chaque slot (nombre de slots = taille de la matrice). */
		colours: string[];
		matrix?: number[];
		multiplier?: number;
		/** Multiplicateur effectif sans surcharge projet (preset, défaut Orca 0,3). */
		defaultMultiplier?: number;
		onclose: () => void;
		onapply: (matrix: number[], multiplier: number) => void;
		/** Calcul moteur depuis les couleurs (null = jamais disponible). */
		compute?: ((colours: string[], multiplier: number) => Promise<number[]>) | null;
		/** Sonde de disponibilité du calcul moteur. */
		computeAvailable?: (() => Promise<boolean>) | null;
	}

	let {
		open,
		colours,
		matrix,
		multiplier = 1,
		defaultMultiplier = 0.3,
		onclose,
		onapply,
		compute = null,
		computeAvailable = null
	}: Props = $props();

	let canCompute = $state<boolean | null>(null);
	let computing = $state(false);
	let computeError = $state<string | null>(null);
	$effect(() => {
		if (!open || !computeAvailable) return;
		canCompute = null;
		computeError = null;
		void computeAvailable().then((ok) => (canCompute = ok));
	});
	async function runCompute() {
		if (!compute || computing) return;
		computing = true;
		computeError = null;
		try {
			const m = await compute(colours, 1);
			if (m.length === colours.length * colours.length) draft = m.map((v) => Math.round(v));
			else computeError = tr('Unexpected engine response (matrix size).');
		} catch (e) {
			computeError = e instanceof Error ? e.message : tr('Calculation failed.');
		} finally {
			computing = false;
		}
	}

	const n = $derived(colours.length);
	let draft = $state<number[]>([]);
	let factor = $state(1);
	// Brouillon réinitialisé à chaque ouverture seulement : un recalcul
	// automatique reçu pendant la saisie (props modifiées) n'écrase ni les
	// volumes saisis ni le multiplicateur ; « Appliquer » pose le brouillon.
	$effect(() => {
		if (!open) return;
		untrack(() => {
			draft = resizeFlushMatrix(matrix, colours.length);
			factor = multiplier;
		});
	});

	const at = (r: number, c: number) => draft[r * n + c] ?? 0;
	function setCell(r: number, c: number, raw: string) {
		const v = Number(raw);
		if (!Number.isFinite(v) || v < 0) return;
		const next = [...draft];
		next[r * n + c] = Math.round(v);
		draft = next;
	}
	const maxVolume = $derived(Math.max(0, ...draft) * factor);
</script>

<Dialog
	{open}
	label={$t('Flushing volumes')}
	title={$t('Flushing volumes')}
	size="lg"
	{onclose}
	data-testid="flush-dialog"
>
	<div class="flex flex-col gap-4">
		<p class="text-sm text-content-muted">
			{$t(
				'Volume extruded into the prime tower (mm³) at each filament change, from the row (unloaded filament) to the column (loaded filament). Going from light to dark needs less flushing than the reverse.'
			)}
		</p>
		<div class="overflow-x-auto">
			<table class="border-separate border-spacing-1 text-xs" data-testid="flush-matrix">
				<thead>
					<tr>
						<th scope="col" class="text-left font-normal text-content-subtle"
							>{$t('From ↓ / To →')}</th
						>
						{#each colours as colour, c (c)}
							<th scope="col">
								<span
									class="inline-flex h-6 w-14 items-center justify-center rounded font-semibold ring-1 ring-border-strong"
									style={swatchStyle(colour)}>{c + 1}</span
								>
							</th>
						{/each}
					</tr>
				</thead>
				<tbody>
					{#each colours as colour, r (r)}
						<tr>
							<th scope="row">
								<span
									class="inline-flex h-6 w-10 items-center justify-center rounded font-semibold ring-1 ring-border-strong"
									style={swatchStyle(colour)}>{r + 1}</span
								>
							</th>
							{#each colours.keys() as c (c)}
								<td>
									{#if r === c}
										<span class="block w-14 text-center text-content-subtle" aria-hidden="true"
											>—</span
										>
									{:else}
										<input
											type="number"
											min="0"
											step="10"
											class="ws-field ws-field-sm w-14 text-center tabular-nums"
											aria-label={$t('Flushing from filament {a} to filament {b} (mm³)', {
												a: r + 1,
												b: c + 1
											})}
											data-testid={`flush-cell-${r + 1}-${c + 1}`}
											value={at(r, c)}
											onchange={(e) => setCell(r, c, e.currentTarget.value)}
										/>
									{/if}
								</td>
							{/each}
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
		<div class="flex flex-wrap items-center gap-3 text-sm">
			<label class="flex items-center gap-2">
				<span class="text-content-muted">{$t('Multiplier')}</span>
				<input
					type="number"
					min="0.1"
					step="0.1"
					class="ws-field ws-field-sm w-20 tabular-nums"
					data-testid="flush-multiplier"
					bind:value={factor}
				/>
			</label>
			<span class="text-xs text-content-subtle" data-testid="flush-max"
				>{$t('Maximum flushing: {v} mm³', { v: Math.round(maxVolume) })}</span
			>
			<button
				type="button"
				class="ws-btn ws-btn-sm ws-btn-secondary ml-auto"
				data-testid="flush-compute"
				data-available={canCompute === null ? 'unknown' : String(canCompute)}
				disabled={!compute || canCompute !== true || computing}
				title={canCompute === false || !compute
					? $t('Automatic calculation unavailable: the engine does not expose it yet.')
					: $t('Volumes calculated by the engine from the colors (like Orca)')}
				onclick={runCompute}>{computing ? $t('Calculating…') : $t('Calculate from colors')}</button
			>
			<button
				type="button"
				class="ws-btn ws-btn-sm ws-btn-ghost"
				onclick={() => {
					draft = resizeFlushMatrix(undefined, n);
					factor = defaultMultiplier;
				}}
				>{$t('Default values ({v} mm³ × {m})', { v: DEFAULT_FLUSH, m: defaultMultiplier })}</button
			>
		</div>
		<!-- Orca auto_calculate_flush : même préférence que Préférences › Comportement. -->
		<label class="flex items-center justify-between gap-2 text-xs text-content-muted">
			<span>{$t('Recalculate automatically when a filament changes')}</span>
			<select
				class="ws-field ws-field-sm w-44"
				data-testid="flush-auto"
				value={$appPreferences.autoFlush}
				onchange={(e) =>
					savePreferences({ ...$appPreferences, autoFlush: e.currentTarget.value as AutoFlush })}
			>
				<option value="all">{$t('Color or filament')}</option>
				<option value="color">{$t('Color only')}</option>
				<option value="disabled">{$t('Never')}</option>
			</select>
		</label>
		{#if computeError}
			<p class="text-xs text-danger" role="alert">{computeError}</p>
		{:else if canCompute === false}
			<p class="text-xs text-content-subtle" data-testid="flush-compute-unavailable">
				{$t(
					'Calculation from colors will be available when the engine exposes it; enter the volumes by hand in the meantime.'
				)}
			</p>
		{/if}
	</div>
	{#snippet footer()}
		<button type="button" class="ws-btn ws-btn-secondary" onclick={onclose}>{$t('Cancel')}</button>
		<button
			type="button"
			class="ws-btn ws-btn-primary"
			data-testid="flush-apply"
			onclick={() => {
				onapply(draft, factor > 0 ? factor : 1);
				onclose();
			}}>{$t('Apply')}</button
		>
	{/snippet}
</Dialog>
