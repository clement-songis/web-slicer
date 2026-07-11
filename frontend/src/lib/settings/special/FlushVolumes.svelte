<script lang="ts">
	// Dialog « volumes de purge » (T045) : matrice NxN des volumes filament→filament
	// + multiplicateur global. La matrice est stockée aplatie (coFloats).
	import {
		flattenMatrix,
		FLUSH_KEYS,
		matrixSize,
		parseNumbers,
		serializeNumbers,
		toMatrix
	} from './dialogs';

	interface Props {
		/** Valeurs par clé (liable). */
		values?: Record<string, unknown>;
	}

	let { values = $bindable({}) }: Props = $props();

	const flat = $derived(parseNumbers(values[FLUSH_KEYS.matrix]));
	const size = $derived(matrixSize(flat));
	const matrix = $derived(toMatrix(flat, size));
	const multiplier = $derived(parseNumbers(values[FLUSH_KEYS.multiplier])[0] ?? 1);

	function setCell(row: number, col: number, v: number) {
		const next = toMatrix(parseNumbers(values[FLUSH_KEYS.matrix]), size);
		next[row][col] = v;
		values[FLUSH_KEYS.matrix] = serializeNumbers(flattenMatrix(next));
	}

	function setMultiplier(v: number) {
		values[FLUSH_KEYS.multiplier] = serializeNumbers([v]);
	}

	const CELL =
		'w-16 rounded border border-border-strong px-1 py-0.5 text-center text-xs bg-surface-raised text-content';
</script>

<div class="flex flex-col gap-3">
	<label class="flex items-center gap-2 text-sm">
		<span class="text-content-muted">Multiplicateur de purge</span>
		<input
			type="number"
			step="any"
			min="0"
			value={multiplier}
			oninput={(e) => setMultiplier(Number(e.currentTarget.value))}
			class={CELL}
		/>
	</label>

	{#if size > 0}
		<div class="overflow-x-auto">
			<table class="border-collapse text-xs">
				<tbody>
					{#each matrix as rowValues, row (row)}
						<tr>
							{#each rowValues as cell, col (col)}
								<td class="p-0.5">
									<input
										type="number"
										step="any"
										min="0"
										disabled={row === col}
										value={cell}
										oninput={(e) => setCell(row, col, Number(e.currentTarget.value))}
										class="{CELL} disabled:opacity-40"
									/>
								</td>
							{/each}
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{:else}
		<p class="text-xs text-content-subtle">
			Matrice de purge indisponible (aucun filament configuré).
		</p>
	{/if}
</div>
