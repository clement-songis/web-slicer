<script lang="ts">
	import { t } from '#lib/i18n';
	// Sélecteur de couleur (gui_type « color »), valeur hexadécimale « #RRGGBB ».
	import { FIELD_CLASS, type WidgetProps } from './types';

	let { def, value = $bindable(), disabled = false, index }: WidgetProps<string> = $props();

	// Orca stocke le texte tel quel (une couleur filament peut être une valeur
	// libre) : le champ texte est le contrôle de référence (`data-index`). Le
	// sélecteur natif n'accepte que « #rrggbb » : il reflète la valeur si elle
	// est valide, sans jamais la réécrire de lui-même (pas de `bind:`).
	const HEX = /^#[0-9a-fA-F]{6}$/;
	const pickerValue = $derived(
		typeof value === 'string' && HEX.test(value.slice(0, 7)) ? value.slice(0, 7) : '#000000'
	);
</script>

<span class="flex items-center gap-2">
	<input
		type="color"
		value={pickerValue}
		oninput={(e) => (value = e.currentTarget.value)}
		{disabled}
		aria-label={$t('{label} (picker)', { label: $t(def.label) })}
		class="h-8 w-10 rounded border border-border-strong"
	/>
	<input
		type="text"
		bind:value
		data-index={index}
		{disabled}
		aria-label={$t(def.label)}
		class={FIELD_CLASS}
	/>
</span>
