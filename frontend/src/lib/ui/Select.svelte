<script lang="ts">
	// Liste déroulante native stylée (.ws-field) : clavier et lecteurs d'écran
	// natifs conservés. Les <option> sont fournies par l'appelant.
	import type { Snippet } from 'svelte';
	import type { HTMLSelectAttributes } from 'svelte/elements';
	import { cx, fieldClasses, type ButtonSizeFull } from './styles';

	type Props = Omit<HTMLSelectAttributes, 'size'> & {
		value?: unknown;
		size?: ButtonSizeFull;
		class?: string;
		children: Snippet;
	};

	let { value = $bindable(), size = 'md', class: extra = '', children, ...rest }: Props = $props();
</script>

<select class={cx(fieldClasses(size), 'pr-7', extra)} bind:value {...rest}>
	{@render children()}
</select>
