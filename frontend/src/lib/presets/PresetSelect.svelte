<script lang="ts">
	// Sélecteur de preset filament/process (T046) : liste système/utilisateur,
	// badge d'héritage, actions dériver / sauvegarder / supprimer. Le filtrage de
	// compatibilité est fait en amont (côté serveur, FR-021).
	import type { PresetSummary } from '../api/types';
	import { inheritanceLabel, isDerived, partitionByOrigin } from './catalog';
	import { filterPresetList, type ActivePrinter } from './search';

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
	// Recherche (accents/casse ignorés) et filtre de compatibilité imprimante.
	let query = $state('');
	let showAll = $state(false);
	// Compact avec imprimante (filament) : tout est proposé, comme Orca (la
	// bibliothèque de filaments vaut pour toutes les imprimantes), mais groupé par
	// fabricant — Génériques en tête — au lieu d'une liste plate de centaines de noms.
	const filtered = $derived(
		searchable
			? filterPresetList(presets, { query, printer, showAll, selectedId })
			: compact && printer
				? filterPresetList(presets, { printer, showAll: true, selectedId })
				: null
	);
	const selected = $derived(presets.find((p) => p.id === selectedId) ?? null);
	const badge = $derived(selected ? inheritanceLabel(selected) : null);
	const isUser = $derived(selected?.origin === 'user');

	const FIELD =
		'w-full rounded border border-border-strong px-2 py-1 text-sm bg-surface-raised text-content';
	const BTN = 'ws-btn ws-btn-sm ws-btn-secondary';
</script>

{#snippet options()}
	<option value={null} disabled>Choisir…</option>
	{#if filtered}
		{#each filtered.groups as g (g.label)}
			<optgroup label={g.label}>
				{#each g.presets as p (p.id)}
					<option value={p.id}>{p.name}{isDerived(p) ? ' ✎' : ''}</option>
				{/each}
			</optgroup>
		{/each}
	{:else if groups.system.length}
		<optgroup label="Système">
			{#each groups.system as p (p.id)}
				<option value={p.id}>{p.name}</option>
			{/each}
		</optgroup>
	{/if}
	{#if !filtered && groups.user.length}
		<optgroup label="Mes presets">
			{#each groups.user as p (p.id)}
				<option value={p.id}>{p.name}{isDerived(p) ? ' ✎' : ''}</option>
			{/each}
		</optgroup>
	{/if}
{/snippet}

{#snippet search()}
	{#if filtered}
		<div class="flex flex-col gap-1">
			<input
				type="search"
				bind:value={query}
				placeholder="Rechercher un profil…"
				aria-label="Rechercher : {label}"
				class={FIELD}
				data-testid={testid ? `${testid}-search` : undefined}
			/>
			<div class="flex items-center justify-between gap-2 text-xs text-content-muted">
				<span data-testid={testid ? `${testid}-count` : undefined}>
					{filtered.shown} / {showAll ? filtered.total : filtered.compatible}
					{showAll ? 'profils' : 'compatibles'}
				</span>
				{#if printer}
					<label class="flex items-center gap-1">
						<input
							type="checkbox"
							bind:checked={showAll}
							data-testid={testid ? `${testid}-show-all` : undefined}
						/>
						Afficher les incompatibles
					</label>
				{/if}
			</div>
		</div>
	{/if}
{/snippet}

{#if compact}
	{#if searchable}{@render search()}{/if}
	<select
		bind:value={selectedId}
		aria-label={label}
		class={FIELD}
		data-testid={testid}
		data-modified={modified ? 'true' : 'false'}
	>
		{@render options()}
	</select>
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
					Dériver
				</button>
				<button
					type="button"
					class={BTN}
					disabled={!isUser}
					onclick={() => selected && onsave?.(selected)}
				>
					Enregistrer
				</button>
				<button
					type="button"
					class={BTN}
					disabled={!isUser}
					onclick={() => selected && ondelete?.(selected)}
				>
					Supprimer
				</button>
			</div>
		</div>
	</div>
{/if}
