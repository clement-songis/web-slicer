<script lang="ts">
	import { t } from '$lib/i18n';
	import { unitLabel } from './unit-label';
	// FloatOrPercent (coFloatOrPercent*) : valeur absolue ou relative, éditée en
	// texte (« 0.4 » ou « 40% ») — le suffixe « % » bascule l'interprétation,
	// rapportée à `ratioOver` (parité Orca).
	import {
		FIELD_CLASS,
		decimalSeparator,
		fromDisplayDecimal,
		toDisplayDecimal,
		unitText,
		type WidgetProps
	} from './types';

	let { def, value = $bindable(), disabled = false, index }: WidgetProps<string> = $props();
	// Séparateur décimal de la langue (comme les champs numériques natifs).
	const sep = decimalSeparator();
	const shown = $derived(toDisplayDecimal(value, sep));
</script>

<span class="flex items-center gap-1">
	<input
		type="text"
		inputmode="decimal"
		value={shown}
		oninput={(e) => (value = fromDisplayDecimal(e.currentTarget.value))}
		data-index={index}
		{disabled}
		aria-label={$t(def.label)}
		placeholder={sep === ',' ? '0,4 ou 40%' : '0.4 ou 40%'}
		class={FIELD_CLASS}
	/>
	{#if unitText(def.sidetext)}
		<span class="text-xs whitespace-nowrap text-content-subtle">{$unitLabel(def.sidetext)}</span>
	{/if}
</span>
