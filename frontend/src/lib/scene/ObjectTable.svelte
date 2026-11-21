<script lang="ts">
	// Tableau des paramètres d'objets (Orca « Modifier dans la Table des
	// Paramètres », ObjectGridTable) : grille éditable des réglages courants de
	// chaque objet et de ses pièces / modificateurs. Plus direct qu'Orca : chaque
	// cellule se modifie sur place (Entrée / sortie du champ), la remise au
	// profil est sur la cellule, les valeurs héritées sont en gris.
	// Présentationnel : les mutations passent par des callbacks.
	import Dialog from '$lib/ui/Dialog.svelte';
	import { t, tr } from '$lib/i18n';
	import IconReset from '~icons/lucide/rotate-ccw';
	import {
		TABLE_CATEGORIES,
		boolValue,
		cellText,
		cellValue,
		cellWarning,
		parseCell,
		sortRows,
		tableRows,
		visibleColumns,
		type SortKey,
		type TableCategory,
		type TableColumn,
		type TableObjectSource,
		type TableRow
	} from './object-table';

	interface Props {
		open: boolean;
		objects: readonly TableObjectSource[];
		/** Valeurs du profil d'impression (héritées par les objets). */
		inherited: Readonly<Record<string, unknown>>;
		/** Couleurs des filaments (slot i ↔ filament i + 1). */
		filamentColors: readonly string[];
		/** Ligne mise en avant à l'ouverture (PopupObjectTableBySelection). */
		focusId?: string | null;
		onclose: () => void;
		onsetting: (row: TableRow, key: string, value: unknown) => void;
		onextruder: (row: TableRow, extruder: number) => void;
		onrename: (row: TableRow, name: string) => void;
		onprintable: (objectId: string, printable: boolean) => void;
		/** Hauteur de couche maximale recommandée de l'imprimante (avertissement). */
		maxLayer?: number | null;
	}

	let {
		open,
		objects,
		inherited,
		filamentColors,
		focusId = null,
		onclose,
		onsetting,
		onextruder,
		onrename,
		onprintable,
		maxLayer = null
	}: Props = $props();
	let warning = $state<string | null>(null);

	let category = $state<TableCategory>('all');
	let sortKey = $state<SortKey | null>(null);
	let descending = $state(false);
	let error = $state<string | null>(null);

	const columns = $derived(visibleColumns(category));
	const rows = $derived(sortRows(tableRows(objects), sortKey, descending));
	const settingsOf = $derived(new Map(objects.map((o) => [o.id, o.settings])));

	function toggleSort(key: SortKey) {
		if (sortKey === key) descending = !descending;
		else {
			sortKey = key;
			descending = false;
		}
	}

	function commit(row: TableRow, column: TableColumn, raw: string | boolean) {
		const value = parseCell(column, raw);
		if (value === null) {
			error = tr('“{value}”: invalid value for {column}.', {
				value: String(raw),
				column: tr(column.label)
			});
			return;
		}
		error = null;
		warning =
			cellWarning(column, value, maxLayer) && maxLayer !== null
				? tr('Too large layer height: the printer recommends at most {max} mm.', {
						max: String(maxLayer)
					})
				: null;
		onsetting(row, column.key, value);
	}

	function rowKey(row: TableRow) {
		return row.volumeId ? `${row.objectId}/${row.volumeId}` : row.objectId;
	}
</script>

<Dialog
	{open}
	label={$t('Object parameter table')}
	title={$t('Object parameter table')}
	size="xl"
	{onclose}
	data-testid="object-table"
>
	<div class="flex flex-col gap-2 text-sm">
		<div class="flex flex-wrap items-center gap-2">
			<label class="flex items-center gap-2">
				<span class="text-content-muted">{$t('Category')}</span>
				<select
					class="ws-field ws-field-sm"
					data-testid="object-table-category"
					bind:value={category}
				>
					{#each TABLE_CATEGORIES as c (c.value)}
						<option value={c.value}>{$t(c.label)}</option>
					{/each}
				</select>
			</label>
			<span class="text-xs text-content-subtle"
				>{$t('Gray values: inherited from the profile. ↺ resets the cell to the profile.')}</span
			>
		</div>
		{#if error}
			<p class="text-xs text-danger" role="alert" data-testid="object-table-error">{error}</p>
		{/if}
		{#if warning}
			<p class="text-xs text-warning-content" role="status" data-testid="object-table-warning">
				{warning}
			</p>
		{/if}
		<div class="max-h-[60vh] overflow-auto rounded border border-border">
			<table class="w-full border-collapse text-xs">
				<thead class="sticky top-0 z-10 bg-surface-raised">
					<tr>
						{#each columns as column (column.key)}
							<th
								class="max-w-28 border-b border-border px-1.5 py-1 text-left align-bottom leading-tight font-medium"
								scope="col"
								data-col={column.key}
								aria-sort={sortKey === column.key
									? descending
										? 'descending'
										: 'ascending'
									: undefined}
							>
								{#if column.sortable}
									<button
										type="button"
										class="hover:text-accent"
										data-testid="object-table-sort"
										onclick={() => toggleSort(column.key as SortKey)}
										>{$t(column.label)}
										<span aria-hidden="true"
											>{sortKey === column.key ? (descending ? '↓' : '↑') : '↑↓'}</span
										></button
									>
								{:else}
									{$t(column.label)}{#if column.unit}<span
											class="block font-normal text-content-subtle"
											data-testid="object-table-unit">({column.unit})</span
										>{/if}
								{/if}
							</th>
						{/each}
					</tr>
				</thead>
				<tbody>
					{#each rows as row (rowKey(row))}
						<tr
							class="border-b border-border/60 {row.kind === 'volume'
								? 'bg-surface-sunken/40'
								: ''} {focusId === row.objectId && row.kind === 'object' ? 'ws-search-hit' : ''}"
							data-testid="object-table-row"
							data-kind={row.kind}
							data-object-id={row.objectId}
							data-volume-id={row.volumeId}
						>
							{#each columns as column (column.key)}
								<td class="px-1.5 py-0.5 align-middle" data-col={column.key}>
									{#if row.kind === 'volume' && column.objectOnly}
										<span class="text-content-subtle" aria-hidden="true">—</span>
									{:else if column.kind === 'printable'}
										<input
											type="checkbox"
											checked={row.printable}
											aria-label={$t('Printable: {name}', { name: row.name })}
											onchange={(e) => onprintable(row.objectId, e.currentTarget.checked)}
										/>
									{:else if column.kind === 'plate'}
										<span class="tabular-nums">{row.plate || '—'}</span>
									{:else if column.kind === 'name'}
										<input
											class="ws-field ws-field-sm {row.kind === 'volume' ? 'ml-3 w-28' : 'w-32'}"
											value={row.name}
											aria-label={$t('Name of {name}', { name: row.name })}
											onkeydown={(e) => {
												e.stopPropagation();
												if (e.key === 'Enter') e.currentTarget.blur();
											}}
											onchange={(e) => {
												const name = e.currentTarget.value.trim();
												if (name && name !== row.name) onrename(row, name);
											}}
										/>
									{:else if column.kind === 'extruder'}
										<span class="flex items-center gap-1">
											<span
												class="h-3 w-3 shrink-0 rounded-full ring-1 ring-border-strong"
												style:background-color={filamentColors[(row.extruder || 1) - 1] ??
													'transparent'}
												aria-hidden="true"
											></span>
											<select
												class="ws-field ws-field-sm h-6 px-0.5"
												aria-label={$t('Filament of {name}', { name: row.name })}
												data-testid="object-table-extruder"
												value={row.kind === 'object' ? row.extruder || 1 : row.extruder}
												onchange={(e) => onextruder(row, Number(e.currentTarget.value))}
											>
												{#if row.kind === 'volume'}<option value={0}
														>{$t('object||extruder')}</option
													>{/if}
												{#each filamentColors.map((_c, i) => i + 1) as n (n)}
													<option value={n}>{n}</option>
												{/each}
											</select>
										</span>
									{:else}
										{@const cell = cellValue(
											row,
											column,
											inherited,
											row.kind === 'volume' ? settingsOf.get(row.objectId) : undefined
										)}
										<span class="flex items-center gap-1" data-overridden={cell.overridden}>
											{#if column.kind === 'bool'}
												<input
													type="checkbox"
													checked={boolValue(cell.value)}
													aria-label={$t('{column}: {name}', {
														column: $t(column.label),
														name: row.name
													})}
													onchange={(e) => commit(row, column, e.currentTarget.checked)}
												/>
											{:else if column.kind === 'enum'}
												<select
													class="ws-field ws-field-sm h-6 w-28 px-0.5 {cell.overridden
														? ''
														: 'text-content-subtle'}"
													aria-label={$t('{column}: {name}', {
														column: $t(column.label),
														name: row.name
													})}
													value={String(cell.value ?? '')}
													onchange={(e) => commit(row, column, e.currentTarget.value)}
												>
													{#each column.choices ?? [] as choice (choice.value)}
														<option value={choice.value}>{$t(choice.label)}</option>
													{/each}
												</select>
											{:else}
												<input
													class="ws-field ws-field-sm w-16 tabular-nums {cell.overridden
														? ''
														: 'text-content-subtle'}"
													inputmode="decimal"
													value={cellText(column, cell.value)}
													aria-label={$t('{column}: {name}', {
														column: $t(column.label),
														name: row.name
													})}
													data-testid="object-table-cell"
													data-key={column.key}
													onkeydown={(e) => {
														e.stopPropagation();
														if (e.key === 'Enter') e.currentTarget.blur();
													}}
													onchange={(e) => commit(row, column, e.currentTarget.value)}
												/>
											{/if}
											{#if cell.overridden}
												<button
													type="button"
													class="ws-icon-btn ws-icon-btn-xs"
													data-testid="object-table-reset"
													aria-label={$t('Reset {column} to the profile for {name}', {
														column: $t(column.label),
														name: row.name
													})}
													data-tip={$t('Profile: {value}', {
														value: cellText(column, cell.inherited) || '—'
													})}
													onclick={() => onsetting(row, column.key, undefined)}
												>
													<IconReset class="h-3 w-3" aria-hidden="true" />
												</button>
											{/if}
										</span>
									{/if}
								</td>
							{/each}
						</tr>
					{/each}
				</tbody>
			</table>
			{#if rows.length === 0}
				<p class="p-4 text-center text-content-muted">{$t('No objects on the plates.')}</p>
			{/if}
		</div>
	</div>
	{#snippet footer()}
		<button type="button" class="ws-btn ws-btn-primary" onclick={onclose}>{$t('Close')}</button>
	{/snippet}
</Dialog>
