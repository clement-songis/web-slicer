<script lang="ts">
	// Bandeau (T094) — message contextuel (info/succès/avertissement/erreur) bâti
	// sur les jetons `*-soft`/`*-content`. `onclose` (optionnel) affiche un bouton
	// de fermeture. `role` s'ajuste : alerte pour danger, statut sinon.
	import type { Snippet } from 'svelte';
	import { bannerClasses, cx, FOCUS_RING, type BannerTone } from './styles';

	type Props = {
		tone?: BannerTone;
		onclose?: () => void;
		closeLabel?: string;
		children: Snippet;
	};

	let { tone = 'info', onclose, closeLabel = 'Fermer', children }: Props = $props();
</script>

<div class={bannerClasses(tone)} role={tone === 'danger' ? 'alert' : 'status'}>
	<span>{@render children()}</span>
	{#if onclose}
		<button
			type="button"
			onclick={onclose}
			class={cx('shrink-0 rounded font-medium underline-offset-2 hover:underline', FOCUS_RING)}
		>
			{closeLabel}
		</button>
	{/if}
</div>
