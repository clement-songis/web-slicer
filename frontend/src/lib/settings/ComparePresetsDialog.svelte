<script lang="ts">
	// Comparaison de deux presets du même type (Orca DiffPresetDialog) : seules
	// les options différentes, regroupées par onglet, valeurs résolues.
	import Dialog from '$lib/ui/Dialog.svelte';
	import { t } from '$lib/i18n';
	import { getResolvedPreset } from '$lib/api/presets';
	import type { PresetSummary } from '$lib/api/types';
	import type { PresetKind } from '../../generated/ui-layout';
	import { diffPresets, formatSetting, type SettingDiff } from './preset-tools';

	interface Props {
		open: boolean;
		kind: PresetKind;
		presets: PresetSummary[];
		leftId: string | null;
		rightId: string | null;
		onclose: () => void;
	}
	let { open, kind, presets, leftId, rightId, onclose }: Props = $props();

	let left = $state<string | null>(null);
	let right = $state<string | null>(null);
	let diffs = $state<SettingDiff[] | null>(null);
	let error = $state<string | null>(null);
	$effect(() => {
		if (open) {
			left = leftId;
			right = rightId;
		}
	});
	let generation = 0;
	$effect(() => {
		const [a, b] = [left, right];
		if (!open || !a || !b) return;
		const g = ++generation;
		diffs = null;
		error = null;
		void Promise.all([getResolvedPreset(a), getResolvedPreset(b)])
			.then(([ra, rb]) => {
				if (g === generation) diffs = diffPresets(kind, ra.values, rb.values);
			})
			.catch((e) => {
				if (g === generation) error = e instanceof Error ? e.message : 'comparaison impossible';
			});
	});
	const pages = $derived.by(() => {
		const out: { page: string; rows: SettingDiff[] }[] = [];
		for (const d of diffs ?? []) {
			const last = out[out.length - 1];
			if (last?.page === d.page) last.rows.push(d);
			else out.push({ page: d.page, rows: [d] });
		}
		return out;
	});
	const nameOf = (id: string | null) => presets.find((p) => p.id === id)?.name ?? '—';
</script>

<Dialog
	{open}
	label="Comparer des presets"
	title="Comparer des presets"
	size="xl"
	{onclose}
	data-testid="compare-dialog"
>
	<div class="flex flex-col gap-3">
		<div class="grid grid-cols-[1fr_auto_1fr] items-center gap-2">
			<select
				class="ws-field"
				bind:value={left}
				aria-label="Preset de gauche"
				data-testid="compare-left"
			>
				{#each presets as p (p.id)}<option value={p.id}>{p.name}</option>{/each}
			</select>
			<button
				type="button"
				class="ws-icon-btn"
				aria-label="Inverser"
				data-tip="Inverser"
				onclick={() => ([left, right] = [right, left])}>⇄</button
			>
			<select
				class="ws-field"
				bind:value={right}
				aria-label="Preset de droite"
				data-testid="compare-right"
			>
				{#each presets as p (p.id)}<option value={p.id}>{p.name}</option>{/each}
			</select>
		</div>
		{#if error}
			<p class="text-sm text-danger" role="alert">{error}</p>
		{:else if diffs === null}
			<p class="text-sm text-content-muted" role="status">Comparaison…</p>
		{:else if diffs.length === 0}
			<p
				class="ws-empty rounded-lg border border-dashed border-border"
				data-testid="compare-identical"
			>
				Aucune différence entre « {nameOf(left)} » et « {nameOf(right)} ».
			</p>
		{:else}
			<p class="text-xs text-content-muted" data-testid="compare-count">
				{diffs.length} option(s) différente(s)
			</p>
			<div class="max-h-[60vh] overflow-y-auto rounded border border-border">
				<table class="w-full text-sm">
					<thead class="sticky top-0 bg-surface-raised text-left text-xs text-content-muted">
						<tr>
							<th class="px-3 py-1.5 font-medium">Option</th>
							<th class="px-3 py-1.5 font-medium">{nameOf(left)}</th>
							<th class="px-3 py-1.5 font-medium">{nameOf(right)}</th>
						</tr>
					</thead>
					<tbody>
						{#each pages as group (group.page)}
							<tr
								><th colspan="3" class="bg-overlay px-3 py-1 text-left text-xs font-semibold"
									>{$t(group.page)}</th
								></tr
							>
							{#each group.rows as row (row.key)}
								<tr class="border-t border-border" data-testid="compare-row" data-key={row.key}>
									<td class="px-3 py-1 text-content-muted" title={row.key}>{$t(row.label)}</td>
									<td class="px-3 py-1 font-mono text-xs break-all">{formatSetting(row.left)}</td>
									<td class="px-3 py-1 font-mono text-xs break-all">{formatSetting(row.right)}</td>
								</tr>
							{/each}
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</div>
	{#snippet footer()}
		<button type="button" class="ws-btn ws-btn-secondary" onclick={onclose}>{$t('Close')}</button>
	{/snippet}
</Dialog>
