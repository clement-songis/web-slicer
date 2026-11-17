<script lang="ts">
	import { untrack } from 'svelte';
	// Rendu générique des onglets de réglages (T042, cadré par type en T099) à
	// partir de `ui-layout.ts`. Reproduit l'organisation OrcaSlicer (pages →
	// sections → options) mais **cadré par type de preset** (`kind`) : une instance
	// n'affiche que les pages de son type (process / filament / machine), au lieu
	// du vidage à plat des 21 pages. Ajoute le sélecteur de mode et une recherche.
	import { SvelteMap } from 'svelte/reactivity';
	import { PARAMS } from '../../generated/params';
	import {
		UI_LAYOUT,
		type PresetKind,
		type UiPage,
		type UiSection
	} from '../../generated/ui-layout';
	import { filterLayout, type DisplayMode } from './filter';
	import { t, locale } from '$lib/i18n';
	import { loadSettingsTranslations } from './i18n';
	import OptionLine from './OptionLine.svelte';
	import { resolveDynamicOptions } from './dynamic-options';

	// Layout généré + lignes dynamiques Orca résolues (notes, vignettes).
	const LAYOUT = resolveDynamicOptions(UI_LAYOUT);

	interface Props {
		/** Type de preset affiché (cadre les onglets). */
		kind?: PresetKind;
		/** Mode d'affichage courant (liable). */
		mode?: DisplayMode;
		/** Valeurs effectives par clé de paramètre (liable). */
		values?: Record<string, unknown>;
		/** Valeurs pas encore chargées : la structure et les champs restent visibles,
		 *  mais un fieldset désactivé interdit toute édition d'un état transitoire. */
		loading?: boolean;
		/** Pages des autres presets liées dans la même modale. Chaque section garde
		 *  son type d'origine pour éditer le bon brouillon (dépendances inter-presets). */
		valuesByKind?: Record<PresetKind, Record<string, unknown>>;
		/** Onglet ouvert à l'affichage (titre de page Orca, ex. « Multimaterial »). */
		initialPage?: string;
		/** Valeurs enregistrées / système par type (marqueurs et retours par option). */
		savedByKind?: Partial<Record<PresetKind, Record<string, unknown> | null>>;
		systemByKind?: Partial<Record<PresetKind, Record<string, unknown> | null>>;
	}

	type LinkedSection = UiSection & { kind: PresetKind };
	type LinkedPage = Omit<UiPage, 'sections'> & { sections: LinkedSection[] };

	let {
		kind = 'process',
		mode = $bindable('simple'),
		values = $bindable({}),
		loading = false,
		valuesByKind,
		initialPage,
		savedByKind,
		systemByKind
	}: Props = $props();

	let query = $state('');

	// Libellés traduits chargés à la demande (hors chunk initial) ; l'affichage
	// anglais reste utilisable pendant le chargement ou en cas d'échec.
	$effect(() => {
		loadSettingsTranslations($locale).catch(() => {});
	});

	const MODES: DisplayMode[] = ['simple', 'advanced', 'expert', 'develop'];
	/** Libellés FR des modes (les identifiants restent ceux d'Orca, cf. data-testid). */
	const MODE_LABELS: Record<DisplayMode, string> = {
		simple: 'Simple',
		advanced: 'Avancé',
		expert: 'Expert',
		develop: 'Développeur'
	};

	/** Slug stable du contrat UI ; la page dynamique mono-extrudeur est « Extruder 1 ». */
	export function pageTitle(title: string): string {
		return title === 'page_name' || title === 'Extruder' ? 'Extruder 1' : title;
	}
	export function pageSlug(title: string): string {
		return pageTitle(title)
			.toLowerCase()
			.replace(/[^a-z0-9]+/g, '-')
			.replace(/^-|-$/g, '');
	}

	// Pages hors dialogue de preset chez Orca : « Frequent » (TabPrintModel,
	// réglages d'objet) et « Plate Settings » (TabPrintPlate, réglages du
	// plateau) ont leurs propres éditeurs (ObjectSettingsPanel, PlateSettings).
	// TabPrint::build (Tab.cpp) ne crée que Quality, Strength, Speed, Support,
	// Multimaterial, Others (+ Dependencies).
	const NON_PRESET_PAGES = new Set(['Frequent', 'Plate Settings']);
	const PRESET_LAYOUT = LAYOUT.filter((page) => !NON_PRESET_PAGES.has(page.title));

	/** Libellé du type de preset d'une page ou section liée. */
	const KIND_LABELS: Record<PresetKind, string> = {
		process: 'Impression',
		filament: 'Filament',
		machine: 'Imprimante'
	};

	// Pages du type demandé seul, ou pages liées (dépendances inter-presets :
	// une option du process grisée selon un réglage machine, etc.). Les pages
	// liées restent atteignables mais NE SE MÊLENT PLUS aux onglets du type :
	// onglets du type d'abord, puis un groupe « Réglages liés » par type, et
	// dans une page fusionnée par titre (« Multimaterial »), les sections du
	// type d'abord puis celles des autres types, étiquetées.
	const scoped = $derived.by((): LinkedPage[] => {
		if (!valuesByKind) {
			return PRESET_LAYOUT.filter((page) => page.kind === kind).map((page) => ({
				...page,
				sections: page.sections.map((section) => ({ ...section, kind: page.kind }))
			}));
		}
		const merged = new SvelteMap<string, LinkedPage>();
		const ordered = [
			...PRESET_LAYOUT.filter((page) => page.kind === kind),
			...PRESET_LAYOUT.filter((page) => page.kind !== kind)
		];
		for (const page of ordered) {
			const sections = page.sections.map((section) => ({ ...section, kind: page.kind }));
			const existing = merged.get(page.title);
			if (existing) existing.sections.push(...sections);
			else merged.set(page.title, { ...page, sections });
		}
		return [...merged.values()];
	});
	// Hors recherche, tous les onglets du type restent présents quel que soit le
	// mode (sélecteur stable) ; seules leurs options sont filtrées.
	const pages = $derived(
		filterLayout(scoped, mode, query, query.trim().length === 0) as LinkedPage[]
	);

	// Page active suivie par position : les titres de page ne sont pas uniques
	// entre types (« Notes », « Multimaterial »), mais le cadrage par type limite
	// les collisions ; on repère l'onglet par son index et on se replie sur 0.
	let activeSlug = $state(untrack(() => (initialPage ? pageSlug(initialPage) : '')));
	// Pendant une recherche, tous les onglets correspondants sont affichés tant
	// que l'utilisateur n'a pas choisi un onglet ; nouvelle saisie → tout revoir.
	let pickedDuringSearch = $state(false);
	let lastQuery = '';
	$effect(() => {
		if (query !== lastQuery) {
			lastQuery = query;
			pickedDuringSearch = false;
		}
	});
	const searchAll = $derived(query.trim().length > 0 && !pickedDuringSearch);
	// Résultats de recherche : onglets classés par nombre d'options dont le
	// LIBELLÉ (traduit) contient la recherche — « remplissage » met « Solidité »
	// (Densité de remplissage…) avant les correspondances d'infobulle seules.
	const foldText = (v: string) =>
		v
			.normalize('NFD')
			.replace(/[\u0300-\u036f]/g, '')
			.toLowerCase();
	const rankedPages = $derived.by(() => {
		const q = foldText(query.trim());
		if (!q) return pages;
		const score = (page: LinkedPage) =>
			page.sections.reduce(
				(n, section) =>
					n +
					section.options.filter((o) => {
						const def = typeof o === 'string' ? PARAMS[o] : undefined;
						return (
							!!def && (foldText($t(def.label)).includes(q) || foldText(def.label).includes(q))
						);
					}).length,
				0
			);
		return pages
			.map((page, i) => ({ page, i, s: score(page) }))
			.sort((x, y) => y.s - x.s || x.i - y.i)
			.map((x) => x.page);
	});
	/** Onglets du type édité, puis pages liées groupées par type d'origine. */
	const ownPages = $derived(pages.filter((page) => page.kind === kind));
	const linkedGroups = $derived(
		(['process', 'filament', 'machine'] as PresetKind[])
			.filter((k) => k !== kind)
			.map((k) => ({ kind: k, pages: pages.filter((page) => page.kind === k) }))
			.filter((group) => group.pages.length > 0)
	);
	const activePage = $derived(
		pages.find((page) => pageSlug(page.title) === activeSlug) ?? pages[0]
	);
</script>

<div class="flex flex-col gap-3">
	<div class="flex flex-col gap-2">
		<div class="ws-seg flex w-full flex-wrap" role="group" aria-label="Mode d'affichage">
			{#each MODES as m (m)}
				<button
					type="button"
					data-testid={`settings-mode-${m}`}
					onclick={() => (mode = m)}
					aria-pressed={mode === m}
					class="ws-seg-item flex-1 {mode === m ? 'ws-seg-on' : ''}"
				>
					{MODE_LABELS[m]}
				</button>
			{/each}
		</div>
		<input
			type="search"
			bind:value={query}
			placeholder="Rechercher un paramètre…"
			aria-label="Rechercher un paramètre"
			class="ws-field w-full px-3"
		/>
	</div>

	{#snippet pageSections(sections: LinkedPage['sections'])}
		{#each sections as section, si (si)}
			<section>
				{#if section.title || section.kind !== kind}
					<h3 class="ws-section-title mb-1 flex items-center gap-2">
						{section.title ? $t(section.title) : ''}
						{#if section.kind !== kind}
							<span class="ws-badge normal-case">{KIND_LABELS[section.kind]}</span>
						{/if}
					</h3>
				{/if}
				<div class="divide-y divide-border">
					{#each section.options as option, oi (typeof option === 'string' ? option : `dyn:${oi}`)}
						{#if typeof option === 'string' && PARAMS[option]}
							{#if valuesByKind}
								<OptionLine
									def={PARAMS[option]}
									bind:value={valuesByKind[section.kind][option]}
									saved={savedByKind?.[section.kind] ?? null}
									system={systemByKind?.[section.kind] ?? null}
								/>
							{:else}
								<OptionLine
									def={PARAMS[option]}
									bind:value={values[option]}
									saved={savedByKind?.[kind] ?? null}
									system={systemByKind?.[kind] ?? null}
								/>
							{/if}
						{/if}
					{/each}
				</div>
			</section>
		{/each}
	{/snippet}

	{#if pages.length === 0}
		<p class="py-6 text-center text-sm text-content-subtle">Aucun paramètre ne correspond.</p>
	{:else}
		{#snippet tabButton(page: LinkedPage)}
			<button
				type="button"
				data-testid={`settings-tab-${pageSlug(page.title)}`}
				role="tab"
				aria-selected={!searchAll && activePage === page}
				onclick={() => {
					activeSlug = pageSlug(page.title);
					pickedDuringSearch = true;
				}}
				class="ws-tab {!searchAll && activePage === page ? 'ws-tab-on' : ''}"
			>
				{$t(pageTitle(page.title))}
			</button>
		{/snippet}
		<div class="flex flex-col gap-1 border-b border-border">
			<div
				class="flex flex-wrap gap-x-1 gap-y-0.5"
				role="tablist"
				aria-label="Onglets {KIND_LABELS[kind]}"
			>
				{#each ownPages as page (pageSlug(page.title))}
					{@render tabButton(page)}
				{/each}
			</div>
			{#each linkedGroups as group (group.kind)}
				<div
					class="flex flex-wrap items-center gap-x-1 gap-y-0.5 border-t border-dashed border-border pt-1"
					role="tablist"
					aria-label="Réglages liés : {KIND_LABELS[group.kind]}"
					data-linked-kind={group.kind}
				>
					<span class="ws-section-title mr-1 text-2xs normal-case">
						Lié · {KIND_LABELS[group.kind]}
					</span>
					{#each group.pages as page (pageSlug(page.title))}
						{@render tabButton(page)}
					{/each}
				</div>
			{/each}
		</div>

		{#if activePage}
			{#if loading}
				<p
					class="text-center text-sm text-content-subtle"
					data-testid="settings-loading"
					role="status"
				>
					Chargement des réglages…
				</p>
			{/if}
			<!-- Ne pas démonter les options pendant `/resolved` : leur présence est
			     indépendante des valeurs. `disabled` empêche néanmoins de modifier un
			     état vide ou celui du preset précédent avant la réponse. -->
			<fieldset disabled={loading} aria-busy={loading} class="min-w-0">
				<div class="flex flex-col gap-4">
					{#if activePage.sections.length === 0}
						<p
							class="py-6 text-center text-sm text-content-subtle"
							data-testid="settings-page-empty"
						>
							Aucun paramètre de cet onglet à ce niveau d'affichage.
						</p>
					{/if}
					{#if searchAll}
						<!-- Recherche : résultats de TOUS les onglets (comme la liste de
						     recherche d'Orca), titrés par onglet ; un clic sur un onglet
						     restreint les résultats à cet onglet. -->
						{#each rankedPages as page (pageSlug(page.title) + page.kind)}
							<div class="flex flex-col gap-2" data-search-page={pageSlug(page.title)}>
								<h2 class="text-xs font-semibold text-primary">
									{$t(pageTitle(page.title))}{#if page.kind !== kind}
										<span class="ws-badge ml-1 normal-case">{KIND_LABELS[page.kind]}</span>{/if}
								</h2>
								{@render pageSections(page.sections)}
							</div>
						{/each}
					{:else}
						{@render pageSections(activePage.sections)}
					{/if}
				</div>
			</fieldset>
		{/if}
	{/if}
</div>
