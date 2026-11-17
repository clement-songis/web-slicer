<script lang="ts">
	// Une ligne d'option : libellé + infobulle à gauche, widget à droite. Le
	// contrat UI expose la clé Orca sur le conteneur et le premier élément des
	// vecteurs via data-index=0.
	import type { ParamDef } from '../../generated/params';
	import { widgetFor } from './widgets';
	import { t } from '$lib/i18n';
	import { clonePresetValues, settingChanged } from './editing';
	import IconUndo from '~icons/lucide/undo-2';
	import IconLockOpen from '~icons/lucide/lock-open';

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
	}

	let {
		def,
		value = $bindable(),
		disabled = false,
		compact = false,
		saved = null,
		system = null
	}: Props = $props();

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
	let overrideEnabled = $state(true);
	const controlDisabled = $derived(disabled || (def.nullable && !overrideEnabled));

	$effect(() => {
		if (def.nullable) overrideEnabled = hasOverride(value);
		if ((scalarVector || pointVector) && !Array.isArray(value)) {
			value = value === null || value === undefined ? [null] : [value];
		}
	});

	function hasOverride(current: unknown): boolean {
		if (Array.isArray(current))
			return current.some((entry) => entry !== null && entry !== undefined);
		return current !== null && current !== undefined;
	}

	function toggleOverride() {
		overrideEnabled = !overrideEnabled;
		if (overrideEnabled && !hasOverride(value)) {
			const fallback = def.default ?? (def.type.startsWith('coBool') ? false : 0);
			value = scalarVector || pointVector ? [fallback] : fallback;
		}
	}
</script>

<div
	class="flex min-h-9 items-center py-1 {compact ? 'gap-2' : 'gap-3'}"
	data-param={def.key}
	data-modified={modified ? 'true' : undefined}
	data-from-system={fromSystem ? 'true' : undefined}
>
	<span
		class="min-w-0 flex-1 text-sm text-content-muted"
		title={def.tooltip ? $t(def.tooltip) : undefined}
	>
		{$t(def.label)}
		{#if def.tooltip}
			<span class="ml-1 cursor-help text-content-subtle" aria-hidden="true">ⓘ</span>
		{/if}
	</span>
	{#if def.nullable}
		<input
			type="checkbox"
			data-override-toggle
			checked={overrideEnabled}
			onchange={toggleOverride}
			aria-label={`Activer ${$t(def.label)}`}
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
					data-tip="Différent du preset système — cliquer pour reprendre la valeur système"
					aria-label={`Valeur système pour ${$t(def.label)}`}
					data-testid="option-reset-system"
					{disabled}
					onclick={() => system && resetTo(system)}
					><IconLockOpen class="h-3.5 w-3.5" aria-hidden="true" /></button
				>
			{/if}
			{#if modified && saved}
				<button
					type="button"
					class="ws-icon-btn ws-icon-btn-xs text-warning-content"
					data-tip="Modifié — cliquer pour revenir à la valeur enregistrée"
					aria-label={`Revenir à la valeur enregistrée pour ${$t(def.label)}`}
					data-testid="option-reset-saved"
					{disabled}
					onclick={() => saved && resetTo(saved)}
					><IconUndo class="h-3.5 w-3.5" aria-hidden="true" /></button
				>
			{/if}
		</div>
	{/if}
	<div class="{compact ? 'w-32' : 'w-56'} shrink-0">
		{#if (scalarVector || pointVector) && Array.isArray(value)}
			<Widget {def} bind:value={value[0]} index={indexed} disabled={controlDisabled} />
		{:else if !scalarVector && !pointVector}
			<Widget {def} bind:value index={indexed} disabled={controlDisabled} />
		{/if}
	</div>
</div>
