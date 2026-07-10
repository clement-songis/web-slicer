<script lang="ts">
	// Chaîne simple (coString non G-code).
	import { FIELD_CLASS, type WidgetProps } from './types';

	let { def, value = $bindable(), disabled = false }: WidgetProps<string> = $props();
</script>

<input type="text" bind:value {disabled} aria-label={def.label} class={FIELD_CLASS} />
