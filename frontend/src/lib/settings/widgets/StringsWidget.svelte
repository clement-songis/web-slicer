<script lang="ts">
	// Liste de chaînes (coStrings non G-code) : une entrée par ligne.
	import { FIELD_CLASS, type WidgetProps } from './types';

	let { def, value = $bindable(), disabled = false }: WidgetProps<string[]> = $props();

	const text = $derived((value ?? []).join('\n'));

	function onInput(event: Event) {
		const raw = (event.currentTarget as HTMLTextAreaElement).value;
		value = raw.length ? raw.split('\n') : [];
	}
</script>

<textarea
	value={text}
	oninput={onInput}
	{disabled}
	rows="3"
	aria-label={def.label}
	class={FIELD_CLASS}></textarea>
