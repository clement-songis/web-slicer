<script lang="ts">
	import { locale, numberLocale, t } from '$lib/i18n';
	// Tableau « Filament » d'Orca (GCodeViewer::render_legend / render_all_plates_stats,
	// lot 31) : une ligne par filament utilisé, colonnes Modèle / Support / Purge /
	// Tour affichées seulement si un filament en consomme, puis Total ; chaque
	// cellule donne la longueur (m) et la masse (g). Données serveur
	// (`meta.filament_table` ou `plates-stats.all`) : ce composant ne calcule rien.
	import {
		FILAMENT_COLUMN_LABELS,
		visibleFilamentColumns,
		type FilamentTable,
		type FilamentUsage
	} from './engine-data';

	interface Props {
		table: FilamentTable;
		testid?: string;
	}
	let { table, testid = 'filament-table' }: Props = $props();
	const columns = $derived(visibleFilamentColumns(table));
	const n2 = (v: number) =>
		v.toLocaleString(numberLocale($locale), { minimumFractionDigits: 2, maximumFractionDigits: 2 });
	const used = (u: FilamentUsage | undefined): u is FilamentUsage => !!u && (u.m > 0 || u.g > 0);
</script>

{#snippet cell(u: FilamentUsage | undefined)}
	{#if used(u)}
		<span class="block">{n2(u.m)} m</span>
		<span class="block text-content-muted">{n2(u.g)} g</span>
	{:else}
		<span class="text-content-subtle">—</span>
	{/if}
{/snippet}

<table class="w-full border-collapse text-xs" data-testid={testid}>
	<thead>
		<tr class="text-content-muted">
			<th class="text-left font-normal">{$t('Filament')}</th>
			{#each columns as c (c)}
				<th class="pl-2 text-right font-normal" data-column={c}>{$t(FILAMENT_COLUMN_LABELS[c])}</th>
			{/each}
			<th class="pl-2 text-right font-normal">{$t('Total')}</th>
		</tr>
	</thead>
	<tbody>
		{#each table.rows as row (row.extruder)}
			<tr class="tabular-nums" data-extruder={row.extruder}>
				<td class="py-0.5">
					<span class="flex items-center gap-1.5">
						<span
							class="inline-block h-3 w-3 shrink-0 rounded-sm border border-border"
							style:background-color={row.color || 'transparent'}
						></span>
						{row.extruder + 1}
					</span>
				</td>
				{#each columns as c (c)}
					<td class="pl-2 text-right whitespace-nowrap">{@render cell(row[c])}</td>
				{/each}
				<td class="pl-2 text-right font-medium whitespace-nowrap">{@render cell(row.total)}</td>
			</tr>
		{/each}
	</tbody>
	{#if table.rows.length > 1}
		<tfoot>
			<tr class="border-t border-border font-medium tabular-nums">
				<td class="text-content-muted">Σ</td>
				{#each columns as c (c)}
					<td class="pl-2 text-right whitespace-nowrap">{@render cell(table.total[c])}</td>
				{/each}
				<td class="pl-2 text-right whitespace-nowrap" data-testid="{testid}-total">
					{@render cell(table.total.total)}
				</td>
			</tr>
		</tfoot>
	{/if}
</table>
