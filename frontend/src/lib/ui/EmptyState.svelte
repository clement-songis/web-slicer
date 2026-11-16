<script lang="ts">
	// État vide : icône (snippet), titre, description, action éventuelle.
	import type { Snippet } from 'svelte';
	import { cx } from './styles';

	type Props = {
		title: string;
		description?: string;
		icon?: Snippet;
		action?: Snippet;
		class?: string;
	};

	let { title, description, icon, action, class: extra = '' }: Props = $props();
</script>

<div class={cx('ws-empty', extra)}>
	{#if icon}
		<div
			class="mb-1 flex h-10 w-10 items-center justify-center rounded-full bg-surface-sunken text-content-subtle"
		>
			{@render icon()}
		</div>
	{/if}
	<p class="font-medium text-content">{title}</p>
	{#if description}<p class="max-w-sm text-content-muted">{description}</p>{/if}
	{#if action}<div class="mt-2">{@render action()}</div>{/if}
</div>
