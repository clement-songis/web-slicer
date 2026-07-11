<script lang="ts">
	// Barre multi-plateaux (T059) : onglets de plateaux (sélection active, ajout,
	// suppression) et sélecteur de type de plaque du plateau courant.
	// Présentationnel — les mutations passent par des callbacks (le parent
	// applique sur le PlateSet et persiste dans le document scène).
	import { PLATE_TYPES } from '../settings/special/dialogs';
	import type { Plate } from './plates';

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
					? 'border-sky-500 bg-slate-700'
					: 'border-slate-600 bg-slate-800'}"
			>
				<button type="button" class="px-2 py-0.5" onclick={() => onselect(plate.id)}>
					{plate.name}
					<span class="text-slate-500">({plate.objectIds.length})</span>
				</button>
				{#if plates.length > 1}
					<button
						type="button"
						class="px-1 text-xs text-slate-400 hover:text-white"
						aria-label={`Supprimer ${plate.name}`}
						onclick={() => onremove(plate.id)}>✕</button
					>
				{/if}
			</div>
		{/each}
		<button
			type="button"
			class="rounded border border-slate-600 px-2 py-0.5 hover:bg-slate-700"
			aria-label="Ajouter un plateau"
			onclick={() => onadd()}>+</button
		>
	</div>

	{#if active}
		<label class="flex items-center gap-2">
			<span class="text-slate-400">Type de plaque</span>
			<select
				class="rounded border border-slate-600 bg-slate-900 px-1 py-0.5"
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
