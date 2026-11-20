<script lang="ts">
	// Barre multi-plateaux (T059) : onglets de plateaux (sélection active, ajout,
	// suppression) et sélecteur de type de plaque du plateau courant.
	// Présentationnel — les mutations passent par des callbacks (le parent
	// applique sur le PlateSet et persiste dans le document scène).
	import { PLATE_TYPES } from '../settings/special/dialogs';
	import { locale, t } from '$lib/i18n';
	import { loadSettingsTranslations } from '$lib/settings/i18n';
	import type { Plate } from './plates.svelte';
	import PlateSettings from './PlateSettings.svelte';
	import IconX from '~icons/lucide/x';
	import IconLock from '~icons/lucide/lock';
	import IconLockOpen from '~icons/lucide/lock-open';
	import IconFront from '~icons/lucide/arrow-left-to-line';

	interface Props {
		plates: readonly Plate[];
		activeId: string | null;
		onselect: (id: string) => void;
		onadd: () => void;
		onremove: (id: string) => void;
		ontype: (id: string, plateType: string) => void;
		/** Renommage en ligne (double-clic sur l'onglet). */
		onrename?: (id: string, name: string) => void;
		/** Réglage par plateau (`undefined` = suivre le profil). */
		onsetting?: (id: string, key: string, value: unknown) => void;
		/** Clic droit sur un onglet : menu contextuel du plateau (`create_plate_menu`). */
		oncontext?: (id: string, x: number, y: number) => void;
		/** Plateaux dont un résultat de tranchage est mémorisé (phase 25, « Slice all »). */
		sliced?: readonly string[];
		/**
		 * Verrouillage d'un plateau (lot 34, Orca PartPlate) : un plateau verrouillé
		 * n'est pas déplacé par « Tout arranger » et refuse « Arranger ce plateau ».
		 */
		onlock?: (id: string, locked: boolean) => void;
		/** Valeurs des presets, affichées dans l'option « Profil (…) » des réglages. */
		inherited?: Record<string, unknown>;
		/** « Déplacer le plateau vers l'avant » (icône du plateau d'Orca). */
		onfront?: (id: string) => void;
	}

	let {
		plates,
		activeId,
		onselect,
		onadd,
		onremove,
		ontype,
		onrename,
		onsetting,
		oncontext,
		sliced = [],
		inherited = {},
		onlock,
		onfront
	}: Props = $props();

	// Types de plaque traduits (dictionnaire des réglages chargé à la demande).
	$effect(() => {
		loadSettingsTranslations($locale).catch(() => {});
	});
	let editingId = $state<string | null>(null);

	const active = $derived(plates.find((p) => p.id === activeId) ?? null);
</script>

<div class="flex flex-col gap-2 text-sm">
	<div class="flex flex-wrap items-center gap-1">
		{#each plates as plate (plate.id)}
			<div
				class="flex items-center rounded border {plate.id === activeId
					? 'border-accent bg-overlay'
					: 'border-border-strong bg-surface-sunken'}"
				data-testid="plate-tab"
				data-plate-id={plate.id}
				data-active={plate.id === activeId ? 'true' : 'false'}
				data-sliced={sliced.includes(plate.id) ? 'true' : 'false'}
				data-locked={plate.locked ? 'true' : 'false'}
				role="presentation"
				oncontextmenu={(e) => {
					if (!oncontext) return;
					e.preventDefault();
					oncontext(plate.id, e.clientX, e.clientY);
				}}
			>
				{#if editingId === plate.id && onrename}
					<input
						class="ws-field ws-field-sm w-32 border-accent"
						data-testid="plate-rename-input"
						aria-label={`Nouveau nom de ${plate.name}`}
						value={plate.name}
						{@attach (el: HTMLInputElement) => {
							el.focus();
							el.select();
						}}
						onkeydown={(e) => {
							e.stopPropagation();
							if (e.key === 'Enter') {
								onrename(plate.id, e.currentTarget.value);
								editingId = null;
							} else if (e.key === 'Escape') editingId = null;
						}}
						onblur={(e) => {
							if (editingId === plate.id) onrename(plate.id, e.currentTarget.value);
							editingId = null;
						}}
					/>
				{:else}
					<button
						type="button"
						class="rounded px-2 py-0.5 hover:text-content"
						aria-pressed={plate.id === activeId}
						title="Double-clic pour renommer"
						onclick={() => onselect(plate.id)}
						ondblclick={() => onrename && (editingId = plate.id)}
					>
						{plate.name}
						<span class="text-content-subtle">({plate.objectIds.length})</span>
						{#if sliced.includes(plate.id)}<span
								class="text-success-text"
								aria-hidden="true"
								title="Plateau tranché : son aperçu est mémorisé">✓</span
							>{/if}
					</button>
				{/if}
				{#if onlock}
					<button
						type="button"
						class="ws-icon-btn ws-icon-btn-xs {plate.locked ? 'text-accent' : 'hover:text-content'}"
						aria-pressed={plate.locked}
						data-testid="plate-lock"
						title={plate.locked
							? 'Plateau verrouillé : l’arrangement ne le déplace pas. Cliquez pour déverrouiller.'
							: 'Verrouiller ce plateau (l’arrangement ne le déplacera pas)'}
						onclick={() => onlock(plate.id, !plate.locked)}
					>
						{#if plate.locked}
							<IconLock class="h-3.5 w-3.5" aria-hidden="true" />
						{:else}
							<IconLockOpen class="h-3.5 w-3.5" aria-hidden="true" />
						{/if}
					</button>
				{/if}
				{#if onfront && plates.length > 1 && plates[0]?.id !== plate.id}
					<button
						type="button"
						class="ws-icon-btn ws-icon-btn-xs hover:text-content"
						data-testid="plate-move-front"
						aria-label={`Déplacer ${plate.name} en premier`}
						title="Déplacer le plateau vers l’avant (premier plateau)"
						onclick={() => onfront(plate.id)}
						><IconFront class="h-3.5 w-3.5" aria-hidden="true" /></button
					>
				{/if}
				{#if plates.length > 1}
					<button
						type="button"
						class="ws-icon-btn ws-icon-btn-xs hover:text-danger"
						aria-label={`Supprimer ${plate.name}`}
						onclick={() => onremove(plate.id)}
						><IconX class="h-3.5 w-3.5" aria-hidden="true" /></button
					>
				{/if}
			</div>
		{/each}
		<button
			type="button"
			class="ws-btn ws-btn-sm ws-btn-secondary"
			aria-label="Ajouter un plateau"
			onclick={() => onadd()}>+</button
		>
	</div>

	{#if active}
		<label class="flex items-center gap-2">
			<span class="text-content-muted">Type de plaque</span>
			<select
				class="ws-field ws-field-sm"
				value={active.plateType}
				onchange={(e) => ontype(active.id, e.currentTarget.value)}
			>
				{#each PLATE_TYPES as pt (pt.label)}
					<option value={pt.label}>{$t(pt.label)}</option>
				{/each}
			</select>
		</label>
		{#if onsetting}
			<PlateSettings
				plate={active}
				{inherited}
				onsetting={(key, value) => onsetting(active.id, key, value)}
			/>
		{/if}
	{/if}
</div>
