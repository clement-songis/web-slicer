<script lang="ts">
	// Dialog « températures par type de plaque » (T045) : table plaque × (temp
	// normale, temp 1re couche). Chaque cellule est un coInts à une entrée.
	import { parseNumbers, PLATE_TYPES, serializeNumbers } from './dialogs';
	import { t } from '#lib/i18n';

	interface Props {
		/** Valeurs par clé (liable). */
		values?: Record<string, unknown>;
	}

	let { values = $bindable({}) }: Props = $props();

	function temp(key: string): number | '' {
		return parseNumbers(values[key])[0] ?? '';
	}

	function setTemp(key: string, v: string) {
		values[key] = v === '' ? [] : serializeNumbers([Number(v)]);
	}

	const CELL =
		'w-20 rounded border border-border-strong px-1 py-0.5 text-center text-sm bg-surface-raised text-content';
</script>

<div class="overflow-x-auto">
	<table class="text-sm">
		<thead>
			<tr class="text-left text-xs text-content-subtle">
				<th class="py-1 pr-4 font-medium">{$t('Plate type')}</th>
				<th class="px-2 py-1 font-medium">{$t('Other layers (°C)')}</th>
				<th class="px-2 py-1 font-medium">{$t('First layer (°C)')}</th>
			</tr>
		</thead>
		<tbody>
			{#each PLATE_TYPES as plate (plate.tempKey)}
				<tr class="border-t border-border">
					<td class="py-1 pr-4 text-content-muted">{$t(plate.label)}</td>
					<td class="px-2 py-1">
						<input
							type="number"
							min="0"
							value={temp(plate.tempKey)}
							oninput={(e) => setTemp(plate.tempKey, e.currentTarget.value)}
							class={CELL}
						/>
					</td>
					<td class="px-2 py-1">
						<input
							type="number"
							min="0"
							value={temp(plate.initialKey)}
							oninput={(e) => setTemp(plate.initialKey, e.currentTarget.value)}
							class={CELL}
						/>
					</td>
				</tr>
			{/each}
		</tbody>
	</table>
</div>
