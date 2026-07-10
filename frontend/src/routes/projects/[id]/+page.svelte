<script lang="ts">
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { draftStore, type DraftRecord } from '$lib/stores/draft';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	// Brouillon local plus récent que la version serveur → proposer la
	// restauration après une fermeture accidentelle (l'éditeur le consommera).
	let pendingDraft = $state<DraftRecord | null>(null);

	onMount(async () => {
		pendingDraft = await draftStore.pendingRestore(data.project.id, data.project.updated_at);
	});

	async function dismissDraft() {
		await draftStore.clear(data.project.id);
		pendingDraft = null;
	}
</script>

<header
	class="flex items-center justify-between border-b border-gray-200 px-6 py-4 dark:border-gray-700"
>
	<div>
		<a href={resolve('/library')} class="text-sm text-blue-600 hover:underline">← Bibliothèque</a>
		<h1 class="text-lg font-semibold text-gray-900 dark:text-gray-100">{data.project.name}</h1>
	</div>
</header>

{#if pendingDraft}
	<div
		class="flex items-center justify-between gap-4 bg-amber-50 px-6 py-3 text-sm text-amber-900 dark:bg-amber-950 dark:text-amber-200"
		role="status"
	>
		<span>Un brouillon local plus récent existe pour ce projet.</span>
		<button onclick={dismissDraft} class="font-medium hover:underline">Ignorer</button>
	</div>
{/if}

<main class="mx-auto max-w-4xl p-6 text-gray-600 dark:text-gray-400">
	<p>Éditeur à venir (phases P4–P6). Le projet est chargé et prêt.</p>
</main>
