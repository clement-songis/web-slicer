<script lang="ts">
	// Grille de sélection d'imprimante (T134) — portage web du dialogue OrcaSlicer
	// « Sélection de l'imprimante » : modèles groupés par marque, multi-sélection
	// (cases + compteur X / N et « tout sélectionner » par marque), bascules de
	// vue, recherche, « Create » (imprimante custom) et Confirmer/Annuler. Toute la
	// logique vit dans `catalog.ts` (règle CLAUDE.md : le .svelte ne fait que la
	// disposition).
	import type { PrinterCatalogModel, PrinterCatalogVendor } from '$lib/api/types';
	import {
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
		onconfirm: (models: PrinterCatalogModel[]) => void;
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
		busy = false
	}: Props = $props();

	let query = $state('');
	let view = $state<CatalogView>('grid');
	let selected = $state<Set<string>>(new Set());

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
		onconfirm(selectedModels(vendors, selected));
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
				class="w-full rounded border border-border-strong bg-surface-raised py-1 pl-8 pr-2 text-sm text-content"
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
					class="rounded p-1.5 {view === v.id
						? 'bg-primary text-primary-content'
						: 'text-content-subtle hover:bg-surface-raised'}"
				>
					<Icon />
				</button>
			{/each}
		</div>
	</div>

	<!-- Sections par marque -->
	<div class="flex-1 overflow-y-auto px-3 py-2" data-testid="printer-catalog">
		{#if filtered.length === 0}
			<p class="py-8 text-center text-sm text-content-subtle">Aucune imprimante trouvée.</p>
		{/if}

		{#each filtered as vendor (vendor.vendor)}
			{@const n = vendor.models.length}
			{@const sel = vendorSelectedCount(selected, vendor)}
			<section class="mb-4">
				<header
					class="mb-2 flex items-center gap-2 border-b border-accent/40 pb-1 text-sm font-medium text-accent"
				>
					<span class="flex-1">{vendor.vendor}</span>
					<span class="text-xs tabular-nums text-content-subtle">{sel} / {n}</span>
					<input
						type="checkbox"
						checked={vendorAllSelected(selected, vendor)}
						onchange={() => (selected = toggleVendorAll(selected, vendor))}
						aria-label={`Tout sélectionner — ${vendor.vendor}`}
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
								<span class="absolute right-1 top-1 z-10 rounded-full bg-accent p-0.5 text-white">
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
								<span class="text-[10px] text-content-subtle">{model.variants.length} buses</span>
							{/if}
						</button>
					{/each}
				</div>
			</section>
		{/each}
	</div>

	<!-- Pied : Create · compteur · Annuler / Confirmer -->
	<div class="flex items-center gap-2 border-t border-border px-3 py-2">
		{#if oncreate}
			<button
				type="button"
				onclick={oncreate}
				class="flex items-center gap-1 rounded border border-border-strong px-2 py-1 text-sm text-content hover:bg-surface-raised"
			>
				<Plus /> Create
			</button>
		{/if}
		<span class="flex-1 text-xs text-content-subtle">
			{count} imprimante{count > 1 ? 's' : ''} sélectionnée{count > 1 ? 's' : ''}
		</span>
		{#if oncancel}
			<button
				type="button"
				onclick={oncancel}
				class="rounded border border-border-strong px-3 py-1 text-sm text-content hover:bg-surface-raised"
			>
				Annuler
			</button>
		{/if}
		<button
			type="button"
			onclick={confirm}
			disabled={count === 0 || busy}
			class="rounded bg-primary px-3 py-1 text-sm font-medium text-primary-content disabled:opacity-50"
		>
			{confirmLabel}
		</button>
	</div>
</div>
