<script lang="ts">
	// Point 2D (coPoint*) : coordonnées X/Y. Les types vectoriels de points
	// éditent ici la première entrée ; le détail par extrudeur relève de T042.
	import { FIELD_CLASS, type WidgetProps } from './types';

	let { def, value = $bindable(), disabled = false }: WidgetProps<number[]> = $props();

	// Garantit un couple [x, y] éditable même si la valeur arrive incomplète.
	$effect(() => {
		if (!Array.isArray(value) || value.length < 2) {
			value = [value?.[0] ?? 0, value?.[1] ?? 0];
		}
	});
</script>

<span class="flex items-center gap-1">
	<input
		type="number"
		step="any"
		bind:value={value[0]}
		{disabled}
		aria-label="{def.label} X"
		class={FIELD_CLASS}
	/>
	<span class="text-xs text-gray-500 dark:text-gray-400">×</span>
	<input
		type="number"
		step="any"
		bind:value={value[1]}
		{disabled}
		aria-label="{def.label} Y"
		class={FIELD_CLASS}
	/>
</span>
