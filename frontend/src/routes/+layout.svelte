<script lang="ts">
	import { onMount } from 'svelte';
	import './layout.css';
	import favicon from '#lib/assets/favicon.svg';
	import { initLocale } from '#lib/i18n';
	import { initTheme } from '#lib/theme';
	import TooltipLayer from '#lib/ui/TooltipLayer.svelte';
	import ConnectionBanner from '#lib/ui/ConnectionBanner.svelte';
	import ReauthDialog from '#lib/ui/ReauthDialog.svelte';

	let { children } = $props();

	// Démarrage : locale mémorisée/navigateur (T080) + thème mémorisé/OS (T093).
	onMount(() => {
		initLocale();
		initTheme();
	});
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>
{@render children()}
<TooltipLayer />
<ConnectionBanner />
<ReauthDialog />
