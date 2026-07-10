<script lang="ts">
	// Page « Setting Overrides » du filament (T044, Annexe B). Chaque option porte
	// une case « activer » : décochée = N/A (le filament n'écrase pas la valeur de
	// l'imprimante/process, valeur `null`) ; cochée = valeur éditable.
	import { PARAMS } from '../../../generated/params';
	import OptionLine from '../OptionLine.svelte';
	import { defaultFor, FILAMENT_OVERRIDES, isOverrideActive } from './groups';

	interface Props {
		/** Valeurs par clé (liable) — `null` signifie N/A. */
		values?: Record<string, unknown>;
	}

	let { values = $bindable({}) }: Props = $props();

	function toggle(key: string, on: boolean) {
		values[key] = on ? defaultFor(PARAMS[key]) : null;
	}
</script>

<div class="flex flex-col gap-4">
	{#each FILAMENT_OVERRIDES as group (group.title)}
		<section>
			<h3
				class="mb-1 text-xs font-semibold tracking-wide text-gray-500 uppercase dark:text-gray-400"
			>
				{group.title}
			</h3>
			<div class="divide-y divide-gray-100 dark:divide-gray-800">
				{#each group.options as key (key)}
					{@const active = isOverrideActive(values[key])}
					<div class="flex items-center gap-3 py-1">
						<input
							type="checkbox"
							checked={active}
							onchange={(e) => toggle(key, e.currentTarget.checked)}
							aria-label="Activer {PARAMS[key].label}"
							class="h-4 w-4 shrink-0 rounded border-gray-300 dark:border-gray-600"
						/>
						{#if active}
							<div class="flex-1">
								<OptionLine def={PARAMS[key]} bind:value={values[key]} />
							</div>
						{:else}
							<span class="flex-1 text-sm text-gray-700 dark:text-gray-300">
								{PARAMS[key].label}
								<span class="ml-2 text-xs text-gray-400 italic">N/A</span>
							</span>
						{/if}
					</div>
				{/each}
			</div>
		</section>
	{/each}
</div>
