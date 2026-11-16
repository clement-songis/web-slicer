<script lang="ts">
	// Une ligne d'option : libellé + infobulle à gauche, widget à droite. Le
	// contrat UI expose la clé Orca sur le conteneur et le premier élément des
	// vecteurs via data-index=0.
	import type { ParamDef } from '../../generated/params';
	import { widgetFor } from './widgets';
	import { t } from '$lib/i18n';

	interface Props {
		def: ParamDef;
		// Les widgets spécialisés resserrent le type après résolution par `def`.
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		value: any;
		disabled?: boolean;
		/** Colonne étroite (panneau latéral : réglages par objet / par plage) :
		 *  widget resserré pour que le libellé tienne sur une ou deux lignes. */
		compact?: boolean;
	}

	let { def, value = $bindable(), disabled = false, compact = false }: Props = $props();

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

<div class="flex min-h-9 items-center py-1 {compact ? 'gap-2' : 'gap-3'}" data-param={def.key}>
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
	<div class="{compact ? 'w-32' : 'w-56'} shrink-0">
		{#if (scalarVector || pointVector) && Array.isArray(value)}
			<Widget {def} bind:value={value[0]} index={indexed} disabled={controlDisabled} />
		{:else if !scalarVector && !pointVector}
			<Widget {def} bind:value index={indexed} disabled={controlDisabled} />
		{/if}
	</div>
</div>
