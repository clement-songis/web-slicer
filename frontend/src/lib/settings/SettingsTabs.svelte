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
	import { filterLayout, isEditorOnlyPage, MODE_RANK, type DisplayMode } from './filter';
	import { t, locale } from '$lib/i18n';
	import { loadSettingsTranslations } from './i18n';
	import OptionLine from './OptionLine.svelte';
	import { resolveDynamicOptions } from './dynamic-options';
	import { variantLabels } from './filament-variants';
	import { LINE_OPTION_LABELS, sectionLines, sectionRows } from './ui-lines';

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
		/** Option à montrer (lien « Régler » d'une notification de validation) :
		 *  onglet, mode suffisant, défilement et surbrillance (Orca activate_option). */
		focusKey?: string | null;
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
		systemByKind,
		focusKey = null
	}: Props = $props();
	let root = $state<HTMLDivElement | null>(null);

	// Phase 34 : variante de buse du filament (Orca TabFilament m_variant_combo,
	// masqué s'il n'y a qu'une variante) : les options par variante éditent la
	// valeur de la variante choisie.
	const variantSource = $derived(
		valuesByKind ? valuesByKind.filament : kind === 'filament' ? values : null
	);
	// Libellés traduits à l'appel : `$locale` pour les recalculer au changement de langue.
	const variants = $derived($locale ? variantLabels(variantSource?.filament_extruder_variant) : []);
	let variantIndex = $state(0);
	const activeVariant = $derived(variantIndex < variants.length ? variantIndex : 0);
	const variantFor = (sectionKind: PresetKind) =>
		sectionKind === 'filament' && variants.length > 1 ? activeVariant : undefined;

	let query = $state('');

	// Libellés traduits chargés à la demande (hors chunk initial) ; l'affichage
	// anglais reste utilisable pendant le chargement ou en cas d'échec.
	$effect(() => {
		loadSettingsTranslations($locale).catch(() => {});
	});

	const MODES: DisplayMode[] = ['simple', 'advanced', 'expert', 'develop'];
	/** Libellés des modes (clés i18n ; les identifiants restent ceux d'Orca, cf. data-testid). */
	const MODE_LABELS: Record<DisplayMode, string> = {
		simple: 'Simple',
		advanced: 'Advanced',
		expert: 'Expert',
		develop: 'Developer'
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
		process: 'Process||kind',
		filament: 'Filament',
		machine: 'Printer'
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
	// Comme Orca (Tab::update_visibility → Page::get_show), un onglet d'éditeurs
	// de texte (G-code, notes) sans option au mode courant est masqué (ex.
	// « G-code de la machine » en Simple) ; il réapparaît en Avancé. Les onglets
	// de paramètres restent, eux, présents à tous les modes (sélecteur stable :
	// « Vitesse » en Simple affiche une page vide avec un lien vers le mode
	// utile) ; en recherche, seuls les onglets ayant un résultat restent.
	const editorOnlyTitles = $derived(
		new Set(scoped.filter((page) => isEditorOnlyPage(page)).map((page) => page.title))
	);
	const pages = $derived.by(() => {
		const searching = query.trim().length > 0;
		return (filterLayout(scoped, mode, query, !searching) as LinkedPage[]).filter(
			(page) => page.sections.length > 0 || (!searching && !editorOnlyTitles.has(page.title))
		);
	});
	/** Premier mode (≥ courant) où l'onglet `slug` du type a des options. */
	function modeShowing(slug: string): DisplayMode | null {
		for (const m of MODES.slice(MODES.indexOf(mode))) {
			const shown = filterLayout(scoped, m, '', false) as LinkedPage[];
			if (shown.some((page) => page.kind === kind && pageSlug(page.title) === slug)) return m;
		}
		return null;
	}
	// Onglets du type masqués au mode courant : signalés (et atteignables d'un
	// clic) pour ne pas « perdre » le G-code machine en mode Simple.
	const hiddenPages = $derived.by(() => {
		if (query.trim()) return [];
		const shown = new Set(pages.filter((p) => p.kind === kind).map((p) => p.title));
		return (filterLayout(scoped, 'develop', '', false) as LinkedPage[]).filter(
			(p) => p.kind === kind && !shown.has(p.title)
		);
	});
	const revealMode = $derived(
		hiddenPages.length ? modeShowing(pageSlug(hiddenPages[0].title)) : null
	);
	// Ouverture ciblée sur un onglet masqué au mode courant : on passe au premier
	// mode qui l'affiche plutôt que de retomber sur le premier onglet.
	untrack(() => {
		if (!initialPage) return;
		const m = modeShowing(pageSlug(initialPage));
		if (m && m !== mode) mode = m;
	});

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
	/** Onglet stable vide au mode courant : premier mode supérieur qui le remplit. */
	const emptyPageMode = $derived.by((): DisplayMode | null => {
		if (!activePage || activePage.sections.length > 0) return null;
		const source = scoped.find((p) => p.title === activePage.title && p.kind === activePage.kind);
		if (!source) return null;
		for (const m of MODES.slice(MODES.indexOf(mode) + 1)) {
			if (filterLayout([source], m, '', false).length > 0) return m;
		}
		return null;
	});

	// Phase 29 : « Régler « option » » depuis une notification de validation
	// (Orca Sidebar::jump_to_option → Tab::activate_option) : onglet de l'option,
	// mode d'affichage relevé si besoin, recherche vidée, ligne centrée puis
	// surlignée quelques secondes.
	const FOCUS_CLASSES = ['rounded-md', 'ring-2', 'ring-primary', 'bg-primary/10'];
	let focusSeq = 0;
	$effect(() => {
		const key = focusKey;
		if (!key) return;
		untrack(() => {
			const def = PARAMS[key];
			if (def && MODE_RANK[def.mode] > MODE_RANK[mode]) mode = def.mode;
			query = '';
			const holds = (p: LinkedPage) => p.sections.some((s) => s.options.includes(key));
			const page = scoped.find((p) => p.kind === kind && holds(p)) ?? scoped.find(holds);
			if (page) {
				activeSlug = pageSlug(page.title);
				pickedDuringSearch = true;
			}
		});
		const seq = ++focusSeq;
		const timer = setTimeout(() => {
			if (seq !== focusSeq) return;
			const el = root?.querySelector<HTMLElement>('[data-param="' + CSS.escape(key) + '"]');
			if (!el) return;
			el.scrollIntoView({ block: 'center' });
			el.classList.add(...FOCUS_CLASSES);
			el.dataset.focused = 'true';
			el.querySelector<HTMLElement>('input, select, textarea, button')?.focus({
				preventScroll: true
			});
			setTimeout(() => {
				el.classList.remove(...FOCUS_CLASSES);
				delete el.dataset.focused;
			}, 2600);
		}, 120);
		return () => clearTimeout(timer);
	});
</script>

<div class="flex flex-col gap-3" bind:this={root}>
	<div class="flex flex-col gap-2">
		<div class="ws-seg flex w-full flex-wrap" role="group" aria-label={$t('Display mode')}>
			{#each MODES as m (m)}
				<button
					type="button"
					data-testid={`settings-mode-${m}`}
					onclick={() => (mode = m)}
					aria-pressed={mode === m}
					class="ws-seg-item flex-1 {mode === m ? 'ws-seg-on' : ''}"
				>
					{$t(MODE_LABELS[m])}
				</button>
			{/each}
		</div>
		<input
			type="search"
			bind:value={query}
			placeholder={$t('Search a setting…')}
			aria-label={$t('Search a setting')}
			class="ws-field w-full px-3"
		/>
		{#if variants.length > 1 && (kind === 'filament' || activePage?.kind === 'filament')}
			<div class="flex flex-col gap-1" data-testid="filament-variant-selector">
				<span class="text-xs text-content-muted">
					{$t(
						'Nozzle variant being edited — settings marked “per nozzle” have one value per variant.'
					)}
				</span>
				<div class="ws-seg flex w-full flex-wrap" role="group" aria-label={$t('Nozzle variant')}>
					{#each variants as label, i (i)}
						<button
							type="button"
							class="ws-seg-item flex-1 {activeVariant === i ? 'ws-seg-on' : ''}"
							aria-pressed={activeVariant === i}
							data-testid={`filament-variant-${i}`}
							onclick={() => (variantIndex = i)}
						>
							{label}
						</button>
					{/each}
				</div>
			</div>
		{/if}
	</div>

	{#snippet optionLine(section: LinkedSection, option: string, label?: string)}
		{#if valuesByKind}
			<OptionLine
				def={PARAMS[option]}
				{label}
				bind:value={
					() => valuesByKind?.[section.kind][option],
					(v) => {
						// Liaison par fonctions (audit 40 A5) : pas de bind: sur une
						// propriété non liée par le parent (ownership_invalid_binding).
						if (valuesByKind) valuesByKind[section.kind][option] = v;
					}
				}
				saved={savedByKind?.[section.kind] ?? null}
				system={systemByKind?.[section.kind] ?? null}
				variantIndex={variantFor(section.kind)}
			/>
		{:else}
			<OptionLine
				def={PARAMS[option]}
				{label}
				bind:value={values[option]}
				saved={savedByKind?.[kind] ?? null}
				system={systemByKind?.[kind] ?? null}
				variantIndex={variantFor(kind)}
			/>
		{/if}
	{/snippet}

	{#snippet pageSections(page: LinkedPage)}
		{#each page.sections as section, si (si)}
			<section>
				{#if section.title || section.kind !== kind}
					<h3 class="ws-section-title mb-1 flex items-center gap-2">
						{section.title ? $t(section.title) : ''}
						{#if section.kind !== kind}
							<span class="ws-badge normal-case">{$t(KIND_LABELS[section.kind])}</span>
						{/if}
					</h3>
				{/if}
				<div class="divide-y divide-border">
					<!-- Lignes composées d'Orca (UI_LINES) : libellé commun (« Nozzle »,
					     « Cool Plate »…) au-dessus de ses options (OptionsGroup::append_line). -->
					{#each sectionRows(section.options, sectionLines(section.kind, page.title, section.title), (key) => PARAMS[key]?.label) as row (row.kind === 'line' ? `line:${row.line.label}` : typeof row.option === 'string' ? row.option : `dyn:${row.index}`)}
						{#if row.kind === 'line'}
							{#if row.options.some((key) => PARAMS[key])}
								<div class="py-1" data-ui-line={row.line.label}>
									<div
										class="pt-1 text-sm font-medium text-content"
										data-testid="settings-line-label"
									>
										{$t(row.line.label)}
									</div>
									<div class="divide-y divide-border pl-4">
										{#each row.options.filter((key) => PARAMS[key]) as option (option)}
											{@render optionLine(section, option, LINE_OPTION_LABELS[option])}
										{/each}
									</div>
								</div>
							{/if}
						{:else if typeof row.option === 'string' && PARAMS[row.option]}
							{@render optionLine(section, row.option)}
						{/if}
					{/each}
				</div>
			</section>
		{/each}
	{/snippet}

	{#if pages.length === 0}
		<p class="py-6 text-center text-sm text-content-subtle">{$t('No matching setting.')}</p>
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
				aria-label={$t('{kind} tabs', { kind: $t(KIND_LABELS[kind]) })}
			>
				{#each ownPages as page (pageSlug(page.title))}
					{@render tabButton(page)}
				{/each}
				{#if hiddenPages.length && revealMode}
					<button
						type="button"
						class="ml-auto self-center rounded px-1.5 text-2xs text-content-muted hover:bg-overlay hover:text-content"
						data-testid="settings-hidden-pages"
						data-tip={$t('Tabs without settings in {mode} mode: {pages}', {
							mode: $t(MODE_LABELS[mode]),
							pages: hiddenPages.map((p) => $t(pageTitle(p.title))).join(', ')
						})}
						onclick={() => {
							activeSlug = pageSlug(hiddenPages[0].title);
							mode = revealMode;
						}}
					>
						{$t(hiddenPages.length > 1 ? '+{n} tabs in {mode} mode' : '+{n} tab in {mode} mode', {
							n: hiddenPages.length,
							mode: $t(MODE_LABELS[revealMode])
						})}
					</button>
				{/if}
			</div>
			{#each linkedGroups as group (group.kind)}
				<div
					class="flex flex-wrap items-center gap-x-1 gap-y-0.5 border-t border-dashed border-border pt-1"
					role="tablist"
					aria-label={$t('Linked settings: {kind}', { kind: $t(KIND_LABELS[group.kind]) })}
					data-linked-kind={group.kind}
				>
					<span class="ws-section-title mr-1 text-2xs normal-case">
						{$t('Linked · {kind}', { kind: $t(KIND_LABELS[group.kind]) })}
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
					{$t('Loading settings…')}
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
							{$t('No setting of this tab at this display level.')}
							{#if emptyPageMode}
								<button
									type="button"
									class="ml-1 text-primary underline-offset-2 hover:underline"
									data-testid="settings-page-reveal"
									onclick={() => {
										if (emptyPageMode) mode = emptyPageMode;
									}}
								>
									{$t('Show in {mode} mode', { mode: $t(MODE_LABELS[emptyPageMode]) })}
								</button>
							{/if}
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
										<span class="ws-badge ml-1 normal-case">{$t(KIND_LABELS[page.kind])}</span>{/if}
								</h2>
								{@render pageSections(page)}
							</div>
						{/each}
					{:else}
						{@render pageSections(activePage)}
					{/if}
				</div>
			</fieldset>
		{/if}
	{/if}
</div>
