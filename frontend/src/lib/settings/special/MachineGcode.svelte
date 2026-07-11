<script lang="ts">
	// Page « Machine G-code » de l'imprimante (T044, Annexe B) : un éditeur
	// multiligne pleine largeur par bloc de G-code personnalisé. Le widget est
	// résolu par type (tous ces champs → MultilineWidget).
	import { PARAMS } from '../../../generated/params';
	import { widgetFor } from '../widgets';
	import { MACHINE_GCODE } from './groups';

	interface Props {
		/** Valeurs par clé (liable). */
		values?: Record<string, unknown>;
	}

	let { values = $bindable({}) }: Props = $props();
</script>

<div class="flex flex-col gap-4">
	{#each MACHINE_GCODE as group (group.title)}
		{@const key = group.options[0]}
		{@const Widget = widgetFor(PARAMS[key])}
		<section>
			<h3 class="mb-1 text-sm font-medium text-content-muted" title={PARAMS[key].tooltip}>
				{group.title}
			</h3>
			<Widget def={PARAMS[key]} bind:value={values[key]} />
		</section>
	{/each}
</div>
