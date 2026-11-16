<script lang="ts">
	// Section de panneau latéral : titre en petites capitales + actions, sans
	// cadre (les sections sont séparées par un filet).
	import type { Snippet } from 'svelte';
	import { cx } from './styles';

	type Props = {
		title: string;
		actions?: Snippet;
		divided?: boolean;
		class?: string;
		children: Snippet;
	};

	let { title, actions, divided = false, class: extra = '', children }: Props = $props();
</script>

<section class={cx('flex flex-col gap-2', divided && 'border-t border-border pt-3', extra)}>
	<div class="flex min-h-7 items-center justify-between gap-2">
		<h3 class="ws-section-title">{title}</h3>
		{#if actions}{@render actions()}{/if}
	</div>
	{@render children()}
</section>
