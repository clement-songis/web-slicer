<script lang="ts">
	// Bouton (T094) — présentationnel, bâti sur les jetons du design system.
	// Les attributs natifs (`onclick`, `type`, `disabled`, `title`, aria…) sont
	// transmis via le rest ; `variant`/`size` pilotent l'apparence.
	import type { Snippet } from 'svelte';
	import type { HTMLButtonAttributes } from 'svelte/elements';
	import { buttonClasses, cx, type ButtonVariant, type ButtonSize } from './styles';

	type Props = HTMLButtonAttributes & {
		variant?: ButtonVariant;
		size?: ButtonSize;
		class?: string;
		children: Snippet;
	};

	let {
		variant = 'primary',
		size = 'md',
		class: extra = '',
		type = 'button',
		children,
		...rest
	}: Props = $props();
</script>

<button {type} class={cx(buttonClasses(variant, size), extra)} {...rest}>
	{@render children()}
</button>
