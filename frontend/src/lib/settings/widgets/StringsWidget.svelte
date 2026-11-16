<script lang="ts">
	import { t } from '$lib/i18n';
	// Liste de chaînes (coStrings non G-code) : une entrée par ligne.
	import { FIELD_CLASS, type WidgetProps } from './types';

	let { def, value = $bindable(), disabled = false, index }: WidgetProps<string[]> = $props();

	const text = $derived((value ?? []).join('\n'));

	function onInput(event: Event) {
		const raw = (event.currentTarget as HTMLTextAreaElement).value;
		value = raw.length ? raw.split('\n') : [];
	}
</script>

<textarea
	value={text}
	oninput={onInput}
	data-index={index}
	{disabled}
	rows="3"
	aria-label={$t(def.label)}
	class={FIELD_CLASS}></textarea>
