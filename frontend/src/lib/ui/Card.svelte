<script lang="ts">
	// Carte (T094) — surface surélevée bordée. `padded` (défaut) applique un
	// rembourrage standard ; passer `false` pour un contenu qui gère le sien.
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cx } from './styles';

	type Props = HTMLAttributes<HTMLDivElement> & {
		padded?: boolean;
		class?: string;
		children: Snippet;
	};

	let { padded = true, class: extra = '', children, ...rest }: Props = $props();
</script>

<div
	class={cx('rounded-lg border border-border bg-surface-raised', padded && 'p-4', extra)}
	{...rest}
>
	{@render children()}
</div>
