<script lang="ts">
	import { onMount } from 'svelte';
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';
	import { initLocale } from '$lib/i18n';

	let { children } = $props();

	// Locale au démarrage : préférence mémorisée, sinon langue du navigateur (T080).
	onMount(() => initLocale());
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>
{@render children()}
