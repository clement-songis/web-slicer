<script lang="ts">
	// Champ texte (design system v2) — recette .ws-field (layout.css).
	// `value` est bindable ; tous les attributs natifs passent par le rest.
	import type { HTMLInputAttributes } from 'svelte/elements';
	import { cx, fieldClasses, type ButtonSizeFull } from './styles';

	type Props = Omit<HTMLInputAttributes, 'size'> & {
		value?: string | number | null;
		size?: ButtonSizeFull;
		class?: string;
	};

	let {
		value = $bindable(),
		size = 'md',
		class: extra = '',
		type = 'text',
		...rest
	}: Props = $props();
</script>

<input {type} class={cx(fieldClasses(size), extra)} bind:value {...rest} />
