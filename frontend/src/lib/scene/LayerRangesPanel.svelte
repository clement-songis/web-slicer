<script lang="ts">
	// Panneau des plages de hauteur d'un objet (Orca `GUI_ObjectLayers`) : bornes
	// [min, max] éditables, ajout après une plage (« + »), suppression, et
	// réglages propres de chaque plage (`layer_config_ranges`).
	import ObjectSettingsPanel from './ObjectSettingsPanel.svelte';
	import type { DisplayMode } from '$lib/settings/filter';
	import { addRangeAfter, removeRange, setRangeBounds, type LayerRange } from './layer-ranges';
	import IconX from '~icons/lucide/x';
	import IconPlus from '~icons/lucide/plus';

	interface Props {
		ranges: LayerRange[];
		/** Hauteur de couche par défaut d'une nouvelle plage. */
		layerHeight: number;
		mode?: DisplayMode;
		base?: Record<string, unknown>;
		onchange: (ranges: LayerRange[]) => void;
	}

	let { ranges, layerHeight, mode, base, onchange }: Props = $props();

	function bound(index: number, which: 'min' | 'max', raw: string) {
		const value = Number(raw);
		if (!Number.isFinite(value)) return;
		const r = ranges[index];
		onchange(
			setRangeBounds(
				ranges,
				index,
				which === 'min' ? value : r.min,
				which === 'max' ? value : r.max
			)
		);
	}
</script>

<section class="flex flex-col gap-2 rounded border border-border p-2" data-testid="layer-ranges">
	<div class="flex items-center gap-1.5">
		<span class="text-sm font-medium text-content">Plages de hauteur</span>
		<span class="ws-badge h-4 px-1.5 tabular-nums">{ranges.length}</span>
	</div>
	{#each ranges as range, i (`${range.min}-${range.max}`)}
		<div class="flex flex-col gap-1 rounded bg-surface-sunken p-1" data-testid="layer-range">
			<div class="flex items-center gap-1 text-xs">
				<label class="flex items-center gap-1">
					<span class="text-content-muted">Z min</span>
					<input
						type="number"
						step="0.1"
						min="0"
						class="ws-field ws-field-sm w-16"
						aria-label={`Z min de la plage ${i + 1}`}
						value={range.min}
						onchange={(e) => bound(i, 'min', e.currentTarget.value)}
					/>
				</label>
				<label class="flex items-center gap-1">
					<span class="text-content-muted">Z max</span>
					<input
						type="number"
						step="0.1"
						min="0"
						class="ws-field ws-field-sm w-16"
						aria-label={`Z max de la plage ${i + 1}`}
						value={range.max}
						onchange={(e) => bound(i, 'max', e.currentTarget.value)}
					/>
					<span class="text-content-subtle">mm</span>
				</label>
				<button
					type="button"
					class="ws-icon-btn ws-icon-btn-xs ml-auto"
					title="Ajouter une plage après"
					aria-label={`Ajouter une plage après la plage ${i + 1}`}
					onclick={() => onchange(addRangeAfter(ranges, i, layerHeight))}
					><IconPlus class="h-3.5 w-3.5" aria-hidden="true" /></button
				>
				<button
					type="button"
					class="ws-icon-btn ws-icon-btn-xs"
					aria-label={`Supprimer la plage ${i + 1}`}
					onclick={() => onchange(removeRange(ranges, i))}
					><IconX class="h-3.5 w-3.5" aria-hidden="true" /></button
				>
			</div>
			<ObjectSettingsPanel
				title={`Réglages ${range.min}–${range.max} mm`}
				settings={range.settings}
				{mode}
				{base}
				testid="layer-range-settings"
				onchange={(settings) => onchange(ranges.map((r, k) => (k === i ? { ...r, settings } : r)))}
			/>
		</div>
	{/each}
</section>
