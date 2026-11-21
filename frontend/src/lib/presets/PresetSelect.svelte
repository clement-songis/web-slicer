<script lang="ts">
	// Sélecteur de preset filament/process (T046) : liste système/utilisateur,
	// badge d'héritage, actions dériver / sauvegarder / supprimer. Le filtrage de
	// compatibilité est fait en amont (côté serveur, FR-021).
	import type { PresetSummary } from '../api/types';
	import { inheritanceLabel, isDerived, partitionByOrigin } from './catalog';
	import { filterPresetList, type ActivePrinter } from './search';
	import { appPreferences } from '$lib/settings/preferences';
	import { unsupportedPresets } from './unsupported';
	import { groupUserPresets, loadUserPresetMeta, type UserPresetMeta } from './user-grouping';
	import { getResolvedPreset } from '../api/presets';
	import ModifiedBadge from './ModifiedBadge.svelte';
	import { locale, t } from '$lib/i18n';
	import { UNSPECIFIED } from './user-grouping';

	interface Props {
		/** Presets d'un type (déjà filtrés par compatibilité). */
		presets: PresetSummary[];
		/** Id du preset sélectionné (liable). */
		selectedId?: string | null;
		/** Étiquette du sélecteur (« Filament », « Process »…). */
		label?: string;
		/** Mode compact (parité OrcaSlicer) : uniquement le menu déroulant, sans
		 *  étiquette ni rangée d'actions — les actions passent par un crayon/dialogue. */
		compact?: boolean;
		/** Hook du contrat UI (`print`, `filament`…). */
		testid?: string;
		/** Une valeur du brouillon diffère du preset résolu. */
		modified?: boolean;
		/** Dériver le preset courant (crée une copie utilisateur). */
		onderive?: (preset: PresetSummary) => void;
		/** Sauvegarder les modifications du preset courant. */
		onsave?: (preset: PresetSummary) => void;
		/** Supprimer le preset utilisateur courant. */
		ondelete?: (preset: PresetSummary) => void;
		/** Champ de recherche + regroupement par fabricant (listes longues). */
		searchable?: boolean;
		/** Imprimante active : seuls ses presets compatibles sont montrés par défaut. */
		printer?: ActivePrinter | null;
	}

	let {
		presets,
		selectedId = $bindable(null),
		label = 'Preset',
		compact = false,
		testid,
		modified = false,
		onderive,
		onsave,
		ondelete,
		searchable = false,
		printer = null
	}: Props = $props();

	const groups = $derived(partitionByOrigin(presets));
	// Orca group_filament_presets (phase 34) : regroupement des presets filament
	// de l'utilisateur ; les autres types gardent un seul groupe « Mes presets ».
	const grouping = $derived(
		presets[0]?.kind === 'filament' ? $appPreferences.groupFilamentPresets : 'all'
	);
	let userMeta = $state<Record<string, UserPresetMeta>>({});
	$effect(() => {
		const ids = groups.user.map((p) => p.id);
		if ((grouping !== 'type' && grouping !== 'vendor') || ids.length === 0) return;
		let alive = true;
		void loadUserPresetMeta(ids, async (id) => (await getResolvedPreset(id)).values).then(
			(meta) => {
				if (alive) userMeta = meta;
			}
		);
		return () => {
			alive = false;
		};
	});
	const userGroups = $derived.by(() => {
		void $locale; // libellés traduits à l'appel : recalcul au changement de langue
		return groupUserPresets(groups.user, grouping, userMeta);
	});
	/** Libellé d'`<optgroup>` affiché : clés anglaises traduites, noms de fabricant tels quels. */
	function groupLabel(label: string): string {
		const suffix = ` · ${UNSPECIFIED}`;
		return label.endsWith(suffix)
			? label.slice(0, -suffix.length) + ` · ${$t(UNSPECIFIED)}`
			: $t(label);
	}
	// Recherche (accents/casse ignorés) et filtre de compatibilité imprimante.
	let query = $state('');
	// Orca `show_unsupported_presets` : la préférence coche « Afficher les
	// incompatibles » par défaut et ajoute les presets prévus pour d'autres
	// imprimantes, grisés (non sélectionnables), en fin de liste.
	let showAll = $state($appPreferences.showUnsupportedPresets);
	let unsupported = $state<PresetSummary[]>([]);
	$effect(() => {
		const kind = presets[0]?.kind;
		if (!$appPreferences.showUnsupportedPresets || !printer || !kind) {
			unsupported = [];
			return;
		}
		let alive = true;
		void unsupportedPresets(kind, presets).then((list) => {
			if (alive) unsupported = list;
		});
		return () => {
			alive = false;
		};
	});
	// Compact avec imprimante (filament) : tout est proposé, comme Orca (la
	// bibliothèque de filaments vaut pour toutes les imprimantes), mais groupé par
	// fabricant — Génériques en tête — au lieu d'une liste plate de centaines de noms.
	const filtered = $derived(
		$locale && searchable
			? filterPresetList(presets, {
					query,
					printer,
					showAll,
					selectedId,
					userGrouping: grouping,
					userMeta
				})
			: compact && printer
				? filterPresetList(presets, {
						query,
						printer,
						showAll: true,
						selectedId,
						userGrouping: grouping,
						userMeta
					})
				: null
	);
	// Phase 38 — liste compacte longue (filaments : ~280 profils système) :
	// loupe qui déplie un champ de recherche au-dessus du menu, sans alourdir
	// chaque ligne de filament. Échap / re-clic : referme et vide la recherche.
	const SEARCH_THRESHOLD = 30;
	const longList = $derived(
		compact && !searchable && !!printer && presets.length > SEARCH_THRESHOLD
	);
	let searchOpen = $state(false);
	let searchInput = $state<HTMLInputElement>();
	function toggleSearch() {
		searchOpen = !searchOpen;
		if (!searchOpen) query = '';
		else queueMicrotask(() => searchInput?.focus());
	}
	const selected = $derived(presets.find((p) => p.id === selectedId) ?? null);
	const badge = $derived(selected && $locale ? inheritanceLabel(selected) : null);
	const isUser = $derived(selected?.origin === 'user');

	const FIELD =
		'w-full rounded border border-border-strong px-2 py-1 text-sm bg-surface-raised text-content';
	const BTN = 'ws-btn ws-btn-sm ws-btn-secondary';
</script>

{#snippet unsupportedOptions()}
	{#if unsupported.length}
		<optgroup
			label={$t('Incompatible with this printer')}
			data-testid={testid ? `${testid}-unsupported` : undefined}
		>
			{#each unsupported as p (p.id)}
				<option value={p.id} disabled>{p.name}</option>
			{/each}
		</optgroup>
	{/if}
{/snippet}

{#snippet options()}
	<option value={null} disabled>{$t('Select…')}</option>
	{#if filtered}
		{#each filtered.groups as g (g.label)}
			{#if g.label}
				<optgroup label={groupLabel(g.label)}>
					{#each g.presets as p (p.id)}
						<option value={p.id}>{p.name}{isDerived(p) ? ' ✎' : ''}</option>
					{/each}
				</optgroup>
			{:else}
				<!-- Orca « None » : presets utilisateur sans sous-groupe. -->
				{#each g.presets as p (p.id)}
					<option value={p.id}>{p.name}{isDerived(p) ? ' ✎' : ''}</option>
				{/each}
			{/if}
		{/each}
	{:else if groups.system.length}
		<optgroup label={$t('System')}>
			{#each groups.system as p (p.id)}
				<option value={p.id}>{p.name}</option>
			{/each}
		</optgroup>
	{/if}
	{#if !filtered}
		{#each userGroups as g (g.label)}
			{#if g.label}
				<optgroup label={groupLabel(g.label)}>
					{#each g.presets as p (p.id)}
						<option value={p.id}>{p.name}{isDerived(p) ? ' ✎' : ''}</option>
					{/each}
				</optgroup>
			{:else}
				{#each g.presets as p (p.id)}
					<option value={p.id}>{p.name}{isDerived(p) ? ' ✎' : ''}</option>
				{/each}
			{/if}
		{/each}
	{/if}
	{@render unsupportedOptions()}
{/snippet}

{#snippet search()}
	{#if filtered}
		<div class="flex flex-col gap-1">
			<input
				type="search"
				bind:value={query}
				placeholder={$t('Search a profile…')}
				aria-label={$t('Search: {label}', { label })}
				class={FIELD}
				data-testid={testid ? `${testid}-search` : undefined}
			/>
			<div class="flex items-center justify-between gap-2 text-xs text-content-muted">
				<span data-testid={testid ? `${testid}-count` : undefined}>
					{filtered.shown} / {showAll ? filtered.total : filtered.compatible}
					{showAll ? $t('profiles') : $t('compatible')}
				</span>
				{#if printer}
					<label class="flex items-center gap-1">
						<input
							type="checkbox"
							bind:checked={showAll}
							data-testid={testid ? `${testid}-show-all` : undefined}
						/>
						{$t('Show incompatible')}
					</label>
				{/if}
			</div>
		</div>
	{/if}
{/snippet}

{#if compact}
	{#if searchable}{@render search()}{/if}
	{#if longList}
		{#if searchOpen}
			<div class="mb-1 flex items-center gap-1">
				<input
					bind:this={searchInput}
					type="search"
					bind:value={query}
					placeholder={$t('Search a profile…')}
					aria-label={$t('Search: {label}', { label })}
					class={FIELD}
					data-testid={testid ? `${testid}-search` : undefined}
					onkeydown={(e) => {
						if (e.key === 'Escape') {
							e.stopPropagation();
							toggleSearch();
						}
					}}
				/>
				{#if filtered}
					<span
						class="shrink-0 text-xs text-content-muted tabular-nums"
						data-testid={testid ? `${testid}-count` : undefined}
						>{filtered.shown}/{filtered.total}</span
					>
				{/if}
			</div>
		{/if}
		<div class="flex items-center gap-1">
			<select
				bind:value={selectedId}
				aria-label={label}
				class={FIELD}
				data-testid={testid}
				data-modified={modified ? 'true' : 'false'}
			>
				{@render options()}
			</select>
			{#if modified}<ModifiedBadge testid={testid ? `${testid}-modified` : undefined} />{/if}
			<button
				type="button"
				class="ws-btn ws-btn-sm ws-btn-ghost shrink-0 px-1.5 {searchOpen ? 'text-primary' : ''}"
				aria-label={$t('Search: {label}', { label })}
				aria-pressed={searchOpen}
				title={$t('Search in the list')}
				data-testid={testid ? `${testid}-search-toggle` : undefined}
				onclick={toggleSearch}
			>
				<svg
					viewBox="0 0 24 24"
					class="size-3.5"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" /></svg
				>
			</button>
		</div>
	{:else}
		<div class="flex items-center gap-1">
			<select
				bind:value={selectedId}
				aria-label={label}
				class={FIELD}
				data-testid={testid}
				data-modified={modified ? 'true' : 'false'}
			>
				{@render options()}
			</select>
			{#if modified}<ModifiedBadge testid={testid ? `${testid}-modified` : undefined} />{/if}
		</div>
	{/if}
{:else}
	<div class="flex flex-col gap-1">
		{@render search()}
		<div class="flex items-center gap-2">
			<span class="w-16 shrink-0 text-sm text-content-muted">{label}</span>
			<select
				bind:value={selectedId}
				aria-label={label}
				class={FIELD}
				data-testid={testid}
				data-modified={modified ? 'true' : 'false'}
			>
				{@render options()}
			</select>
			{#if modified}<ModifiedBadge testid={testid ? `${testid}-modified` : undefined} />{/if}
		</div>

		<div class="flex items-center gap-2 pl-18">
			{#if badge}
				<span class="rounded bg-primary/10 px-1.5 py-0.5 text-xs text-primary">
					{badge}
				</span>
			{/if}
			<div class="ml-auto flex gap-1">
				<button
					type="button"
					class={BTN}
					disabled={!selected}
					onclick={() => selected && onderive?.(selected)}
				>
					{$t('Derive')}
				</button>
				<button
					type="button"
					class={BTN}
					disabled={!isUser}
					onclick={() => selected && onsave?.(selected)}
				>
					{$t('Save')}
				</button>
				<button
					type="button"
					class={BTN}
					disabled={!isUser}
					onclick={() => selected && ondelete?.(selected)}
				>
					{$t('Delete')}
				</button>
			</div>
		</div>
	</div>
{/if}
