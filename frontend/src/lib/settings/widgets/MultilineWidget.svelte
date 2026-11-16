<script lang="ts">
	import { t } from '$lib/i18n';
	// Éditeur multiligne des G-code personnalisés (coString/coStrings « *gcode* »),
	// police à chasse fixe pour lire les commandes.
	import { FIELD_CLASS, type WidgetProps } from './types';

	let {
		def,
		value = $bindable(),
		disabled = false,
		index
	}: WidgetProps<string | string[]> = $props();

	const text = $derived(Array.isArray(value) ? (value[0] ?? '') : (value ?? ''));

	function onInput(event: Event) {
		const raw = (event.currentTarget as HTMLTextAreaElement).value;
		value = def.type === 'coStrings' ? [raw] : raw;
	}
</script>

<textarea
	value={text}
	oninput={onInput}
	data-index={index}
	{disabled}
	rows="6"
	spellcheck="false"
	aria-label={$t(def.label)}
	class="{FIELD_CLASS} font-mono"></textarea>
