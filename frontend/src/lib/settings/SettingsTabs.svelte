<script lang="ts">
	// Rendu générique des onglets de réglages (T042) à partir de `ui-layout.ts`.
	// Reproduit l'organisation OrcaSlicer (pages → sections → options) ; ajoute
	// le sélecteur de mode (simple/advanced/expert) et une recherche additive.
	import { PARAMS } from '../../generated/params';
	import { UI_LAYOUT } from '../../generated/ui-layout';
	import { filterLayout, type DisplayMode } from './filter';
	import OptionLine from './OptionLine.svelte';

	interface Props {
		/** Mode d'affichage courant (liable). */
		mode?: DisplayMode;
		/** Valeurs effectives par clé de paramètre (liable). */
		values?: Record<string, unknown>;
	}

	let { mode = $bindable('simple'), values = $bindable({}) }: Props = $props();

	let query = $state('');

	const MODES: DisplayMode[] = ['simple', 'advanced', 'expert'];

	const pages = $derived(filterLayout(UI_LAYOUT, mode, query));

	// Page active suivie par position : les titres de page ne sont pas uniques
	// (OrcaSlicer répète « Notes », « Multimaterial »… entre catégories), donc
	// on repère l'onglet par son index et on se replie sur le premier visible.
	let activeIndex = $state(0);
	const activePage = $derived(pages[activeIndex] ?? pages[0]);
</script>

<div class="flex flex-col gap-3">
	<div class="flex items-center gap-3">
		<div class="inline-flex overflow-hidden rounded border border-border-strong">
			{#each MODES as m (m)}
				<button
					type="button"
					onclick={() => (mode = m)}
					class="px-3 py-1 text-sm capitalize {mode === m
						? 'bg-primary text-white'
						: 'bg-surface-raised text-content-muted'}"
				>
					{m}
				</button>
			{/each}
		</div>
		<input
			type="search"
			bind:value={query}
			placeholder="Rechercher un paramètre…"
			aria-label="Rechercher un paramètre"
			class="flex-1 rounded border border-border-strong px-3 py-1 text-sm bg-surface-raised text-content"
		/>
	</div>

	{#if pages.length === 0}
		<p class="py-6 text-center text-sm text-content-subtle">Aucun paramètre ne correspond.</p>
	{:else}
		<div class="flex flex-wrap gap-1 border-b border-border">
			{#each pages as page, i (i)}
				<button
					type="button"
					onclick={() => (activeIndex = i)}
					class="rounded-t px-3 py-1 text-sm {activeIndex === i
						? 'border-b-2 border-primary font-medium text-primary'
						: 'text-content-muted'}"
				>
					{page.title}
				</button>
			{/each}
		</div>

		{#if activePage}
			<div class="flex flex-col gap-4">
				{#each activePage.sections as section, si (si)}
					<section>
						<h3 class="mb-1 text-xs font-semibold tracking-wide text-content-subtle uppercase">
							{section.title}
						</h3>
						<div class="divide-y divide-border">
							{#each section.options as option, oi (typeof option === 'string' ? option : `dyn:${oi}`)}
								{#if typeof option === 'string' && PARAMS[option]}
									<OptionLine def={PARAMS[option]} bind:value={values[option]} />
								{:else}
									<p class="py-1 text-xs text-content-subtle italic">
										Options générées dynamiquement (par extrudeur)
									</p>
								{/if}
							{/each}
						</div>
					</section>
				{/each}
			</div>
		{/if}
	{/if}
</div>
