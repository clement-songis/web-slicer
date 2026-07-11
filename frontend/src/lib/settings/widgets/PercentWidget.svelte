<script lang="ts">
	// Pourcentage (coPercent*) : saisie numérique, unité « % » forcée.
	import { FIELD_CLASS, type WidgetProps } from './types';

	let { def, value = $bindable(), disabled = false }: WidgetProps<number> = $props();
</script>

<span class="flex items-center gap-1">
	<input
		type="number"
		step="any"
		min={def.min ?? 0}
		max={def.max ?? undefined}
		bind:value
		{disabled}
		aria-label={def.label}
		class={FIELD_CLASS}
	/>
	<span class="text-xs text-content-subtle">{def.sidetext || '%'}</span>
</span>
