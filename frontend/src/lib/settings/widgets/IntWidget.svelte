<script lang="ts">
	import { t } from '$lib/i18n';
	// Entier (coInt*), bornes min/max du registre appliquées.
	import { FIELD_CLASS, type WidgetProps } from './types';

	let { def, value = $bindable(), disabled = false, index }: WidgetProps<number> = $props();
</script>

<span class="flex items-center gap-1">
	<input
		type="number"
		step="1"
		min={def.min ?? undefined}
		max={def.max ?? undefined}
		bind:value
		data-index={index}
		{disabled}
		aria-label={$t(def.label)}
		class={FIELD_CLASS}
	/>
	{#if def.sidetext}
		<span class="text-xs whitespace-nowrap text-content-subtle">{$t(def.sidetext)}</span>
	{/if}
</span>
