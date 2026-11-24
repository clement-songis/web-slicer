<script lang="ts">
	import { t } from '#lib/i18n';
	// Chaîne simple (coString non G-code).
	import { FIELD_CLASS, type WidgetProps } from './types';

	let { def, value = $bindable(), disabled = false, index }: WidgetProps<string> = $props();
</script>

<input
	type="text"
	bind:value
	{disabled}
	data-index={index}
	aria-label={$t(def.label)}
	class={FIELD_CLASS}
/>
