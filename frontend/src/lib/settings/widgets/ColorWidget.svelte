<script lang="ts">
	// Sélecteur de couleur (gui_type « color »), valeur hexadécimale « #RRGGBB ».
	import { FIELD_CLASS, type WidgetProps } from './types';

	let { def, value = $bindable(), disabled = false }: WidgetProps<string> = $props();
</script>

<span class="flex items-center gap-2">
	<input
		type="color"
		bind:value
		{disabled}
		aria-label={def.label}
		class="h-8 w-10 rounded border border-gray-300 dark:border-gray-600"
	/>
	<input type="text" bind:value {disabled} aria-label="{def.label} (hex)" class={FIELD_CLASS} />
</span>
