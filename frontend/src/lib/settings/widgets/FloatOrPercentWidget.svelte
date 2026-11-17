<script lang="ts">
	import { t } from '$lib/i18n';
	// FloatOrPercent (coFloatOrPercent*) : valeur absolue ou relative, éditée en
	// texte (« 0.4 » ou « 40% ») — le suffixe « % » bascule l'interprétation,
	// rapportée à `ratioOver` (parité Orca).
	import { FIELD_CLASS, unitText, type WidgetProps } from './types';

	let { def, value = $bindable(), disabled = false, index }: WidgetProps<string> = $props();
</script>

<span class="flex items-center gap-1">
	<input
		type="text"
		inputmode="decimal"
		bind:value
		data-index={index}
		{disabled}
		aria-label={$t(def.label)}
		placeholder="0.4 ou 40%"
		class={FIELD_CLASS}
	/>
	{#if unitText(def.sidetext)}
		<span class="text-xs whitespace-nowrap text-content-subtle">{$t(unitText(def.sidetext))}</span>
	{/if}
</span>
