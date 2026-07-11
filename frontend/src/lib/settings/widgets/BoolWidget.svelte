<script lang="ts">
	// Interrupteur booléen (coBool*).
	import type { WidgetProps } from './types';

	let { def, value = $bindable(), disabled = false }: WidgetProps<boolean> = $props();
</script>

<input
	type="checkbox"
	bind:checked={value}
	{disabled}
	aria-label={def.label}
	class="h-4 w-4 rounded border-border-strong"
/>
