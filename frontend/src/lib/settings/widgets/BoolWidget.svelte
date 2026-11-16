<script lang="ts">
	import { t } from '$lib/i18n';
	// Interrupteur booléen (coBool*).
	import type { WidgetProps } from './types';

	let { def, value = $bindable(), disabled = false, index }: WidgetProps<boolean> = $props();
</script>

<input
	type="checkbox"
	bind:checked={value}
	data-index={index}
	{disabled}
	aria-label={$t(def.label)}
	class="h-4 w-4 rounded border-border-strong"
/>
