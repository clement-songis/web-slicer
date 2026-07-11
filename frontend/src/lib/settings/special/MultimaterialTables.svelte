<script lang="ts">
	// Page « Multimaterial » du filament (T044, Annexe B) : tables de purge et de
	// changement d'outil, rendues en lignes d'option groupées.
	import { PARAMS } from '../../../generated/params';
	import OptionLine from '../OptionLine.svelte';
	import { FILAMENT_MULTIMATERIAL } from './groups';

	interface Props {
		/** Valeurs par clé (liable). */
		values?: Record<string, unknown>;
	}

	let { values = $bindable({}) }: Props = $props();
</script>

<div class="flex flex-col gap-4">
	{#each FILAMENT_MULTIMATERIAL as group (group.title)}
		<section>
			<h3 class="mb-1 text-xs font-semibold tracking-wide text-content-subtle uppercase">
				{group.title}
			</h3>
			<div class="divide-y divide-border">
				{#each group.options as key (key)}
					<OptionLine def={PARAMS[key]} bind:value={values[key]} />
				{/each}
			</div>
		</section>
	{/each}
</div>
