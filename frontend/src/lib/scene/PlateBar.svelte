<script lang="ts">
	// Barre multi-plateaux (T059) : onglets de plateaux (sélection active, ajout,
	// suppression) et sélecteur de type de plaque du plateau courant.
	// Présentationnel — les mutations passent par des callbacks (le parent
	// applique sur le PlateSet et persiste dans le document scène).
	import { PLATE_TYPES } from '../settings/special/dialogs';
	import type { Plate } from './plates.svelte';

	interface Props {
		plates: readonly Plate[];
		activeId: string | null;
		onselect: (id: string) => void;
		onadd: () => void;
		onremove: (id: string) => void;
		ontype: (id: string, plateType: string) => void;
	}

	let { plates, activeId, onselect, onadd, onremove, ontype }: Props = $props();

	const active = $derived(plates.find((p) => p.id === activeId) ?? null);
</script>

<div class="flex flex-col gap-2 text-sm">
	<div class="flex flex-wrap items-center gap-1">
		{#each plates as plate (plate.id)}
			<div
				class="flex items-center rounded border {plate.id === activeId
					? 'border-accent bg-overlay'
					: 'border-border-strong bg-surface-sunken'}"
			>
				<button type="button" class="px-2 py-0.5" onclick={() => onselect(plate.id)}>
					{plate.name}
					<span class="text-content-subtle">({plate.objectIds.length})</span>
				</button>
				{#if plates.length > 1}
					<button
						type="button"
						class="px-1 text-xs text-content-muted hover:text-white"
						aria-label={`Supprimer ${plate.name}`}
						onclick={() => onremove(plate.id)}>✕</button
					>
				{/if}
			</div>
		{/each}
		<button
			type="button"
			class="rounded border border-border-strong px-2 py-0.5 hover:bg-overlay"
			aria-label="Ajouter un plateau"
			onclick={() => onadd()}>+</button
		>
	</div>

	{#if active}
		<label class="flex items-center gap-2">
			<span class="text-content-muted">Type de plaque</span>
			<select
				class="rounded border border-border-strong bg-surface-raised px-1 py-0.5"
				value={active.plateType}
				onchange={(e) => ontype(active.id, e.currentTarget.value)}
			>
				{#each PLATE_TYPES as pt (pt.label)}
					<option value={pt.label}>{pt.label}</option>
				{/each}
			</select>
		</label>
	{/if}
</div>
