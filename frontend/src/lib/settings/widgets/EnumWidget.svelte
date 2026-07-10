<script lang="ts">
	// Sélecteur d'enum (coEnum* et enums ouverts). Les étiquettes lisibles
	// (`enumLabels`) doublent les valeurs config (`enumValues`) quand présentes.
	import { FIELD_CLASS, type WidgetProps } from './types';

	let { def, value = $bindable(), disabled = false }: WidgetProps<string> = $props();

	const options = $derived(
		def.enumValues.map((v, i) => ({ value: v, label: def.enumLabels[i] ?? v }))
	);
</script>

<select bind:value {disabled} aria-label={def.label} class={FIELD_CLASS}>
	{#each options as opt (opt.value)}
		<option value={opt.value}>{opt.label}</option>
	{/each}
</select>
