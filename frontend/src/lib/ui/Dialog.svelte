<script lang="ts">
	// Dialogue modal (design system v2) : voile .ws-scrim, boîte .ws-dialog,
	// en-tête (titre) / corps défilant / pied d'actions. Focus géré par
	// `dialogFocus` (focus à l'ouverture, restitué à la fermeture), Échap et
	// clic sur le voile ferment. `label` = nom accessible (role="dialog").
	// Les data-* (data-testid…) passent sur l'élément role="dialog".
	import type { Snippet } from 'svelte';
	import { dialogFocus } from './dialog-focus';
	import { cx } from './styles';

	type Props = {
		open: boolean;
		label: string;
		title?: string;
		/** Largeur : sm 24 rem, md 32 rem, lg 48 rem, xl 64 rem. */
		size?: 'sm' | 'md' | 'lg' | 'xl';
		onclose: () => void;
		class?: string;
		header?: Snippet;
		footer?: Snippet;
		children: Snippet;
		[key: `data-${string}`]: string | undefined;
	};

	let {
		open,
		label,
		title,
		size = 'md',
		onclose,
		class: extra = '',
		header,
		footer,
		children,
		...rest
	}: Props = $props();

	const WIDTH = { sm: 'w-96', md: 'w-[32rem]', lg: 'w-[48rem]', xl: 'w-[64rem]' } as const;
</script>

{#if open}
	<div class="ws-scrim" role="presentation" onclick={onclose}>
		<div
			class={cx('ws-dialog flex max-h-full max-w-full flex-col', WIDTH[size], extra)}
			role="dialog"
			aria-modal="true"
			aria-label={label}
			tabindex="-1"
			use:dialogFocus={onclose}
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => {
				if (e.key === 'Escape') {
					e.stopPropagation();
					onclose();
				}
			}}
			{...rest}
		>
			{#if header || title}
				<header class="ws-dialog-header">
					{#if header}{@render header()}{:else}<h2 class="ws-dialog-title">{title}</h2>{/if}
				</header>
			{/if}
			<div class="ws-dialog-body">{@render children()}</div>
			{#if footer}
				<footer class="ws-dialog-footer">{@render footer()}</footer>
			{/if}
		</div>
	</div>
{/if}
