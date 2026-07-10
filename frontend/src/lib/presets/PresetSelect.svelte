<script lang="ts">
	// Sélecteur de preset filament/process (T046) : liste système/utilisateur,
	// badge d'héritage, actions dériver / sauvegarder / supprimer. Le filtrage de
	// compatibilité est fait en amont (côté serveur, FR-021).
	import type { PresetSummary } from '../api/types';
	import { inheritanceLabel, isDerived, partitionByOrigin } from './catalog';

	interface Props {
		/** Presets d'un type (déjà filtrés par compatibilité). */
		presets: PresetSummary[];
		/** Id du preset sélectionné (liable). */
		selectedId?: string | null;
		/** Étiquette du sélecteur (« Filament », « Process »…). */
		label?: string;
		/** Dériver le preset courant (crée une copie utilisateur). */
		onderive?: (preset: PresetSummary) => void;
		/** Sauvegarder les modifications du preset courant. */
		onsave?: (preset: PresetSummary) => void;
		/** Supprimer le preset utilisateur courant. */
		ondelete?: (preset: PresetSummary) => void;
	}

	let {
		presets,
		selectedId = $bindable(null),
		label = 'Preset',
		onderive,
		onsave,
		ondelete
	}: Props = $props();

	const groups = $derived(partitionByOrigin(presets));
	const selected = $derived(presets.find((p) => p.id === selectedId) ?? null);
	const badge = $derived(selected ? inheritanceLabel(selected) : null);
	const isUser = $derived(selected?.origin === 'user');

	const FIELD =
		'w-full rounded border border-gray-300 px-2 py-1 text-sm dark:border-gray-600 dark:bg-gray-700 dark:text-gray-100';
	const BTN =
		'rounded border border-gray-300 px-2 py-1 text-xs hover:bg-gray-100 disabled:opacity-40 dark:border-gray-600 dark:hover:bg-gray-700';
</script>

<div class="flex flex-col gap-1">
	<div class="flex items-center gap-2">
		<span class="w-16 shrink-0 text-sm text-gray-700 dark:text-gray-300">{label}</span>
		<select bind:value={selectedId} aria-label={label} class={FIELD}>
			<option value={null} disabled>Choisir…</option>
			{#if groups.system.length}
				<optgroup label="Système">
					{#each groups.system as p (p.id)}
						<option value={p.id}>{p.name}</option>
					{/each}
				</optgroup>
			{/if}
			{#if groups.user.length}
				<optgroup label="Mes presets">
					{#each groups.user as p (p.id)}
						<option value={p.id}>{p.name}{isDerived(p) ? ' ✎' : ''}</option>
					{/each}
				</optgroup>
			{/if}
		</select>
	</div>

	<div class="flex items-center gap-2 pl-18">
		{#if badge}
			<span
				class="rounded bg-blue-50 px-1.5 py-0.5 text-xs text-blue-700 dark:bg-blue-900/40 dark:text-blue-300"
			>
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
