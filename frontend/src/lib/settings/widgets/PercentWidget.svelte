<script lang="ts">
	import { t } from '$lib/i18n';
	// Pourcentage (coPercent*) : saisie « 60 » ou « 60% », comme dans Orca.
	import { FIELD_CLASS, percentText, type WidgetProps, unitText } from './types';

	let {
		def,
		value = $bindable(),
		disabled = false,
		index
	}: WidgetProps<number | string> = $props();

	// `/resolved` porte un pourcentage pur en nombre (15) ; Orca l'affiche « 15% ».
	// Affichage seul : la valeur liée n'est réécrite qu'à la saisie.
	const shown = $derived(percentText(value));
</script>

<span class="flex items-center gap-1">
	<input
		type="text"
		inputmode="decimal"
		value={shown}
		oninput={(e) => (value = e.currentTarget.value)}
		data-index={index}
		{disabled}
		aria-label={$t(def.label)}
		class={FIELD_CLASS}
	/>
	<span class="text-xs whitespace-nowrap text-content-subtle"
		>{unitText(def.sidetext) ? $t(unitText(def.sidetext)) : '%'}</span
	>
</span>
