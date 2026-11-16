<script lang="ts">
	// Panneau (surface surélevée bordée, rayon 8 px) avec titre optionnel.
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cx } from './styles';

	type Props = HTMLAttributes<HTMLElement> & {
		title?: string;
		actions?: Snippet;
		padded?: boolean;
		class?: string;
		children: Snippet;
	};

	let { title, actions, padded = true, class: extra = '', children, ...rest }: Props = $props();
</script>

<section class={cx('ws-panel', padded && 'p-3', extra)} {...rest}>
	{#if title || actions}
		<div class="mb-2 flex items-center justify-between gap-2">
			{#if title}<h3 class="ws-section-title">{title}</h3>{/if}
			{#if actions}{@render actions()}{/if}
		</div>
	{/if}
	{@render children()}
</section>
