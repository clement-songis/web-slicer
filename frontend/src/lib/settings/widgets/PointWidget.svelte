<script lang="ts">
	import { t } from '#lib/i18n';
	// Point Orca `coPoint*` : un seul champ texte au format canonique `XxY`.
	// Le contrat Playwright/Orca remplit ce contrôle avec « 172x172 » ; deux
	// inputs number (X et Y séparés) ne peuvent pas accepter cette forme.
	import { FIELD_CLASS, type WidgetProps } from './types';
	import { formatPoint, parsePoint } from './point';

	let { def, value = $bindable(), disabled = false, index }: WidgetProps<unknown> = $props();
	let text = $state('');
	let focused = $state(false);

	$effect(() => {
		if (!focused) text = formatPoint(value);
	});

	function onInput(event: Event) {
		text = (event.currentTarget as HTMLInputElement).value;
		const parsed = parsePoint(text);
		if (parsed !== null) value = parsed;
	}
</script>

<input
	type="text"
	inputmode="decimal"
	value={text}
	oninput={onInput}
	onfocus={() => (focused = true)}
	onblur={() => {
		focused = false;
		text = formatPoint(value);
	}}
	data-index={index}
	{disabled}
	aria-label={$t(def.label)}
	placeholder="XxY"
	class={FIELD_CLASS}
/>
