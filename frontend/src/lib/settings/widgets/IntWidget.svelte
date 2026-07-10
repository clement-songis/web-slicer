<script lang="ts">
	// Entier (coInt*), bornes min/max du registre appliquées.
	import { FIELD_CLASS, type WidgetProps } from './types';

	let { def, value = $bindable(), disabled = false }: WidgetProps<number> = $props();
</script>

<span class="flex items-center gap-1">
	<input
		type="number"
		step="1"
		min={def.min ?? undefined}
		max={def.max ?? undefined}
		bind:value
		{disabled}
		aria-label={def.label}
		class={FIELD_CLASS}
	/>
	{#if def.sidetext}
		<span class="text-xs text-gray-500 dark:text-gray-400">{def.sidetext}</span>
	{/if}
</span>
