<script lang="ts">
	// Une ligne d'option : libellé + infobulle à gauche, widget à droite. Le
	// contrat UI expose la clé Orca sur le conteneur et le premier élément des
	// vecteurs via data-index=0.
	import type { ParamDef } from '../../generated/params';
	import { widgetFor } from './widgets';
	import { t } from '$lib/i18n';
	import { clonePresetValues, settingChanged } from './editing';
	import { untrack } from 'svelte';
	import IconUndo from '~icons/lucide/undo-2';
	import IconLockOpen from '~icons/lucide/lock-open';
	import { wikiUrl } from './wiki';
	import { variantSlot } from './filament-variants';

	interface Props {
		def: ParamDef;
		// Les widgets spécialisés resserrent le type après résolution par `def`.
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		value: any;
		disabled?: boolean;
		/** Colonne étroite (panneau latéral : réglages par objet / par plage) :
		 *  widget resserré pour que le libellé tienne sur une ou deux lignes. */
		compact?: boolean;
		/** Valeurs du preset enregistré : option modifiée → marqueur et retour
		 *  à la valeur enregistrée (flèche orange des onglets d'Orca). */
		saved?: Record<string, unknown> | null;
		/** Valeurs du preset système parent : option différente → cadenas ouvert,
		 *  retour à la valeur système (cadenas d'Orca). */
		system?: Record<string, unknown> | null;
		/** Variante de buse éditée (filament multi-buses, Orca m_variant_combo) :
		 *  index du vecteur pour les options par variante ; 0 sinon. */
		variantIndex?: number;
		/** Libellé remplaçant celui de `def` dans une ligne composée d'Orca
		 *  (ex. « Target » pour chamber_temperature, Tab.cpp). */
		label?: string;
	}

	let {
		def,
		value = $bindable(),
		disabled = false,
		compact = false,
		saved = null,
		system = null,
		variantIndex,
		label
	}: Props = $props();
	const shownLabel = $derived(label ?? def.label);

	// Phase 34 : clic sur le libellé → wiki d'Orca (OptionsGroup::get_url).
	const wiki = $derived(wikiUrl(def.key));
	// Valeur éditée : celle de la variante choisie pour une option par variante.
	const vi = $derived(variantSlot(def.key, value, variantIndex ?? 0));
	// Badge seulement quand le sélecteur de variante est actif (onglet Filament).
	const perVariant = $derived(variantIndex !== undefined && variantSlot(def.key, value, 1) === 1);

	const markers = $derived(saved !== null || system !== null);
	const modified = $derived(saved !== null && settingChanged(def.key, saved, value));
	const fromSystem = $derived(system !== null && settingChanged(def.key, system, value));
	function resetTo(reference: Record<string, unknown>) {
		const v = reference[def.key];
		value = v === undefined ? def.default : clonePresetValues({ v }).v;
	}

	const Widget = $derived(widgetFor(def));
	const scalarVector = $derived(
		[
			'coBools',
			'coBoolsNullable',
			'coEnums',
			'coEnumsNullable',
			'coInts',
			'coFloats',
			'coFloatsNullable',
			'coPercents',
			'coPercentsNullable',
			'coFloatsOrPercents'
		].includes(def.type)
	);
	const pointVector = $derived(def.type === 'coPoints' || def.type === 'coPointsGroups');
	const indexed = $derived(
		scalarVector || pointVector || def.type === 'coPoint' || def.type === 'coStrings'
			? 0
			: undefined
	);
	// État initial pris de la valeur : partir de « coché » montait d'abord le
	// widget lié, dont la case écrivait `false` dans un vecteur « nil ».
	let overrideEnabled = $state(untrack(() => !def.nullable || hasOverride(value)));
	const controlDisabled = $derived(disabled || (def.nullable && !overrideEnabled));

	$effect(() => {
		if (def.nullable) overrideEnabled = hasOverride(value);
		// Surcharge filament non cochée (absente ou « nil ») : ne RIEN écrire —
		// un `[null]` posé ici partait dans le brouillon et le tranchage échouait
		// (« valeur illisible pour filament_retract_lift_above », phase 27).
		if (def.nullable && !hasOverride(value)) return;
		if ((scalarVector || pointVector) && !Array.isArray(value)) {
			value = value === null || value === undefined ? [null] : [value];
		}
	});

	function hasOverride(current: unknown): boolean {
		// Variante choisie (filament multi-buses) : seule sa valeur compte.
		const slot = variantSlot(def.key, current, variantIndex ?? 0);
		if (Array.isArray(current) && slot > 0) {
			const entry = current[slot];
			return entry !== null && entry !== undefined;
		}
		if (Array.isArray(current))
			return current.some((entry) => entry !== null && entry !== undefined);
		return current !== null && current !== undefined;
	}

	function toggleOverride() {
		overrideEnabled = !overrideEnabled;
		if (overrideEnabled && !hasOverride(value)) {
			const fallback = def.default ?? (def.type.startsWith('coBool') ? false : 0);
			if (vi > 0 && Array.isArray(value)) {
				const next = [...value];
				next[vi] = fallback;
				value = next;
			} else value = scalarVector || pointVector ? [fallback] : fallback;
		}
	}
</script>

<div
	class="flex min-h-9 items-center py-1 {compact ? 'gap-2' : 'gap-3'}"
	data-param={def.key}
	data-variant={perVariant ? vi : undefined}
	data-modified={modified ? 'true' : undefined}
	data-from-system={fromSystem ? 'true' : undefined}
>
	<span
		class="min-w-0 flex-1 text-sm text-content-muted"
		title={def.tooltip ? $t(def.tooltip) : undefined}
	>
		{#if wiki}
			<!-- Orca : libellé cliquable → page du wiki de l'option (nouvel onglet). -->
			<!-- Lien externe (wiki d'Orca) : pas une route de l'application. -->
			<!-- eslint-disable svelte/no-navigation-without-resolve -->
			<a
				href={wiki}
				target="_blank"
				rel="noopener noreferrer"
				class="underline-offset-2 hover:text-primary hover:underline focus-visible:text-primary focus-visible:underline"
				data-testid="option-wiki-link"
				aria-label={$t('{label} — OrcaSlicer wiki help (new tab)', { label: $t(shownLabel) })}
			>
				{$t(shownLabel)}
			</a>
			<!-- eslint-enable svelte/no-navigation-without-resolve -->
		{:else}
			{$t(shownLabel)}
		{/if}
		{#if def.tooltip}
			<span class="ml-1 cursor-help text-content-subtle" aria-hidden="true">ⓘ</span>
		{/if}
		{#if perVariant}
			<span
				class="ws-badge ml-1 normal-case"
				data-testid="option-variant-badge"
				title={$t(
					'Value specific to each nozzle variant: choose the variant at the top of the Filament tab.'
				)}
			>
				{$t('per nozzle')}
			</span>
		{/if}
	</span>
	{#if def.nullable}
		<input
			type="checkbox"
			data-override-toggle
			checked={overrideEnabled}
			onchange={toggleOverride}
			aria-label={$t('Enable {label}', { label: $t(def.label) })}
			class="h-4 w-4 shrink-0 rounded border-border-strong"
		/>
	{/if}
	{#if markers}
		<!-- Colonne fixe : l'alignement des champs ne bouge pas quand un marqueur
		     apparaît. -->
		<div class="flex w-12 shrink-0 items-center justify-end gap-0.5">
			{#if fromSystem && system}
				<button
					type="button"
					class="ws-icon-btn ws-icon-btn-xs text-content-subtle"
					data-tip={$t('Different from the system preset — click to restore the system value')}
					aria-label={$t('System value for {label}', { label: $t(def.label) })}
					data-testid="option-reset-system"
					{disabled}
					onclick={() => system && resetTo(system)}
				>
					<IconLockOpen class="h-3.5 w-3.5" aria-hidden="true" />
				</button>
			{/if}
			{#if modified && saved}
				<button
					type="button"
					class="ws-icon-btn ws-icon-btn-xs text-warning-content"
					data-tip={$t('Modified — click to revert to the saved value')}
					aria-label={$t('Revert to the saved value for {label}', { label: $t(def.label) })}
					data-testid="option-reset-saved"
					{disabled}
					onclick={() => saved && resetTo(saved)}
				>
					<IconUndo class="h-3.5 w-3.5" aria-hidden="true" />
				</button>
			{/if}
		</div>
	{/if}
	<div class="{compact ? 'w-32' : 'w-56'} shrink-0">
		{#if def.nullable && !overrideEnabled}
			<!-- Non lié : un widget inactif (case décochée) ne doit pas écrire
			     sa valeur neutre (false, 0…) dans un vecteur « nil ». -->
			<Widget
				{def}
				value={Array.isArray(value) ? value[vi] : value}
				index={indexed}
				disabled={true}
			/>
		{:else if (scalarVector || pointVector) && Array.isArray(value)}
			<Widget {def} bind:value={value[vi]} index={indexed} disabled={controlDisabled} />
		{:else if !scalarVector && !pointVector}
			<Widget {def} bind:value index={indexed} disabled={controlDisabled} />
		{/if}
	</div>
</div>
