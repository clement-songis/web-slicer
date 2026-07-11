<script lang="ts">
	// Bouton-icône carré (T094) — pour les actions compactes (barres d'outils,
	// fermeture de bandeau). `label` est obligatoire (accessibilité).
	import type { Snippet } from 'svelte';
	import type { HTMLButtonAttributes } from 'svelte/elements';
	import { iconButtonClasses, cx, type ButtonVariant, type ButtonSize } from './styles';

	type Props = HTMLButtonAttributes & {
		label: string;
		variant?: ButtonVariant;
		size?: ButtonSize;
		class?: string;
		children: Snippet;
	};

	let {
		label,
		variant = 'ghost',
		size = 'md',
		class: extra = '',
		type = 'button',
		children,
		...rest
	}: Props = $props();
</script>

<button
	{type}
	class={cx(iconButtonClasses(variant, size), extra)}
	aria-label={label}
	title={label}
	{...rest}
>
	{@render children()}
</button>
