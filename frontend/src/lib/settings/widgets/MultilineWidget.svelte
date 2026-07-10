<script lang="ts">
	// Éditeur multiligne des G-code personnalisés (coString/coStrings « *gcode* »),
	// police à chasse fixe pour lire les commandes.
	import { FIELD_CLASS, type WidgetProps } from './types';

	let { def, value = $bindable(), disabled = false }: WidgetProps<string> = $props();
</script>

<textarea
	bind:value
	{disabled}
	rows="6"
	spellcheck="false"
	aria-label={def.label}
	class="{FIELD_CLASS} font-mono"></textarea>
