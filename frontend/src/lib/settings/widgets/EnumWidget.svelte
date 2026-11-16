<script lang="ts">
	// Sélecteur d'enum (coEnum* et enums ouverts). Les étiquettes lisibles
	// (`enumLabels`) doublent les valeurs config (`enumValues`) quand présentes.
	import { FIELD_CLASS, type WidgetProps } from './types';
	import { t } from '$lib/i18n';

	import { isOpenEnum } from './types';

	let {
		def,
		value = $bindable(),
		disabled = false,
		index
	}: WidgetProps<string | number> = $props();

	const options = $derived(
		def.enumValues.map((v, i) => ({ value: v, label: $t(def.enumLabels[i] ?? v) }))
	);
	// Enum ouvert Orca (`*_enum_open`, `select_open` : combo éditable, ex. couches
	// d'interface 0–3 « ou autre ») ou liste de valeurs absente du registre : un
	// `<select>` ne pourrait ni afficher la valeur du preset (2 ≠ « 2 », valeur
	// hors liste) ni la saisir. Champ texte + suggestions, valeur telle quelle.
	const open = $derived(isOpenEnum(def));
	const listId = $derived(`enum-${def.key}-${index ?? 0}`);
</script>

{#if open}
	<input
		type="text"
		list={options.length ? listId : undefined}
		value={value === undefined || value === null ? '' : String(value)}
		oninput={(e) => (value = e.currentTarget.value)}
		{disabled}
		data-index={index}
		aria-label={$t(def.label)}
		class={FIELD_CLASS}
	/>
	{#if options.length}
		<datalist id={listId}>
			{#each options as opt (opt.value)}
				<option value={opt.value}>{opt.label}</option>
			{/each}
		</datalist>
	{/if}
{:else}
	<select bind:value {disabled} data-index={index} aria-label={$t(def.label)} class={FIELD_CLASS}>
		{#each options as opt (opt.value)}
			<option value={opt.value}>{opt.label}</option>
		{/each}
	</select>
{/if}
