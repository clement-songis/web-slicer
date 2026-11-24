<script lang="ts">
	import { t } from '#lib/i18n';
	import { FIELD_CLASS, type WidgetProps } from './types';
	import GcodeEditorDialog from '../GcodeEditorDialog.svelte';

	let {
		def,
		value = $bindable(),
		disabled = false,
		index
	}: WidgetProps<string | string[]> = $props();

	const text = $derived(Array.isArray(value) ? (value[0] ?? '') : (value ?? ''));
	// G-code personnalisé : éditeur dédié avec les variables (Orca « Edit
	// custom G-code »).
	const isGcode = $derived(/gcode/.test(def.key));
	let editing = $state(false);
	let wrapper = $state<HTMLDivElement | null>(null);

	function set(raw: string) {
		value = def.type === 'coStrings' ? [raw] : raw;
	}
	function onInput(event: Event) {
		set((event.currentTarget as HTMLTextAreaElement).value);
	}
</script>

<div class="flex flex-col gap-1" bind:this={wrapper}>
	<textarea
		value={text}
		oninput={onInput}
		data-index={index}
		{disabled}
		rows="6"
		spellcheck="false"
		aria-label={$t(def.label)}
		class="{FIELD_CLASS} font-mono"></textarea>
	{#if isGcode}
		<button
			type="button"
			class="ws-btn ws-btn-sm ws-btn-ghost self-end"
			data-testid="gcode-edit"
			{disabled}
			onclick={() => (editing = true)}
		>
			{$t('Edit with variables…')}
		</button>
		<GcodeEditorDialog
			open={editing}
			gcodeKey={def.key}
			title={$t(def.label)}
			value={text}
			onapply={(next) => {
				set(next);
				// Même chemin qu'une saisie : le conteneur des réglages note la clé
				// (événement « change » remonté depuis la ligne d'option).
				wrapper?.dispatchEvent(new Event('change', { bubbles: true }));
			}}
			onclose={() => (editing = false)}
		/>
	{/if}
</div>
