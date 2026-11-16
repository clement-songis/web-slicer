<script lang="ts">
	// Grille de sélection d'imprimante (T134) — portage web du dialogue OrcaSlicer
	// « Sélection de l'imprimante » : modèles groupés par marque, multi-sélection
	// (cases + compteur X / N et « tout sélectionner » par marque), bascules de
	// vue, recherche, « Create » (imprimante custom) et Confirmer/Annuler. Toute la
	// logique vit dans `catalog.ts` (règle CLAUDE.md : le .svelte ne fait que la
	// disposition).
	import type { PrinterCatalogModel, PrinterCatalogVendor } from '$lib/api/types';
	import {
		modelKey,
		coverUrl,
		filterCatalog,
		isSelected,
		selectedModels,
		toggleModel,
		toggleVendorAll,
		vendorAllSelected,
		vendorSelectedCount,
		type CatalogView
	} from './catalog';
	import Search from '~icons/lucide/search';
	import ListIcon from '~icons/lucide/list';
	import GridIcon from '~icons/lucide/grid-3x3';
	import LargeIcon from '~icons/lucide/layout-grid';
	import PrinterIcon from '~icons/lucide/printer';
	import Plus from '~icons/lucide/plus';
	import Check from '~icons/lucide/check';

	interface Props {
		/** Catalogue (marque → modèle → variantes) renvoyé par `getPrinterCatalog`. */
		vendors: PrinterCatalogVendor[];
		/** Confirmation : les modèles sélectionnés (un par imprimante à créer). */
		onconfirm: (models: PrinterCatalogModel[], nozzles?: Record<string, string[]>) => void;
		/** Choix des buses par modèle (assistant d'Orca) : `nozzles[modelKey]` =
		 *  presets machine retenus, transmis à `onconfirm`. */
		chooseNozzle?: boolean;
		/** Annulation (facultatif). */
		oncancel?: () => void;
		/** Création d'une imprimante custom (facultatif). */
		oncreate?: () => void;
		/** Libellé du bouton de confirmation. */
		confirmLabel?: string;
		/** Désactive les actions pendant un envoi. */
		busy?: boolean;
	}

	let {
		vendors,
		onconfirm,
		oncancel,
		oncreate,
		confirmLabel = 'Confirmer',
		busy = false,
		chooseNozzle = false
	}: Props = $props();

	let selected = $state<Set<string>>(new Set());

	// Buses cochées par modèle (défaut : variante par défaut du catalogue).
	let nozzlePick = $state<Record<string, string[]>>({});
	const picked = $derived(
		selectedModels(vendors, selected).map((m) => ({
			model: m,
			key: modelKey(m),
			ids: nozzlePick[modelKey(m)] ?? [m.default_machine_preset_id]
		}))
	);
	const sortedVariants = (m: PrinterCatalogModel) =>
		[...m.variants].sort((a, b) => Number(a.nozzle || 0) - Number(b.nozzle || 0));
	function toggleNozzle(key: string, current: string[], id: string) {
		const next = current.includes(id) ? current.filter((x) => x !== id) : [...current, id];
		nozzlePick = { ...nozzlePick, [key]: next };
	}
	const missingNozzle = $derived(chooseNozzle && picked.some((p) => p.ids.length === 0));

	let query = $state('');
	let view = $state<CatalogView>('grid');

	const filtered = $derived(filterCatalog(vendors, query));
	const count = $derived(selected.size);

	// Classes de grille selon la densité choisie.
	const gridClass = $derived(
		view === 'list'
			? 'grid grid-cols-1 gap-1'
			: view === 'large'
				? 'grid grid-cols-2 gap-3 sm:grid-cols-3'
				: 'grid grid-cols-3 gap-2 sm:grid-cols-4 md:grid-cols-5'
	);
	const thumbClass = $derived(
		view === 'list' ? 'h-10 w-10' : view === 'large' ? 'h-28 w-full' : 'h-20 w-full'
	);

	function confirm() {
		if (chooseNozzle)
			onconfirm(
				picked.map((p) => p.model),
				Object.fromEntries(picked.map((p) => [p.key, p.ids]))
			);
		else onconfirm(selectedModels(vendors, selected));
	}

	const VIEWS: { id: CatalogView; icon: typeof ListIcon; label: string }[] = [
		{ id: 'list', icon: ListIcon, label: 'Liste' },
		{ id: 'grid', icon: GridIcon, label: 'Grille' },
		{ id: 'large', icon: LargeIcon, label: 'Grandes vignettes' }
	];
</script>

<div class="flex h-full flex-col">
	<!-- Barre d'outils : recherche + bascules de vue -->
	<div class="flex items-center gap-2 border-b border-border px-3 py-2">
		<div class="relative flex-1">
			<Search
				class="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-content-subtle"
			/>
			<input
				bind:value={query}
				type="search"
				placeholder="Rechercher une imprimante…"
				aria-label="Rechercher une imprimante"
				class="ws-field w-full pl-8 pr-2"
			/>
		</div>
		<div class="flex items-center gap-1" role="group" aria-label="Densité d'affichage">
			{#each VIEWS as v (v.id)}
				{@const Icon = v.icon}
				<button
					type="button"
					onclick={() => (view = v.id)}
					aria-pressed={view === v.id}
					aria-label={v.label}
					title={v.label}
					class="ws-tool ws-tool-sm {view === v.id ? 'ws-tool-on' : ''}"
				>
					<Icon />
				</button>
			{/each}
		</div>
	</div>

	<!-- Sections par marque -->
	<div class="flex-1 overflow-y-auto px-3 py-2" data-testid="printer-catalog">
		{#if filtered.length === 0}
			<p class="ws-empty">Aucune imprimante trouvée.</p>
		{/if}

		{#each filtered as vendor (vendor.vendor)}
			{@const n = vendor.models.length}
			{@const sel = vendorSelectedCount(selected, vendor)}
			<section class="mb-4">
				<header
					class="mb-2 flex items-center gap-2 border-b border-accent/40 pb-1 text-sm font-medium text-accent"
				>
					<span class="flex-1">{vendor.vendor === 'Custom' ? 'Personnalisé' : vendor.vendor}</span>
					<span class="text-xs tabular-nums text-content-subtle">{sel} / {n}</span>
					<input
						type="checkbox"
						checked={vendorAllSelected(selected, vendor)}
						onchange={() => (selected = toggleVendorAll(selected, vendor))}
						aria-label={`Tout sélectionner — ${vendor.vendor === 'Custom' ? 'Personnalisé' : vendor.vendor}`}
					/>
				</header>

				<div class={gridClass}>
					{#each vendor.models as model (model.model)}
						{@const on = isSelected(selected, model)}
						<button
							type="button"
							onclick={() => (selected = toggleModel(selected, model))}
							aria-pressed={on}
							class="group relative flex items-center rounded border p-1 text-left {view === 'list'
								? 'flex-row gap-2'
								: 'flex-col gap-1'} {on
								? 'border-accent ring-1 ring-accent'
								: 'border-border hover:border-border-strong'}"
						>
							<!-- Coche de sélection -->
							{#if on}
								<span
									class="absolute right-1 top-1 z-10 rounded-full bg-accent p-0.5 text-primary-content"
								>
									<Check class="text-xs" />
								</span>
							{/if}
							<!-- Vignette : cover PNG, repli placeholder si absente -->
							<span
								class="relative flex shrink-0 items-center justify-center overflow-hidden rounded bg-surface {thumbClass}"
							>
								<PrinterIcon class="absolute text-content-subtle opacity-40" />
								<img
									src={coverUrl(model)}
									alt={model.model}
									loading="lazy"
									onerror={(e) => e.currentTarget.classList.add('hidden')}
									class="relative h-full w-full object-contain"
								/>
							</span>
							<span
								class="truncate text-xs text-content {view === 'list'
									? 'flex-1'
									: 'w-full text-center'}"
								title={model.model}>{model.model}</span
							>
							{#if model.variants.length > 1}
								<span class="text-2xs text-content-subtle">{model.variants.length} buses</span>
							{/if}
						</button>
					{/each}
				</div>
			</section>
		{/each}
	</div>

	<!-- Buses des modèles choisis (assistant d'Orca : diamètres cochés par modèle). -->
	{#if chooseNozzle && picked.some((p) => p.model.variants.length > 1)}
		<div
			class="flex max-h-40 flex-col gap-1.5 overflow-y-auto border-t border-border px-3 py-2"
			data-testid="nozzle-choice"
		>
			<span class="ws-section-title">Buses</span>
			{#each picked.filter((p) => p.model.variants.length > 1) as p (p.key)}
				<div class="flex flex-wrap items-center gap-2 text-sm">
					<span class="min-w-40 text-content">{p.model.model}</span>
					<div class="flex flex-wrap gap-1" role="group" aria-label="Diamètres de buse">
						{#each sortedVariants(p.model) as v (v.machine_preset_id)}
							{@const on = p.ids.includes(v.machine_preset_id)}
							<button
								type="button"
								class="ws-chip {on ? 'ws-chip-on' : ''}"
								aria-pressed={on}
								data-testid="nozzle-option"
								onclick={() => toggleNozzle(p.key, p.ids, v.machine_preset_id)}
								>{v.nozzle ? `${v.nozzle} mm` : 'standard'}</button
							>
						{/each}
					</div>
					{#if p.ids.length === 0}
						<span class="text-xs text-danger">Choisissez au moins une buse.</span>
					{/if}
				</div>
			{/each}
		</div>
	{/if}

	<!-- Pied : Create · compteur · Annuler / Confirmer -->
	<div class="flex items-center gap-2 border-t border-border px-3 py-2">
		{#if oncreate}
			<button type="button" onclick={oncreate} class="ws-btn ws-btn-secondary">
				<Plus /> Create
			</button>
		{/if}
		<span class="flex-1 text-xs text-content-subtle">
			{count} imprimante{count > 1 ? 's' : ''} sélectionnée{count > 1 ? 's' : ''}
		</span>
		{#if oncancel}
			<button type="button" onclick={oncancel} class="ws-btn ws-btn-secondary"> Annuler </button>
		{/if}
		<button
			type="button"
			onclick={confirm}
			disabled={count === 0 || busy || missingNozzle}
			class="ws-btn ws-btn-primary"
		>
			{confirmLabel}
		</button>
	</div>
</div>
