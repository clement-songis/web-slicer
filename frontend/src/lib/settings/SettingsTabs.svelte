<script lang="ts">
	// Rendu générique des onglets de réglages (T042, cadré par type en T099) à
	// partir de `ui-layout.ts`. Reproduit l'organisation OrcaSlicer (pages →
	// sections → options) mais **cadré par type de preset** (`kind`) : une instance
	// n'affiche que les pages de son type (process / filament / machine), au lieu
	// du vidage à plat des 21 pages. Ajoute le sélecteur de mode et une recherche.
	import { PARAMS } from '../../generated/params';
	import { UI_LAYOUT, type PresetKind } from '../../generated/ui-layout';
	import { filterLayout, type DisplayMode } from './filter';
	import { t } from '$lib/i18n';
	import OptionLine from './OptionLine.svelte';

	interface Props {
		/** Type de preset affiché (cadre les onglets). */
		kind?: PresetKind;
		/** Mode d'affichage courant (liable). */
		mode?: DisplayMode;
		/** Valeurs effectives par clé de paramètre (liable). */
		values?: Record<string, unknown>;
	}

	let { kind = 'process', mode = $bindable('simple'), values = $bindable({}) }: Props = $props();

	let query = $state('');

	const MODES: DisplayMode[] = ['simple', 'advanced', 'expert'];

	// Pages du type demandé uniquement (cadrage T099), puis filtre mode/recherche.
	const scoped = $derived(UI_LAYOUT.filter((page) => page.kind === kind));
	const pages = $derived(filterLayout(scoped, mode, query));

	// Page active suivie par position : les titres de page ne sont pas uniques
	// entre types (« Notes », « Multimaterial »), mais le cadrage par type limite
	// les collisions ; on repère l'onglet par son index et on se replie sur 0.
	let activeIndex = $state(0);
	const activePage = $derived(pages[activeIndex] ?? pages[0]);
</script>

<div class="flex flex-col gap-3">
	<div class="flex flex-col gap-2">
		<div class="flex flex-wrap gap-1 rounded border border-border-strong p-0.5">
			{#each MODES as m (m)}
				<button
					type="button"
					onclick={() => (mode = m)}
					class="flex-1 rounded px-2 py-1 text-xs whitespace-nowrap capitalize {mode === m
						? 'bg-primary text-primary-content'
						: 'text-content-muted hover:bg-overlay'}"
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
			class="w-full min-w-0 rounded border border-border-strong bg-surface-raised px-3 py-1 text-sm text-content"
		/>
	</div>

	{#if pages.length === 0}
		<p class="py-6 text-center text-sm text-content-subtle">Aucun paramètre ne correspond.</p>
	{:else}
		<div class="flex flex-wrap gap-x-1 gap-y-0.5 border-b border-border">
			{#each pages as page, i (i)}
				<button
					type="button"
					onclick={() => (activeIndex = i)}
					class="rounded-t px-2 py-1 text-xs whitespace-nowrap {activeIndex === i
						? 'border-b-2 border-primary font-medium text-primary'
						: 'text-content-muted hover:text-content'}"
				>
					{$t(page.title)}
				</button>
			{/each}
		</div>

		{#if activePage}
			<div class="flex flex-col gap-4">
				{#each activePage.sections as section, si (si)}
					<section>
						{#if section.title}
							<h3 class="mb-1 text-xs font-semibold tracking-wide text-content-subtle uppercase">
								{$t(section.title)}
							</h3>
						{/if}
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
