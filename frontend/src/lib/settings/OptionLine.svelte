<script lang="ts">
	// Une ligne d'option (T042) : libellé + infobulle à gauche, widget d'édition
	// à droite (résolu par type de registre, T041). L'unité (sidetext) est rendue
	// par le widget lui-même.
	import type { ParamDef } from '../../generated/params';
	import { widgetFor } from './widgets';

	interface Props {
		def: ParamDef;
		/** Valeur courante du paramètre (liaison bidirectionnelle). */
		value: unknown;
		/** Champ verrouillé (preset système, valeur héritée non surchargée…). */
		disabled?: boolean;
	}

	let { def, value = $bindable(), disabled = false }: Props = $props();

	const Widget = $derived(widgetFor(def));
</script>

<div class="flex items-center gap-3 py-1">
	<!-- Le contrôle porte son propre `aria-label` (def.label) ; ce texte est
	     purement visuel, d'où un span plutôt qu'un <label>. -->
	<span class="flex-1 text-sm text-gray-700 dark:text-gray-300" title={def.tooltip}>
		{def.label}
		{#if def.tooltip}
			<span class="ml-1 cursor-help text-gray-400" aria-hidden="true">ⓘ</span>
		{/if}
	</span>
	<div class="w-56 shrink-0">
		<Widget {def} bind:value {disabled} />
	</div>
</div>
