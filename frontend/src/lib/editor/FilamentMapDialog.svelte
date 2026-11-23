<script lang="ts">
	// Répartition des filaments par buse avant le tranchage (Orca
	// FilamentMapDialog / try_pop_up_before_slice, imprimantes à deux buses).
	// Plus direct qu'Orca : deux colonnes Gauche / Droite, un clic sur une
	// pastille la fait passer de l'autre côté (au lieu du glisser-déposer).
	import { dialogFocus } from '$lib/ui/dialog-focus';
	import { swatchStyle } from '$lib/ui/color';
	import { t } from '$lib/i18n';
	import { nextNozzle, nozzleLabel, type FilamentMapMode } from './filament-map';

	interface Props {
		/** Nombre de buses (2). */
		extruders: number;
		/** Couleur de chaque filament de la barre. */
		colours: readonly string[];
		/** Type de chaque filament (PLA, PETG…), facultatif. */
		types?: readonly string[];
		/** Mode appliqué à l'ouverture. */
		mode: FilamentMapMode;
		/** Carte actuelle (1-based, un élément par filament). */
		map: readonly number[];
		/** Filaments listés (1-based) : ceux du plateau, ou tous. */
		filaments: readonly number[];
		/** « Tout trancher » : le choix s'applique à tous les plateaux. */
		allPlates?: boolean;
		onconfirm: (mode: FilamentMapMode, map: number[]) => void;
		oncancel: () => void;
		/** « Ne plus demander » : désactive la préférence. */
		ondisable?: () => void;
	}
	let {
		extruders,
		colours,
		types = [],
		mode,
		map,
		filaments,
		allPlates = false,
		onconfirm,
		oncancel,
		ondisable
	}: Props = $props();

	// Brouillon initialisé une fois à l'ouverture (le dialogue est recréé à chaque tranchage).
	// svelte-ignore state_referenced_locally
	let kind = $state<'auto' | 'manual'>(mode === 'Manual' ? 'manual' : 'auto');
	// svelte-ignore state_referenced_locally
	let autoMode = $state<FilamentMapMode>(
		mode === 'Auto For Match' ? 'Auto For Match' : 'Auto For Flush'
	);
	// svelte-ignore state_referenced_locally
	let draft = $state<number[]>([...map]);
	let stopAsking = $state(false);

	const nozzles = $derived(Array.from({ length: extruders }, (_, k) => k + 1));

	function confirm() {
		if (stopAsking) ondisable?.();
		onconfirm(kind === 'manual' ? 'Manual' : autoMode, [...draft]);
	}
</script>

<div class="fixed inset-0 z-50 flex items-center justify-center bg-scrim p-4" role="presentation">
	<div
		class="ws-dialog flex w-[34rem] max-w-[calc(100vw-2rem)] flex-col gap-3 p-4"
		role="dialog"
		aria-modal="true"
		aria-labelledby="fmap-title"
		data-testid="filament-map-dialog"
		tabindex="-1"
		use:dialogFocus={oncancel}
	>
		<div>
			<h2 id="fmap-title" class="ws-dialog-title">{$t('Filament grouping')}</h2>
			<p class="text-xs text-content-subtle">
				{$t('{scope}: which nozzle prints each filament?', {
					scope: allPlates ? $t('For all plates') : $t('For this plate')
				})}
			</p>
		</div>
		<div class="ws-seg flex w-full" role="radiogroup" aria-label={$t('Grouping')}>
			<button
				type="button"
				role="radio"
				aria-checked={kind === 'auto'}
				data-testid="filament-map-kind-auto"
				class="ws-seg-item flex-1 {kind === 'auto' ? 'ws-seg-on' : ''}"
				onclick={() => (kind = 'auto')}
			>
				{$t('Automatic')}
			</button>
			<button
				type="button"
				role="radio"
				aria-checked={kind === 'manual'}
				data-testid="filament-map-kind-manual"
				class="ws-seg-item flex-1 {kind === 'manual' ? 'ws-seg-on' : ''}"
				onclick={() => (kind = 'manual')}
			>
				{$t('Manual||map')}
			</button>
		</div>

		{#if kind === 'auto'}
			<fieldset class="flex flex-col gap-1.5 text-sm" data-testid="filament-map-auto">
				<legend class="sr-only">{$t('Automatic grouping criterion')}</legend>
				<label class="flex items-start gap-2 rounded border border-border p-2">
					<input
						type="radio"
						name="fmap-auto"
						value="Auto For Flush"
						data-testid="filament-map-auto-flush"
						bind:group={autoMode}
					/>
					<span>
						<span class="block text-content">{$t('Less flushing')}</span>
						<span class="block text-xs text-content-subtle">
							{$t('The slicer groups filaments to limit changes and waste.')}
						</span>
					</span>
				</label>
				<label class="flex items-start gap-2 rounded border border-border p-2">
					<input
						type="radio"
						name="fmap-auto"
						value="Auto For Match"
						data-testid="filament-map-auto-match"
						bind:group={autoMode}
					/>
					<span>
						<span class="block text-content">{$t('By loaded spools')}</span>
						<span class="block text-xs text-content-subtle">
							{$t('Follows the filaments present on each printer nozzle (AMS).')}
						</span>
					</span>
				</label>
			</fieldset>
		{:else}
			<div class="grid gap-2" style="grid-template-columns: repeat({extruders}, minmax(0, 1fr))">
				{#each nozzles as n (n)}
					<section
						class="flex min-h-24 flex-col gap-1.5 rounded border border-border p-2"
						data-testid="filament-map-nozzle-{n}"
						aria-label={$t('Nozzle {name}', { name: nozzleLabel(n, extruders) })}
					>
						<h3 class="ws-section-title">{nozzleLabel(n, extruders)}</h3>
						{#each filaments.filter((f) => (draft[f - 1] ?? 1) === n) as f (f)}
							<button
								type="button"
								class="flex items-center gap-2 rounded px-1.5 py-1 text-left text-sm hover:bg-surface-sunken"
								data-testid="filament-map-chip-{f}"
								title={$t('Move to nozzle {name}', {
									name: nozzleLabel(nextNozzle(n, extruders), extruders)
								})}
								onclick={() => (draft[f - 1] = nextNozzle(n, extruders))}
							>
								<span
									class="flex h-5 w-5 shrink-0 items-center justify-center rounded text-[10px] font-semibold"
									style={swatchStyle(colours[f - 1])}
								>
									{f}
								</span>
								<span class="truncate">{types[f - 1] || $t('Filament {n}', { n: f })}</span>
								<span class="ml-auto text-xs text-content-subtle" aria-hidden="true">⇄</span>
							</button>
						{:else}
							<p class="text-xs text-content-subtle">{$t('No filament')}</p>
						{/each}
					</section>
				{/each}
			</div>
			<p class="text-xs text-content-subtle">{$t('Click a filament to change its nozzle.')}</p>
		{/if}

		<div class="flex items-center justify-between gap-2">
			{#if ondisable}
				<label class="flex items-center gap-2 text-xs text-content-muted">
					<input type="checkbox" data-testid="filament-map-stop-asking" bind:checked={stopAsking} />
					{$t("Don't ask again")}
				</label>
			{:else}
				<span></span>
			{/if}
			<div class="flex gap-2">
				<button
					type="button"
					class="ws-btn ws-btn-secondary"
					data-testid="filament-map-cancel"
					onclick={oncancel}
				>
					{$t('Cancel')}
				</button>
				<button
					type="button"
					class="ws-btn ws-btn-primary"
					data-testid="filament-map-confirm"
					onclick={confirm}
				>
					{$t('Slice')}
				</button>
			</div>
		</div>
	</div>
</div>
