<script lang="ts">
	// FloatOrPercent (coFloatOrPercent*) : valeur absolue ou relative, éditée en
	// texte (« 0.4 » ou « 40% ») — le suffixe « % » bascule l'interprétation,
	// rapportée à `ratioOver` (parité Orca).
	import { FIELD_CLASS, type WidgetProps } from './types';

	let { def, value = $bindable(), disabled = false }: WidgetProps<string> = $props();
</script>

<span class="flex items-center gap-1">
	<input
		type="text"
		inputmode="decimal"
		bind:value
		{disabled}
		aria-label={def.label}
		placeholder="0.4 ou 40%"
		class={FIELD_CLASS}
	/>
	{#if def.sidetext}
		<span class="text-xs text-gray-500 dark:text-gray-400">{def.sidetext}</span>
	{/if}
</span>
